use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "aiscan")]
#[command(author = "Haasonsaas")]
#[command(version = "0.1.0")]
#[command(about = "Fast security scanner for AI/LLM usage in codebases", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize configuration file
    Init {
        /// Directory to initialize config in
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Scan codebase for AI/LLM usage inventory
    Scan {
        /// Path to scan
        path: PathBuf,

        /// Output file for inventory JSON
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Run security audit on AI/LLM usage
    Audit {
        /// Path to scan and audit
        path: PathBuf,

        /// Output file for audit report
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// CI mode with machine-readable output
    Ci {
        /// Path to scan and audit
        path: PathBuf,

        /// Baseline file to suppress unchanged findings
        #[arg(long)]
        baseline: Option<PathBuf>,

        /// Output as JSON (always enabled in CI mode)
        #[arg(long, default_value = "true")]
        json: bool,
    },
}
