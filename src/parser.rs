use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tree_sitter::{Language, Parser, Query, QueryCursor};

use crate::core::AiCall;

static LANGUAGE_CONFIGS: Lazy<HashMap<&'static str, LanguageConfig>> = Lazy::new(|| {
    let mut configs = HashMap::new();

    configs.insert(
        "rs",
        LanguageConfig {
            language: tree_sitter_rust::language(),
            queries: vec![
                // OpenAI client calls
                r#"(call_expression
                function: (field_expression
                    value: (identifier) @client
                    field: (field_identifier) @method)
                (#match? @client "openai|client|gpt|claude|anthropic")
                (#match? @method "chat|completion|create|generate"))"#,
                // LangChain-like patterns
                r#"(call_expression
                function: (scoped_identifier
                    path: (identifier) @module
                    name: (identifier) @type)
                (#match? @module "langchain|llm|ai")
                (#match? @type "ChatOpenAI|Claude|LLM"))"#,
            ],
        },
    );

    configs.insert(
        "py",
        LanguageConfig {
            language: tree_sitter_python::language(),
            queries: vec![
                // OpenAI API calls
                r#"(call
                function: (attribute
                    object: (identifier) @client
                    attribute: (identifier) @method)
                (#match? @client "openai|client|gpt|claude|anthropic")
                (#match? @method "chat|completion|create|generate|ChatCompletion"))"#,
                // LangChain patterns
                r#"(call
                function: (identifier) @wrapper
                (#match? @wrapper "ChatOpenAI|Claude|ChatAnthropic|LLMChain|ConversationChain"))"#,
                // Autogen patterns
                r#"(call
                function: (attribute
                    object: (identifier) @module
                    attribute: (identifier) @type)
                (#match? @module "autogen")
                (#match? @type "AssistantAgent|UserProxyAgent|GroupChat"))"#,
            ],
        },
    );

    configs.insert(
        "js",
        LanguageConfig {
            language: tree_sitter_javascript::language(),
            queries: vec![
                // OpenAI SDK
                r#"(call_expression
                function: (member_expression
                    object: (identifier) @client
                    property: (property_identifier) @method)
                (#match? @client "openai|gpt|claude|anthropic")
                (#match? @method "chat|completion|create|generate"))"#,
                // Fetch to AI endpoints
                r#"(call_expression
                function: (identifier) @func
                arguments: (arguments
                    (string) @url)
                (#eq? @func "fetch")
                (#match? @url "openai\.com|anthropic\.com|api\.openai|api\.anthropic"))"#,
            ],
        },
    );

    configs.insert(
        "ts",
        LanguageConfig {
            language: tree_sitter_typescript::language_typescript(),
            queries: vec![
                // Same as JS patterns
                r#"(call_expression
                function: (member_expression
                    object: (identifier) @client
                    property: (property_identifier) @method)
                (#match? @client "openai|gpt|claude|anthropic")
                (#match? @method "chat|completion|create|generate"))"#,
            ],
        },
    );

    configs
});

struct LanguageConfig {
    language: Language,
    queries: Vec<&'static str>,
}

pub struct FileParser {
    parsers: std::sync::Mutex<HashMap<String, Parser>>,
}

impl FileParser {
    pub fn new() -> Result<Self> {
        let mut parsers = HashMap::new();

        for (ext, config) in LANGUAGE_CONFIGS.iter() {
            let mut parser = Parser::new();
            parser
                .set_language(config.language)
                .context(format!("Failed to set language for {}", ext))?;
            parsers.insert(ext.to_string(), parser);
        }

        Ok(Self {
            parsers: std::sync::Mutex::new(parsers),
        })
    }

    pub fn parse_file(&self, path: &Path, content: &str) -> Result<Vec<AiCall>> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| anyhow::anyhow!("No file extension"))?;

        let mut parsers = self.parsers.lock().unwrap();
        let parser = parsers
            .get_mut(ext)
            .ok_or_else(|| anyhow::anyhow!("Unsupported file type: {}", ext))?;

        let config = LANGUAGE_CONFIGS
            .get(ext)
            .ok_or_else(|| anyhow::anyhow!("No language config for {}", ext))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse file"))?;

        let mut ai_calls = Vec::new();

        for query_str in &config.queries {
            if let Ok(query) = Query::new(config.language, query_str) {
                let mut cursor = QueryCursor::new();
                let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

                for match_ in matches {
                    if let Some(ai_call) = self.extract_ai_call(path, content, &match_, &query) {
                        ai_calls.push(ai_call);
                    }
                }
            }
        }

        Ok(ai_calls)
    }

    fn extract_ai_call(
        &self,
        path: &Path,
        content: &str,
        match_: &tree_sitter::QueryMatch,
        _query: &Query,
    ) -> Option<AiCall> {
        let node = match_.captures.first()?.node;
        let start = node.start_position();

        // Find the containing function call node
        let mut call_node = node;
        while call_node.kind() != "call_expression"
            && call_node.kind() != "call"
            && call_node.parent().is_some()
        {
            call_node = call_node.parent()?;
        }

        // Extract wrapper name
        let wrapper = self.extract_wrapper_name(call_node, content)?;

        // Extract model if present
        let model = self.extract_model_from_args(call_node, content);

        // Get context (surrounding lines)
        let context = self.extract_context(content, start.row);

        Some(AiCall {
            file: path.to_path_buf(),
            line: start.row + 1,
            column: start.column + 1,
            wrapper,
            model,
            params: serde_json::json!({}), // TODO: Extract actual params
            context,
        })
    }

    fn extract_wrapper_name(&self, node: tree_sitter::Node, content: &str) -> Option<String> {
        let start = node.start_byte();
        let end = node.end_byte();
        let text = &content[start..end.min(content.len())];

        // Extract the function/method name from the call
        if let Some(func_name) = text.split('(').next() {
            Some(func_name.trim().to_string())
        } else {
            None
        }
    }

    fn extract_model_from_args(&self, node: tree_sitter::Node, content: &str) -> Option<String> {
        // Look for model parameter in arguments
        let args_text = &content[node.start_byte()..node.end_byte().min(content.len())];

        // Simple regex to find model parameter
        let model_regex = regex::Regex::new(r#"model\s*[:=]\s*["']([^"']+)["']"#).ok()?;
        if let Some(captures) = model_regex.captures(args_text) {
            captures.get(1).map(|m| m.as_str().to_string())
        } else {
            None
        }
    }

    fn extract_context(&self, content: &str, line: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let start = line.saturating_sub(2);
        let end = (line + 3).min(lines.len());

        lines[start..end].join("\n")
    }
}
