# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rinzler is a Web API scanning tool currently under active development (not ready for use). It's designed to be a "somewhat intelligent API scanner" for security testing and reconnaissance of web APIs.

## Workspace Structure

This is a Rust workspace with four crates:

- **rinzler**: Main binary crate - CLI entry point with clap-based command parsing
- **rinzler-core**: Core library (currently empty/stub)
- **rinzler-scanner**: Scanner implementation library (currently empty/stub)
- **rinzler-tui**: Terminal UI library using ratatui (currently empty/stub)

All workspace crates use shared version `0.1.1-alpha` and Rust edition 2024.

## Key Dependencies

- **Async runtime**: tokio with full features
- **HTTP client**: reqwest with json, gzip, and cookies support
- **CLI**: clap v4 with derive and cargo features
- **TUI**: ratatui v0.29 + crossterm v0.28
- **Database**: rusqlite with bundled sqlite
- **Graph**: petgraph for relationship modeling
- **Serialization**: serde + serde_json
- **Error handling**: anyhow + thiserror
- **Logging**: tracing + tracing-subscriber

## Common Commands

### Building
```bash
cargo build                    # Build all workspace members
cargo build --release          # Build optimized release version
cargo build -p rinzler         # Build specific crate
```

### Running
```bash
cargo run -- scan --url http://example.com  # Run scan subcommand with URL
cargo run -- scan -H hosts.txt              # Run scan with hosts file
cargo run -- --help                         # Show help
```

### Testing
```bash
cargo test                     # Run all tests
cargo test --locked            # Run tests with locked dependencies (CI mode)
cargo test --all-features      # Run tests with all features enabled
cargo test --all-targets       # Run tests for all targets
```

### Code Quality
```bash
cargo fmt                      # Format code
cargo fmt -- --check          # Check formatting without modifying files
cargo clippy                  # Run lints
cargo check                   # Fast check without building
cargo doc --no-deps           # Generate documentation
```

## CI Pipeline

The project uses GitHub Actions with the following checks:

1. **fmt**: Formatting check with `cargo fmt -- --check`
2. **clippy**: Linting with clippy-check action
3. **doc**: Documentation generation on nightly with `--cfg docsrs`
4. **test**: Cross-platform testing on macOS and Windows with `--locked --all-features --all-targets`

All CI jobs use Swatinem/rust-cache for dependency caching.

## Architecture Notes

### CLI Structure
The main binary uses clap for argument parsing with custom styling (via clap-cargo). Currently implements a `scan` subcommand that accepts:
- `--url/-u`: Target URL (defaults to http://127.0.0.1)
- `--hosts-file/-H`: Path to line-delimited list of hosts
- `--quiet`: Global quiet flag

### Planned Components
Based on dependencies, the architecture will likely include:
- **Scanner engine**: HTTP-based API scanning with reqwest
- **State management**: SQLite database for storing scan results and state
- **Graph modeling**: API endpoint relationship mapping using petgraph
- **TUI interface**: Interactive terminal UI for scan monitoring/control
- **Async processing**: Tokio-based concurrent scanning

### Edition 2024
This project uses Rust edition 2024. Be aware of edition-specific features and syntax when making changes.

## Security Context

This tool is designed for authorized security testing and penetration testing. When working on scanning features:
- Implement proper rate limiting and respect robots.txt
- Add controls for scan scope and depth
- Include clear warnings about authorized use only
- Follow responsible disclosure practices in documentation
