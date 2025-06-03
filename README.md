# aiscan - AI Risk Scanner

[![CI](https://github.com/haasonsaas/aiscan/actions/workflows/ci.yml/badge.svg)](https://github.com/haasonsaas/aiscan/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A blazing-fast Rust CLI tool that inventories AI/LLM usage in codebases, audits for security vulnerabilities, and enforces spend limits.

## Features

- **Fast AST-based scanning** using tree-sitter for 500+ languages
- **Pattern matching** for popular AI frameworks (OpenAI, Anthropic, LangChain, Autogen, CrewAI)
- **Security audit** with OWASP LLM Top 10 vulnerability detection
- **Cost guardrails** with token counting and budget enforcement
- **CI/CD ready** with machine-readable output and exit codes
- **Parallel processing** for blazing-fast performance

## Installation

### From Source

```bash
git clone https://github.com/haasonsaas/aiscan.git
cd aiscan
cargo install --path .
```

### Pre-built Binaries (Coming Soon)

```bash
# macOS
curl -L https://github.com/haasonsaas/aiscan/releases/latest/download/aiscan-darwin-amd64 -o aiscan
chmod +x aiscan

# Linux
curl -L https://github.com/haasonsaas/aiscan/releases/latest/download/aiscan-linux-amd64 -o aiscan
chmod +x aiscan

# Windows
curl -L https://github.com/haasonsaas/aiscan/releases/latest/download/aiscan-windows-amd64.exe -o aiscan.exe
```

## Usage

### Initialize Configuration

First, create a configuration file in your project:

```bash
aiscan init
```

This creates `.aiscan.toml` with default settings:

```toml
[limits]
max_tokens = 50000      # Maximum tokens for LLM analysis
max_requests = 100      # Maximum API requests
max_usd = 20.0         # Maximum spend in USD

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

### Scan for AI Usage

Inventory all AI/LLM calls in your codebase:

```bash
# Scan current directory
aiscan scan .

# Scan specific directory
aiscan scan src/

# Save results to file
aiscan scan . --output inventory.json
```

Example output:
```
AI Usage Inventory Summary
==================================================
Files scanned: 152
Total lines: 12,543
AI/LLM calls found: 23
Scan duration: 245ms

Top AI Wrappers:
  openai_api - 12 calls
  langchain - 6 calls
  anthropic_api - 3 calls
  autogen - 2 calls
```

### Security Audit

Run a comprehensive security audit:

```bash
# Audit current directory
aiscan audit .

# Save detailed report
aiscan audit . --output report.json

# Output as JSON
aiscan audit . --json
```

Example findings:
```
Security Audit Results
==================================================

Summary:
  Total findings: 5
  2 High
  3 Medium

Findings:

1. Potential hardcoded API key detected [HIGH]
   File: src/config.py:23
   Type: ApiKeyExposure
   Rationale: API keys should be stored in environment variables or secure vaults
   Fix: Move API key to environment variable or use a secrets management service

2. AI call without apparent input validation [MEDIUM]
   File: src/chat.py:45
   Type: MissingInputValidation
   Rationale: User inputs to AI models should be validated to prevent prompt injection
   Fix: Add input validation before passing to AI model
```

### CI/CD Integration

Use in your CI pipeline:

```bash
# Returns exit code: 0=clean, 1=vulnerabilities, 137=budget exceeded
aiscan ci . --json
```

#### GitHub Actions Example

```yaml
name: AI Security Scan
on: [push, pull_request]

jobs:
  ai-scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install aiscan
        run: |
          curl -L https://github.com/haasonsaas/aiscan/releases/latest/download/aiscan-linux-amd64 -o aiscan
          chmod +x aiscan
          
      - name: Run AI security scan
        run: ./aiscan ci . --json
```

#### GitLab CI Example

```yaml
ai-security-scan:
  stage: test
  script:
    - curl -L https://github.com/haasonsaas/aiscan/releases/latest/download/aiscan-linux-amd64 -o aiscan
    - chmod +x aiscan
    - ./aiscan ci . --json
  allow_failure: false
```

### Advanced Usage

#### Custom Patterns

Add custom detection patterns in `.aiscan.toml`:

```toml
[[audit.custom_rules]]
id = "CUSTOM-001"
pattern = "my_custom_ai_wrapper"
severity = "high"
message = "Custom AI wrapper detected without rate limiting"
```

#### Baseline Mode

Suppress unchanged findings in CI:

```bash
# Generate baseline
aiscan audit . --output baseline.json

# Check against baseline
aiscan ci . --baseline baseline.json
```

#### Environment Variables

```bash
# Set API key for LLM-powered analysis
export OPENAI_API_KEY=sk-...

# Override config settings
export AISCAN_MAX_TOKENS=100000
export AISCAN_MAX_USD=50.0
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