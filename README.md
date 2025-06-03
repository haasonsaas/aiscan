# aiscan - AI Risk Scanner

A blazing-fast Rust CLI tool that inventories AI/LLM usage in codebases, audits for security vulnerabilities, and enforces spend limits.

## Features

- **Fast AST-based scanning** using tree-sitter for 500+ languages
- **Pattern matching** for popular AI frameworks (OpenAI, Anthropic, LangChain, Autogen, CrewAI)
- **Security audit** with OWASP LLM Top 10 vulnerability detection
- **Cost guardrails** with token counting and budget enforcement
- **CI/CD ready** with machine-readable output and exit codes
- **Parallel processing** for blazing-fast performance

## Installation

```bash
# From source
cargo install --path .

# From crates.io (coming soon)
cargo install aiscan

# Homebrew (coming soon)
brew install haasonsaas/tap/aiscan
```

## Quick Start

```bash
# Initialize configuration
aiscan init

# Scan current directory for AI usage
aiscan scan .

# Run security audit
aiscan audit .

# CI mode with JSON output
aiscan ci . --json
```

## Configuration

Create `.aiscan.toml` in your project root:

```toml
[limits]
max_tokens = 50000
max_requests = 100
max_usd = 20.0

[scan]
exclude_patterns = ["node_modules/**", "venv/**", ".git/**"]
include_hidden = false
follow_symlinks = false
max_file_size_mb = 10

[audit]
llm_model = "gpt-4o"
temperature = 0.1
enable_llm_audit = true
```

## Exit Codes

- `0` - Clean scan, no issues found
- `1` - Security vulnerabilities detected
- `137` - Budget exceeded
- Other - Tool error

## Security Findings

aiscan detects vulnerabilities based on OWASP LLM Top 10:

- **LLM01** - Prompt Injection
- **LLM02** - Insecure Output Handling
- **LLM03** - Training Data Poisoning
- **LLM04** - Model Denial of Service
- **LLM05** - Supply Chain Vulnerabilities
- **LLM06** - Sensitive Information Disclosure
- **LLM07** - Insecure Plugin Design
- **LLM08** - Excessive Agency
- **LLM09** - Overreliance
- **LLM10** - Model Theft

## Performance

- Scans 100k LOC in < 5 seconds
- Parallel file processing with Rayon
- Memory-mapped file reading
- Incremental parsing with tree-sitter

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench

# Format code
cargo fmt

# Lint
cargo clippy
```

## Architecture

```
aiscan/
├── src/
│   ├── cli/          # Command-line interface
│   ├── core/         # Core scanner logic
│   ├── parser/       # Tree-sitter AST parsing
│   ├── patterns/     # AI framework patterns
│   ├── cost/         # Token counting & budgets
│   ├── audit/        # Security vulnerability detection
│   ├── config/       # Configuration management
│   └── report/       # Output formatting
└── tests/            # Integration tests
```

## Contributing

Pull requests welcome! Please read CONTRIBUTING.md first.

## License

MIT - see LICENSE file

## Author

Built by [haasonsaas](https://github.com/haasonsaas)