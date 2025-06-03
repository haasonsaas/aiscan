use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tiktoken_rs::{get_bpe_from_model, CoreBPE};

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub tokens: TokenUsage,
    pub estimated_cost_usd: f64,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct Budget {
    pub max_tokens: Option<usize>,
    pub max_requests: Option<usize>,
    pub max_usd: Option<f64>,
    pub used_tokens: usize,
    pub used_requests: usize,
    pub used_usd: f64,
}

impl Budget {
    pub fn from_config(config: &Config) -> Self {
        Self {
            max_tokens: config.limits.max_tokens,
            max_requests: config.limits.max_requests,
            max_usd: config.limits.max_usd,
            used_tokens: 0,
            used_requests: 0,
            used_usd: 0.0,
        }
    }
    
    pub fn consume(&mut self, tokens: usize) -> Result<()> {
        if let Some(max) = self.max_tokens {
            if self.used_tokens + tokens > max {
                anyhow::bail!("Token budget exceeded: {} + {} > {}", self.used_tokens, tokens, max);
            }
        }
        
        self.used_tokens += tokens;
        self.used_requests += 1;
        
        if let Some(max_requests) = self.max_requests {
            if self.used_requests > max_requests {
                anyhow::bail!("Request budget exceeded: {} > {}", self.used_requests, max_requests);
            }
        }
        
        Ok(())
    }
    
    pub fn consume_cost(&mut self, cost_usd: f64) -> Result<()> {
        if let Some(max_usd) = self.max_usd {
            if self.used_usd + cost_usd > max_usd {
                anyhow::bail!("Cost budget exceeded: ${:.2} + ${:.2} > ${:.2}", 
                    self.used_usd, cost_usd, max_usd);
            }
        }
        
        self.used_usd += cost_usd;
        Ok(())
    }
    
    pub fn is_exceeded(&self) -> bool {
        if let Some(max) = self.max_tokens {
            if self.used_tokens >= max {
                return true;
            }
        }
        
        if let Some(max) = self.max_requests {
            if self.used_requests >= max {
                return true;
            }
        }
        
        if let Some(max) = self.max_usd {
            if self.used_usd >= max {
                return true;
            }
        }
        
        false
    }
    
    pub fn remaining_tokens(&self) -> Option<usize> {
        self.max_tokens.map(|max| max.saturating_sub(self.used_tokens))
    }
    
    pub fn remaining_usd(&self) -> Option<f64> {
        self.max_usd.map(|max| max - self.used_usd)
    }
}

pub struct TokenCounter {
    encoders: HashMap<String, CoreBPE>,
}

impl TokenCounter {
    pub fn new() -> Result<Self> {
        let mut encoders = HashMap::new();
        
        // Initialize encoders for common models
        let models = vec!["gpt-4", "gpt-3.5-turbo", "text-embedding-ada-002"];
        
        for model in models {
            if let Ok(bpe) = get_bpe_from_model(model) {
                encoders.insert(model.to_string(), bpe);
            }
        }
        
        // Default to gpt-4 encoder for unknown models
        let default_encoder = get_bpe_from_model("gpt-4")?;
        encoders.insert("default".to_string(), default_encoder);
        
        Ok(Self { encoders })
    }
    
    pub fn estimate_tokens(&self, text: &str, model: &str) -> usize {
        let encoder = self.encoders.get(model)
            .or_else(|| self.encoders.get("default"))
            .expect("Default encoder should always exist");
        
        encoder.encode_with_special_tokens(text).len()
    }
    
    pub fn estimate_cost(&self, tokens: &TokenUsage, model: &str) -> f64 {
        // Pricing per 1K tokens (as of 2024)
        let (input_price, output_price) = match model {
            "gpt-4o" | "gpt-4" => (0.03, 0.06),
            "gpt-4o-mini" => (0.00015, 0.0006),
            "gpt-3.5-turbo" => (0.0005, 0.0015),
            "claude-3-opus" => (0.015, 0.075),
            "claude-3-sonnet" => (0.003, 0.015),
            "claude-3-haiku" => (0.00025, 0.00125),
            _ => (0.002, 0.002), // Conservative default
        };
        
        let input_cost = (tokens.prompt_tokens as f64 / 1000.0) * input_price;
        let output_cost = (tokens.completion_tokens as f64 / 1000.0) * output_price;
        
        input_cost + output_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_budget_tracking() {
        let config = Config {
            limits: crate::config::Limits {
                max_tokens: Some(1000),
                max_requests: Some(10),
                max_usd: Some(5.0),
            },
            ..Default::default()
        };
        
        let mut budget = Budget::from_config(&config);
        
        assert!(budget.consume(500).is_ok());
        assert_eq!(budget.used_tokens, 500);
        assert_eq!(budget.remaining_tokens(), Some(500));
        
        assert!(budget.consume(600).is_err());
        assert!(!budget.is_exceeded());
        
        assert!(budget.consume(500).is_ok());
        assert!(budget.is_exceeded());
    }
}