use anyhow::Result;
use colored::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::core::Inventory;
use crate::cost::{Budget, TokenCounter, TokenUsage};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub id: String,
    pub severity: Severity,
    pub file: String,
    pub line: usize,
    pub issue_type: IssueType,
    pub description: String,
    pub rationale: String,
    pub fix: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    // OWASP LLM Top 10
    LLM01PromptInjection,
    LLM02InsecureOutputHandling,
    LLM03TrainingDataPoisoning,
    LLM04ModelDoS,
    LLM05SupplyChainVulnerabilities,
    LLM06SensitiveInfoDisclosure,
    LLM07InsecurePluginDesign,
    LLM08ExcessiveAgency,
    LLM09Overreliance,
    LLM10ModelTheft,

    // Additional security concerns
    ApiKeyExposure,
    MissingInputValidation,
    UnrestrictedModelAccess,
    MissingRateLimiting,
    InsecureModelStorage,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AuditResult {
    pub findings: Vec<SecurityFinding>,
    pub summary: AuditSummary,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AuditSummary {
    pub total_findings: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

impl AuditResult {
    pub fn has_high_severity(&self) -> bool {
        self.summary.critical > 0 || self.summary.high > 0
    }

    pub fn print_findings(&self) {
        println!("\n{}", "Security Audit Results".bold().yellow());
        println!("{}", "=".repeat(50));

        if self.findings.is_empty() {
            println!("{}", "No security issues found!".green());
            return;
        }

        println!("\n{}", "Summary:".bold());
        println!("  Total findings: {}", self.summary.total_findings);
        if self.summary.critical > 0 {
            println!(
                "  {} Critical",
                self.summary.critical.to_string().red().bold()
            );
        }
        if self.summary.high > 0 {
            println!("  {} High", self.summary.high.to_string().red());
        }
        if self.summary.medium > 0 {
            println!("  {} Medium", self.summary.medium.to_string().yellow());
        }
        if self.summary.low > 0 {
            println!("  {} Low", self.summary.low.to_string().blue());
        }
        if self.summary.info > 0 {
            println!("  {} Info", self.summary.info.to_string().white());
        }

        println!("\n{}", "Findings:".bold());
        for (i, finding) in self.findings.iter().enumerate() {
            println!(
                "\n{}. {} [{}]",
                i + 1,
                finding.description.bold(),
                self.severity_color(&finding.severity)
            );
            println!("   File: {}:{}", finding.file, finding.line);
            println!("   Type: {:?}", finding.issue_type);
            println!("   Rationale: {}", finding.rationale);
            println!("   Fix: {}", finding.fix.green());
        }
    }

    fn severity_color(&self, severity: &Severity) -> ColoredString {
        match severity {
            Severity::Critical => "CRITICAL".red().bold(),
            Severity::High => "HIGH".red(),
            Severity::Medium => "MEDIUM".yellow(),
            Severity::Low => "LOW".blue(),
            Severity::Info => "INFO".white(),
        }
    }
}

pub struct SecurityAuditor {
    budget: Arc<Mutex<Budget>>,
    token_counter: TokenCounter,
}

impl SecurityAuditor {
    pub fn new(budget: Arc<Mutex<Budget>>) -> Self {
        Self {
            budget,
            token_counter: TokenCounter::new()
                .unwrap_or_else(|_| panic!("Failed to initialize token counter")),
        }
    }

    pub async fn audit(&self, inventory: &Inventory, config: &Config) -> Result<AuditResult> {
        let mut findings = Vec::new();

        // Static analysis findings
        findings.extend(self.static_analysis(inventory)?);

        // LLM-powered analysis if budget allows
        if !inventory.ai_calls.is_empty() {
            match self.llm_analysis(inventory, config).await {
                Ok(llm_findings) => findings.extend(llm_findings),
                Err(e) => {
                    tracing::warn!("LLM analysis failed: {}", e);
                }
            }
        }

        // Deduplicate and sort findings
        findings.sort_by_key(|f| (f.file.clone(), f.line, f.id.clone()));
        findings.dedup_by_key(|f| (f.file.clone(), f.line, f.id.clone()));

        // Calculate summary
        let summary = self.calculate_summary(&findings);

        Ok(AuditResult { findings, summary })
    }

    fn static_analysis(&self, inventory: &Inventory) -> Result<Vec<SecurityFinding>> {
        let mut findings = Vec::new();

        for call in &inventory.ai_calls {
            // Check for hardcoded API keys
            if call.context.contains("api_key") || call.context.contains("API_KEY") {
                if !call.context.contains("env") && !call.context.contains("getenv") {
                    findings.push(SecurityFinding {
                        id: format!("STATIC-001-{}:{}", call.file.display(), call.line),
                        severity: Severity::High,
                        file: call.file.display().to_string(),
                        line: call.line,
                        issue_type: IssueType::ApiKeyExposure,
                        description: "Potential hardcoded API key detected".to_string(),
                        rationale: "API keys should be stored in environment variables or secure vaults, not in code".to_string(),
                        fix: "Move API key to environment variable or use a secrets management service".to_string(),
                    });
                }
            }

            // Check for missing input validation
            if call.wrapper.contains("chat") || call.wrapper.contains("completion") {
                if !call.context.contains("validate") && !call.context.contains("sanitize") {
                    findings.push(SecurityFinding {
                        id: format!("STATIC-002-{}:{}", call.file.display(), call.line),
                        severity: Severity::Medium,
                        file: call.file.display().to_string(),
                        line: call.line,
                        issue_type: IssueType::MissingInputValidation,
                        description: "AI call without apparent input validation".to_string(),
                        rationale: "User inputs to AI models should be validated to prevent prompt injection".to_string(),
                        fix: "Add input validation before passing to AI model".to_string(),
                    });
                }
            }

            // Check for unrestricted model access
            if call
                .model
                .as_ref()
                .map_or(false, |m| m.contains("gpt-4") || m.contains("claude"))
            {
                if !call.context.contains("limit") && !call.context.contains("quota") {
                    findings.push(SecurityFinding {
                        id: format!("STATIC-003-{}:{}", call.file.display(), call.line),
                        severity: Severity::Medium,
                        file: call.file.display().to_string(),
                        line: call.line,
                        issue_type: IssueType::UnrestrictedModelAccess,
                        description: "Expensive model usage without rate limiting".to_string(),
                        rationale: "High-cost models should have usage limits to prevent abuse"
                            .to_string(),
                        fix: "Implement rate limiting or usage quotas for expensive model calls"
                            .to_string(),
                    });
                }
            }
        }

        Ok(findings)
    }

    async fn llm_analysis(
        &self,
        inventory: &Inventory,
        _config: &Config,
    ) -> Result<Vec<SecurityFinding>> {
        // Prepare context for LLM
        let inventory_json = serde_json::to_string_pretty(inventory)?;
        let prompt = self.create_security_prompt(&inventory_json);

        // Estimate tokens and cost
        let model = "gpt-4o";
        let estimated_tokens = self.token_counter.estimate_tokens(&prompt, model);
        
        // Estimate token usage (assuming roughly equal input/output for analysis)
        let token_usage = TokenUsage {
            prompt_tokens: estimated_tokens,
            completion_tokens: estimated_tokens / 2, // Rough estimate
            total_tokens: estimated_tokens + estimated_tokens / 2,
        };
        
        let estimated_cost = self.token_counter.estimate_cost(&token_usage, model);

        // Check budget
        {
            let mut budget = self.budget.lock().await;
            budget.consume(token_usage.total_tokens)?;
            budget.consume_cost(estimated_cost)?;
            
            // Log remaining budget
            if let Some(remaining_tokens) = budget.remaining_tokens() {
                tracing::info!("Remaining token budget: {}", remaining_tokens);
            }
            if let Some(remaining_usd) = budget.remaining_usd() {
                tracing::info!("Remaining cost budget: ${:.2}", remaining_usd);
            }
        }

        // Mock LLM response for now (replace with actual API call)
        // In production, this would call OpenAI/Anthropic API
        let llm_findings = vec![SecurityFinding {
            id: "LLM-001".to_string(),
            severity: Severity::High,
            file: inventory
                .ai_calls
                .first()
                .map(|c| c.file.display().to_string())
                .unwrap_or_default(),
            line: inventory.ai_calls.first().map(|c| c.line).unwrap_or(0),
            issue_type: IssueType::LLM01PromptInjection,
            description: "Potential prompt injection vulnerability".to_string(),
            rationale: "User input is concatenated directly into prompts without sanitization"
                .to_string(),
            fix: "Use prompt templates and validate/sanitize all user inputs".to_string(),
        }];

        Ok(llm_findings)
    }

    fn create_security_prompt(&self, inventory_json: &str) -> String {
        format!(
            r#"You are a senior AI-security reviewer. Analyze the following AI/LLM usage inventory for security vulnerabilities.

Focus on OWASP LLM Top 10 (2024) and MITRE ATT&CK issues:
- LLM01: Prompt Injection
- LLM02: Insecure Output Handling  
- LLM03: Training Data Poisoning
- LLM04: Model Denial of Service
- LLM05: Supply Chain Vulnerabilities
- LLM06: Sensitive Information Disclosure
- LLM07: Insecure Plugin Design
- LLM08: Excessive Agency
- LLM09: Overreliance
- LLM10: Model Theft

Inventory:
{}

Return a JSON array of findings:
[{{
  "id": "unique-id",
  "severity": "critical|high|medium|low|info",
  "file": "path/to/file",
  "line": 123,
  "issue_type": "LLM01PromptInjection",
  "description": "Brief description",
  "rationale": "Why this is a security issue",
  "fix": "How to fix it"
}}]"#,
            inventory_json
        )
    }

    fn calculate_summary(&self, findings: &[SecurityFinding]) -> AuditSummary {
        let mut summary = AuditSummary {
            total_findings: findings.len(),
            ..Default::default()
        };

        for finding in findings {
            match finding.severity {
                Severity::Critical => summary.critical += 1,
                Severity::High => summary.high += 1,
                Severity::Medium => summary.medium += 1,
                Severity::Low => summary.low += 1,
                Severity::Info => summary.info += 1,
            }
        }

        summary
    }
}
