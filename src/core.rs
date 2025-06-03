use anyhow::Result;
use dashmap::DashMap;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::audit::{AuditResult, SecurityAuditor};
use crate::config::Config;
use crate::cost::Budget;
use crate::parser::FileParser;
use crate::patterns::PatternMatcher;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiCall {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub wrapper: String,
    pub model: Option<String>,
    pub params: serde_json::Value,
    pub context: String,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Inventory {
    pub ai_calls: Vec<AiCall>,
    pub files_scanned: usize,
    pub total_lines: usize,
    pub scan_duration_ms: u64,
}

impl Inventory {
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn print_summary(&self) {
        use colored::*;

        println!("\n{}", "AI Usage Inventory Summary".bold().green());
        println!("{}", "=".repeat(50));
        println!("Files scanned: {}", self.files_scanned);
        println!("Total lines: {}", self.total_lines);
        println!("AI/LLM calls found: {}", self.ai_calls.len());
        println!("Scan duration: {}ms", self.scan_duration_ms);

        if !self.ai_calls.is_empty() {
            println!("\n{}", "Top AI Wrappers:".bold());
            let mut wrapper_counts: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();
            for call in &self.ai_calls {
                *wrapper_counts.entry(&call.wrapper).or_insert(0) += 1;
            }
            let mut counts: Vec<_> = wrapper_counts.into_iter().collect();
            counts.sort_by(|a, b| b.1.cmp(&a.1));

            for (wrapper, count) in counts.iter().take(5) {
                println!("  {} - {} calls", wrapper, count);
            }
        }
    }
}

pub struct Scanner {
    config: Config,
    budget: Arc<Mutex<Budget>>,
    parser: Arc<FileParser>,
    pattern_matcher: Arc<PatternMatcher>,
}

impl Scanner {
    pub fn new() -> Result<Self> {
        let config = Config::load_or_default()?;
        let budget = Arc::new(Mutex::new(Budget::from_config(&config)));
        let parser = Arc::new(FileParser::new()?);
        let pattern_matcher = Arc::new(PatternMatcher::new());

        Ok(Self {
            config,
            budget,
            parser,
            pattern_matcher,
        })
    }

    pub async fn scan_directory(&self, path: &Path) -> Result<Inventory> {
        let start_time = std::time::Instant::now();
        let progress = ProgressBar::new_spinner();
        progress.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        progress.set_message("Scanning files...");

        let files = self.collect_files(path)?;
        let total_files = files.len();

        progress.set_length(total_files as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} files")
                .unwrap()
                .progress_chars("#>-"),
        );

        let results = Arc::new(DashMap::new());
        let total_lines = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        files.par_iter().for_each(|file_path| {
            if let Ok(ai_calls) = self.scan_file(file_path) {
                if !ai_calls.is_empty() {
                    results.insert(file_path.clone(), ai_calls);
                }
            }
            progress.inc(1);
        });

        progress.finish_with_message("Scan complete!");

        let mut all_calls = Vec::new();
        for entry in results.iter() {
            all_calls.extend(entry.value().clone());
        }

        Ok(Inventory {
            ai_calls: all_calls,
            files_scanned: total_files,
            total_lines: total_lines.load(std::sync::atomic::Ordering::Relaxed),
            scan_duration_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    pub async fn audit_inventory(&self, inventory: &Inventory) -> Result<AuditResult> {
        let auditor = SecurityAuditor::new(self.budget.clone());
        auditor.audit(inventory, &self.config).await
    }

    pub fn is_budget_exceeded(&self) -> bool {
        let budget = self.budget.blocking_lock();
        budget.is_exceeded()
    }

    fn collect_files(&self, path: &Path) -> Result<Vec<PathBuf>> {
        use ignore::WalkBuilder;

        let mut files = Vec::new();
        let walker = WalkBuilder::new(path)
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker {
            let entry = entry?;
            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                if self.is_supported_file(entry.path()) {
                    files.push(entry.path().to_path_buf());
                }
            }
        }

        Ok(files)
    }

    fn is_supported_file(&self, path: &Path) -> bool {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        matches!(
            ext,
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "java" | "rb" | "cpp" | "c" | "cs"
        )
    }

    fn scan_file(&self, path: &Path) -> Result<Vec<AiCall>> {
        let content = std::fs::read_to_string(path)?;
        let mut ai_calls = Vec::new();

        // Parse file with tree-sitter
        if let Ok(parsed_calls) = self.parser.parse_file(path, &content) {
            ai_calls.extend(parsed_calls);
        }

        // Apply pattern matching for additional detection
        let pattern_matches = self.pattern_matcher.find_matches(path, &content);
        ai_calls.extend(pattern_matches);

        // Deduplicate by location
        ai_calls.sort_by_key(|call| (call.file.clone(), call.line, call.column));
        ai_calls.dedup_by_key(|call| (call.file.clone(), call.line, call.column));

        Ok(ai_calls)
    }
}
