mod audit;
mod cli;
mod config;
mod core;
mod cost;
mod parser;
mod patterns;
mod report;

use anyhow::Result;
use clap::Parser;
use colored::*;
use tracing_subscriber;

use crate::cli::{Cli, Commands};
use crate::core::Scanner;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => {
            config::init_config(&path)?;
            println!("Initialized .aiscan.toml in {}", path.display());
        }
        Commands::Scan { path, output } => {
            let scanner = Scanner::new()?;
            let inventory = scanner.scan_directory(&path).await?;

            if let Some(output_path) = output {
                inventory.save_to_file(&output_path)?;
                println!("Inventory saved to {}", output_path.display());
            } else {
                inventory.print_summary();
            }
        }
        Commands::Audit { path, output, json } => {
            let scanner = Scanner::new()?;
            let inventory = scanner.scan_directory(&path).await?;
            let audit_result = scanner.audit_inventory(&inventory).await?;

            if json || output.is_some() {
                let report = report::generate_report(&inventory, &audit_result)?;
                if let Some(output_path) = output {
                    report.save_to_file(&output_path)?;
                    println!("Audit report saved to {}", output_path.display());
                } else {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
            } else {
                audit_result.print_findings();

                // Display budget status
                let (used_tokens, max_tokens, used_usd, max_usd) =
                    scanner.get_budget_status().await;
                println!("\n{}", "Budget Status:".bold());
                if let Some(max_t) = max_tokens {
                    println!("  Tokens: {} / {} used", used_tokens, max_t);
                }
                if let Some(max_u) = max_usd {
                    println!("  Cost: ${:.2} / ${:.2} used", used_usd, max_u);
                }
            }
        }
        Commands::Ci {
            path,
            baseline: _,
            json,
        } => {
            let scanner = Scanner::new()?;
            let inventory = scanner.scan_directory(&path).await?;
            let audit_result = scanner.audit_inventory(&inventory).await?;

            let exit_code = if audit_result.has_high_severity() {
                1
            } else if scanner.is_budget_exceeded().await {
                137
            } else {
                0
            };

            if json {
                let (used_tokens, max_tokens, used_usd, max_usd) =
                    scanner.get_budget_status().await;
                let mut report = report::generate_ci_report(&inventory, &audit_result)?;

                // Update budget status with actual values
                report.summary.budget_status.tokens_used = used_tokens;
                report.summary.budget_status.tokens_limit = max_tokens;
                report.summary.budget_status.cost_usd = used_usd;
                report.summary.budget_status.cost_limit = max_usd;
                report.summary.budget_status.exceeded = scanner.is_budget_exceeded().await;

                println!("{}", serde_json::to_string(&report)?);
            }

            std::process::exit(exit_code);
        }
    }

    Ok(())
}
