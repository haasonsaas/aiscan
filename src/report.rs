use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::audit::AuditResult;
use crate::core::Inventory;

#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub metadata: ReportMetadata,
    pub inventory_summary: InventorySummary,
    pub security_summary: SecuritySummary,
    pub findings: Vec<Finding>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub version: String,
    pub generated_at: DateTime<Utc>,
    pub scan_duration_ms: u64,
    pub tool_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InventorySummary {
    pub total_ai_calls: usize,
    pub unique_wrappers: Vec<String>,
    pub files_with_ai: usize,
    pub most_used_models: Vec<ModelUsage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecuritySummary {
    pub total_findings: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub top_issues: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub severity: String,
    pub file: String,
    pub line: usize,
    pub issue_type: String,
    pub description: String,
    pub fix: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CiReport {
    pub passed: bool,
    pub exit_code: i32,
    pub summary: CiSummary,
    pub failures: Vec<CiFailure>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CiSummary {
    pub files_scanned: usize,
    pub ai_calls_found: usize,
    pub security_issues: usize,
    pub budget_status: BudgetStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BudgetStatus {
    pub tokens_used: usize,
    pub tokens_limit: Option<usize>,
    pub cost_usd: f64,
    pub cost_limit: Option<f64>,
    pub exceeded: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CiFailure {
    pub reason: String,
    pub details: String,
}

impl Report {
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

pub fn generate_report(inventory: &Inventory, audit_result: &AuditResult) -> Result<Report> {
    let mut wrapper_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut model_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut files_with_ai: std::collections::HashSet<String> = std::collections::HashSet::new();

    for call in &inventory.ai_calls {
        *wrapper_counts.entry(call.wrapper.clone()).or_insert(0) += 1;
        if let Some(model) = &call.model {
            *model_counts.entry(model.clone()).or_insert(0) += 1;
        }
        files_with_ai.insert(call.file.display().to_string());
    }

    let unique_wrappers: Vec<String> = wrapper_counts.keys().cloned().collect();
    let mut model_usage: Vec<ModelUsage> = model_counts
        .into_iter()
        .map(|(model, count)| ModelUsage { model, count })
        .collect();
    model_usage.sort_by(|a, b| b.count.cmp(&a.count));

    let top_issues = audit_result
        .findings
        .iter()
        .map(|f| format!("{:?}", f.issue_type))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .take(5)
        .collect();

    let findings = audit_result
        .findings
        .iter()
        .map(|f| Finding {
            id: f.id.clone(),
            severity: format!("{:?}", f.severity),
            file: f.file.clone(),
            line: f.line,
            issue_type: format!("{:?}", f.issue_type),
            description: f.description.clone(),
            fix: f.fix.clone(),
        })
        .collect();

    let recommendations = generate_recommendations(inventory, audit_result);

    Ok(Report {
        metadata: ReportMetadata {
            version: "1.0.0".to_string(),
            generated_at: Utc::now(),
            scan_duration_ms: inventory.scan_duration_ms,
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        inventory_summary: InventorySummary {
            total_ai_calls: inventory.ai_calls.len(),
            unique_wrappers,
            files_with_ai: files_with_ai.len(),
            most_used_models: model_usage,
        },
        security_summary: SecuritySummary {
            total_findings: audit_result.summary.total_findings,
            critical: audit_result.summary.critical,
            high: audit_result.summary.high,
            medium: audit_result.summary.medium,
            low: audit_result.summary.low,
            top_issues,
        },
        findings,
        recommendations,
    })
}

pub fn generate_ci_report(inventory: &Inventory, audit_result: &AuditResult) -> Result<CiReport> {
    let has_critical_issues = audit_result.has_high_severity();
    let budget_exceeded = false; // TODO: Get from scanner - would need to pass budget info

    let mut failures = Vec::new();

    if has_critical_issues {
        failures.push(CiFailure {
            reason: "Critical security issues found".to_string(),
            details: format!(
                "{} critical/high severity findings",
                audit_result.summary.critical + audit_result.summary.high
            ),
        });
    }

    if budget_exceeded {
        failures.push(CiFailure {
            reason: "Budget exceeded".to_string(),
            details: "Token or cost limits have been exceeded".to_string(),
        });
    }

    let exit_code = if has_critical_issues {
        1
    } else if budget_exceeded {
        137
    } else {
        0
    };

    Ok(CiReport {
        passed: failures.is_empty(),
        exit_code,
        summary: CiSummary {
            files_scanned: inventory.files_scanned,
            ai_calls_found: inventory.ai_calls.len(),
            security_issues: audit_result.summary.total_findings,
            budget_status: BudgetStatus {
                tokens_used: 0, // Would need to be passed from scanner
                tokens_limit: Some(50_000),
                cost_usd: 0.0,
                cost_limit: Some(20.0),
                exceeded: budget_exceeded,
            },
        },
        failures,
    })
}

fn generate_recommendations(inventory: &Inventory, audit_result: &AuditResult) -> Vec<String> {
    let mut recommendations = Vec::new();

    // Check for API key exposure
    if audit_result
        .findings
        .iter()
        .any(|f| matches!(f.issue_type, crate::audit::IssueType::ApiKeyExposure))
    {
        recommendations.push(
            "Use environment variables or a secrets management service for API keys".to_string(),
        );
    }

    // Check for missing validation
    if audit_result.findings.iter().any(|f| {
        matches!(
            f.issue_type,
            crate::audit::IssueType::MissingInputValidation
        )
    }) {
        recommendations.push(
            "Implement input validation and sanitization for all user inputs to AI models"
                .to_string(),
        );
    }

    // Check for expensive model usage
    let expensive_models = inventory
        .ai_calls
        .iter()
        .filter_map(|c| c.model.as_ref())
        .filter(|m| m.contains("gpt-4") || m.contains("claude"))
        .count();

    if expensive_models > 10 {
        recommendations.push(
            "Consider using cheaper models for non-critical tasks to reduce costs".to_string(),
        );
    }

    // General recommendations
    if inventory.ai_calls.len() > 50 {
        recommendations.push("Implement centralized AI call management and monitoring".to_string());
    }

    if audit_result.summary.total_findings == 0 {
        recommendations.push(
            "Great job! Continue to monitor AI usage and stay updated on security best practices"
                .to_string(),
        );
    }

    recommendations
}
