use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_init_command() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".aiscan.toml");

    // Test that config doesn't exist yet
    assert!(!config_path.exists());

    // Initialize config
    aiscan::config::init_config(temp_dir.path()).unwrap();

    // Verify config was created
    assert!(config_path.exists());

    // Verify content
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[limits]"));
    assert!(content.contains("max_tokens"));
}

#[test]
fn test_pattern_detection() {
    let matcher = aiscan::patterns::PatternMatcher::new();
    let content = r#"
import openai

OPENAI_API_KEY = "sk-test123"
client = openai.Client(api_key=OPENAI_API_KEY)

response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Hello"}]
)
"#;

    let matches = matcher.find_matches(&PathBuf::from("test.py"), content);
    assert!(!matches.is_empty());
    assert!(matches.iter().any(|m| m.wrapper == "openai_config"));
}
