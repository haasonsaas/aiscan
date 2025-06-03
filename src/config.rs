use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub limits: Limits,
    pub scan: ScanConfig,
    pub audit: AuditConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    pub max_tokens: Option<usize>,
    pub max_requests: Option<usize>,
    pub max_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub exclude_patterns: Vec<String>,
    pub include_hidden: bool,
    pub follow_symlinks: bool,
    pub max_file_size_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub llm_model: String,
    pub temperature: f32,
    pub enable_llm_audit: bool,
    pub custom_rules: Vec<CustomRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRule {
    pub id: String,
    pub pattern: String,
    pub severity: String,
    pub message: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            limits: Limits {
                max_tokens: Some(50_000),
                max_requests: Some(100),
                max_usd: Some(20.0),
            },
            scan: ScanConfig {
                exclude_patterns: vec![
                    "node_modules/**".to_string(),
                    "venv/**".to_string(),
                    ".git/**".to_string(),
                    "dist/**".to_string(),
                    "build/**".to_string(),
                    "target/**".to_string(),
                ],
                include_hidden: false,
                follow_symlinks: false,
                max_file_size_mb: 10,
            },
            audit: AuditConfig {
                llm_model: "gpt-4o".to_string(),
                temperature: 0.1,
                enable_llm_audit: true,
                custom_rules: vec![],
            },
        }
    }
}

impl Config {
    pub fn load_or_default() -> Result<Self> {
        let config_path = Path::new(".aiscan.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let toml_string = toml::to_string_pretty(self)?;
        std::fs::write(path, toml_string)?;
        Ok(())
    }
}

pub fn init_config(path: &Path) -> Result<()> {
    let config_path = path.join(".aiscan.toml");

    if config_path.exists() {
        anyhow::bail!(
            "Configuration file already exists at {}",
            config_path.display()
        );
    }

    let default_config = Config::default();
    default_config.save(&config_path)?;

    // Also create a .gitignore entry
    let gitignore_path = path.join(".gitignore");
    if gitignore_path.exists() {
        let mut content = std::fs::read_to_string(&gitignore_path)?;
        if !content.contains("ai_inventory.json") {
            content.push_str("\n# AI Risk Scanner\nai_inventory.json\nai_audit_report.json\n");
            std::fs::write(&gitignore_path, content)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".aiscan.toml");

        let config = Config::default();
        config.save(&config_path).unwrap();

        assert!(config_path.exists());

        let loaded = Config::load_or_default().unwrap();
        assert_eq!(loaded.limits.max_tokens, config.limits.max_tokens);
    }
}
