use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::core::AiCall;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub name: String,
    pub regex: String,
    pub wrapper_type: String,
    pub extract_model: bool,
}

static AI_PATTERNS: Lazy<Vec<Pattern>> = Lazy::new(|| {
    vec![
        // OpenAI patterns
        Pattern {
            name: "openai_api_key".to_string(),
            regex: r#"(?i)(openai[_-]?api[_-]?key|OPENAI_API_KEY)\s*[:=]\s*["']([^"']+)["']"#.to_string(),
            wrapper_type: "openai_config".to_string(),
            extract_model: false,
        },
        Pattern {
            name: "openai_endpoint".to_string(),
            regex: r#"https?://api\.openai\.com/v\d+/[\w/]+"#.to_string(),
            wrapper_type: "openai_api".to_string(),
            extract_model: false,
        },
        
        // Anthropic/Claude patterns
        Pattern {
            name: "anthropic_api_key".to_string(),
            regex: r#"(?i)(anthropic[_-]?api[_-]?key|ANTHROPIC_API_KEY)\s*[:=]\s*["']([^"']+)["']"#.to_string(),
            wrapper_type: "anthropic_config".to_string(),
            extract_model: false,
        },
        Pattern {
            name: "claude_endpoint".to_string(),
            regex: r#"https?://api\.anthropic\.com/v\d+/[\w/]+"#.to_string(),
            wrapper_type: "anthropic_api".to_string(),
            extract_model: false,
        },
        
        // LangChain patterns
        Pattern {
            name: "langchain_llm".to_string(),
            regex: r#"(?:from\s+langchain[.\w]*\s+import|langchain[.\w]*\.)(?:ChatOpenAI|Claude|ChatAnthropic|LLM)"#.to_string(),
            wrapper_type: "langchain".to_string(),
            extract_model: true,
        },
        
        // Autogen patterns
        Pattern {
            name: "autogen_agent".to_string(),
            regex: r#"(?:from\s+autogen\s+import|autogen\.)(?:AssistantAgent|UserProxyAgent|GroupChat)"#.to_string(),
            wrapper_type: "autogen".to_string(),
            extract_model: true,
        },
        
        // CrewAI patterns
        Pattern {
            name: "crewai_agent".to_string(),
            regex: r#"(?:from\s+crewai\s+import|crewai\.)(?:Agent|Task|Crew)"#.to_string(),
            wrapper_type: "crewai".to_string(),
            extract_model: true,
        },
        
        // Hugging Face patterns
        Pattern {
            name: "huggingface_pipeline".to_string(),
            regex: r#"(?:from\s+transformers\s+import|transformers\.)(?:pipeline|AutoModel|AutoTokenizer)"#.to_string(),
            wrapper_type: "huggingface".to_string(),
            extract_model: true,
        },
        
        // Generic model loading
        Pattern {
            name: "model_load".to_string(),
            regex: r#"(?i)(?:load_model|from_pretrained|load_checkpoint)\s*\(\s*["']([^"']+)["']"#.to_string(),
            wrapper_type: "model_loader".to_string(),
            extract_model: true,
        },
        
        // API keys in environment
        Pattern {
            name: "env_api_key".to_string(),
            regex: r#"(?i)(?:getenv|environ\.get|process\.env)\s*\(\s*["'](?:OPENAI_API_KEY|ANTHROPIC_API_KEY|HUGGINGFACE_TOKEN)["']"#.to_string(),
            wrapper_type: "env_config".to_string(),
            extract_model: false,
        },
    ]
});

static COMPILED_PATTERNS: Lazy<HashMap<String, Regex>> = Lazy::new(|| {
    let mut compiled = HashMap::new();
    for pattern in AI_PATTERNS.iter() {
        if let Ok(regex) = Regex::new(&pattern.regex) {
            compiled.insert(pattern.name.clone(), regex);
        }
    }
    compiled
});

pub struct PatternMatcher;

impl PatternMatcher {
    pub fn new() -> Self {
        Self
    }
    
    pub fn find_matches(&self, path: &Path, content: &str) -> Vec<AiCall> {
        let mut matches = Vec::new();
        
        for pattern in AI_PATTERNS.iter() {
            if let Some(regex) = COMPILED_PATTERNS.get(&pattern.name) {
                for capture in regex.captures_iter(content) {
                    if let Some(ai_call) = self.create_ai_call(path, content, &capture, pattern) {
                        matches.push(ai_call);
                    }
                }
            }
        }
        
        matches
    }
    
    fn create_ai_call(
        &self,
        path: &Path,
        content: &str,
        capture: &regex::Captures,
        pattern: &Pattern,
    ) -> Option<AiCall> {
        let match_ = capture.get(0)?;
        let match_start = match_.start();
        
        // Calculate line and column
        let (line, column) = self.byte_offset_to_line_col(content, match_start);
        
        // Extract model if pattern supports it
        let model = if pattern.extract_model {
            capture.get(1).map(|m| m.as_str().to_string())
        } else {
            None
        };
        
        // Get context
        let context = self.extract_context(content, line);
        
        Some(AiCall {
            file: path.to_path_buf(),
            line: line + 1,
            column: column + 1,
            wrapper: pattern.wrapper_type.clone(),
            model,
            params: serde_json::json!({
                "pattern": pattern.name,
                "match": match_.as_str(),
            }),
            context,
        })
    }
    
    fn byte_offset_to_line_col(&self, content: &str, byte_offset: usize) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;
        let mut current_offset = 0;
        
        for ch in content.chars() {
            if current_offset >= byte_offset {
                break;
            }
            
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
            
            current_offset += ch.len_utf8();
        }
        
        (line, col)
    }
    
    fn extract_context(&self, content: &str, line: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let start = line.saturating_sub(2);
        let end = (line + 3).min(lines.len());
        
        lines[start..end].join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_openai_key_pattern() {
        let matcher = PatternMatcher::new();
        let content = r#"
import os
OPENAI_API_KEY = "sk-1234567890"
client = OpenAI(api_key=OPENAI_API_KEY)
"#;
        
        let matches = matcher.find_matches(&PathBuf::from("test.py"), content);
        assert!(!matches.is_empty());
        assert_eq!(matches[0].wrapper, "openai_config");
    }
    
    #[test]
    fn test_langchain_pattern() {
        let matcher = PatternMatcher::new();
        let content = r#"
from langchain.llms import ChatOpenAI
llm = ChatOpenAI(model="gpt-4", temperature=0.7)
"#;
        
        let matches = matcher.find_matches(&PathBuf::from("test.py"), content);
        assert!(!matches.is_empty());
        assert_eq!(matches[0].wrapper, "langchain");
    }
}