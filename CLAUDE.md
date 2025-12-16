# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rinzler is a Web API scanning tool under active development. It's designed to be a "somewhat intelligent API scanner" for security testing and reconnaissance of web APIs. Core features now implemented include web crawling with passive security analysis, forced browsing/fuzzing, database persistence, and multi-format report generation (text, JSON).

## Workspace Structure

This is a Rust workspace with three crates:

- **rinzler**: Main binary crate - CLI entry point with clap-based command parsing and handlers
- **rinzler-core**: Core library with crawl orchestration, database management, and reporting
- **rinzler-scanner**: Scanner implementation library with HTML crawling and link extraction

All workspace crates use shared version `0.1.10-alpha` and Rust edition 2024.

## Key Dependencies

- **Async runtime**: tokio with full features
- **HTTP client**: reqwest with json, gzip, and cookies support
- **CLI**: clap v4 with derive and cargo features + clap-cargo for styling
- **Database**: rusqlite with bundled sqlite
- **Graph**: petgraph for relationship modeling
- **Serialization**: serde + serde_json
- **Error handling**: anyhow + thiserror
- **Logging**: tracing + tracing-subscriber
- **HTML parsing**: scraper for extracting links and page structure
- **Progress UI**: indicatif for spinner and progress bar displays
- **Pager**: pager for paginated output (uses less -R for color support)
- **URL handling**: url crate for parsing and validation
- **Path expansion**: shellexpand for tilde expansion in paths
- **Testing**: tempfile for temporary test files

## Common Commands

### Building
```bash
cargo build                    # Build all workspace members
cargo build --release          # Build optimized release version
cargo build -p rinzler         # Build specific crate
```

### Running
```bash
# Initialize database and configuration
cargo run -- init                                    # Initialize at default location (~/.config/rinzler/)
cargo run -- init ~/.local/share/rinzler/            # Initialize at custom location
cargo run -- init --force                            # Force overwrite existing database

# Crawl commands
cargo run -- crawl --url http://example.com          # Crawl a single URL
cargo run -- crawl -H hosts.txt                      # Crawl multiple hosts from file
cargo run -- crawl -u http://example.com -t 20       # Use 20 worker threads
cargo run -- crawl -u http://example.com --follow    # Prompt for cross-domain links
cargo run -- crawl -u http://example.com --auto-follow  # Auto-follow all cross-domain links
cargo run -- crawl -u http://example.com -o report.txt  # Save report to file
cargo run -- crawl -u http://example.com -f json     # Generate JSON format report
cargo run -- crawl -u http://example.com --include-sitemap  # Include sitemap in report

# Fuzz commands
cargo run -- fuzz --url http://example.com           # Fuzz a single URL with default wordlist
cargo run -- fuzz -H hosts.txt -w wordlist.txt       # Fuzz with custom wordlist
cargo run -- fuzz -u http://example.com -t 5         # Fuzz with 5 worker threads

# Other commands
cargo run -- --help                                  # Show help
cargo run -- --quiet crawl -u http://example.com     # Suppress banner output
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
The main binary uses clap for argument parsing with custom styling (via clap-cargo). Implemented commands:

#### `init` - Database Initialization
- `[PATH]`: Location to store database (default: `~/.config/rinzler/`)
- `--force/-f`: Force overwrite of existing database
- Creates directory structure, installs default wordlist, initializes SQLite database

#### `crawl` - Web Crawling (IMPLEMENTED)
- `--url/-u <URL>`: Target URL to crawl
- `--hosts-file/-H <PATH>`: Line-delimited file of URLs to crawl
- `--threads/-t <NUM>`: Number of async worker threads (default: 10)
- `--follow`: Prompt user for each cross-domain link
- `--auto-follow`: Automatically follow all cross-domain links
- `--output/-o <PATH>`: Save report to file (default: display to screen)
- `--format/-f <FORMAT>`: Report format - text, json, csv, html, markdown (default: text)
- `--include-sitemap`: Include visual sitemap tree in report
- Max depth: 3 levels (hardcoded)
- Features:
  - Multi-threaded async crawling with worker pools
  - Progress bars showing per-worker status
  - Cross-domain link detection with three modes (disabled/prompt/auto)
  - HTML parsing to extract links, forms, and scripts
  - Passive security analysis (insecure transport, interesting files, error messages)
  - Database persistence of all findings with severity ratings
  - Multi-format report generation (text, JSON)
  - Optional sitemap visualization in reports
  - Colored output report grouped by host
  - Paginated results using less -R

#### `fuzz` - Forced Browsing/Directory Enumeration (IMPLEMENTED)
- `--url/-u <URL>`: Target URL (default: http://127.0.0.1)
- `--hosts-file/-H <PATH>`: Line-delimited file of hosts to fuzz
- `--wordlist-file/-w <PATH>`: Wordlist (default: `~/.config/rinzler/wordlists/default.txt`)
- `--threads/-t <NUM>`: Worker threads (default: 10)
- Features:
  - Distributed fuzzing across worker threads with progress bars
  - Smart URL construction (base URL + wordlist entries)
  - Concurrent requests with semaphore-based rate limiting
  - Filters responses (saves status < 500)
  - Results grouped by status code in report
  - Shows content length and content type for each finding
  - Default wordlist with 99 API-focused endpoints

#### `workspace` - Workspace Management (STUB)
- `create --name <NAME>`: Create workspace
- `remove --name <NAME>`: Remove workspace
- `list`: List all workspaces
- `rename --old-name <NAME> --new-name <NAME>`: Rename workspace

#### `plugin` - Plugin Management (STUB)
- `list`: List registered plugins
- `register --file <PATH> --name <NAME>`: Register plugin
- `unregister --name <NAME>`: Unregister plugin

### Implemented Components

#### rinzler-scanner (Library)
- **Crawler**: Async web crawler with configurable depth and worker pools
  - `Crawler::new()`: Builder pattern for configuration
  - `.with_max_depth(usize)`: Set crawl depth limit
  - `.with_auto_follow(bool)`: Enable/disable automatic cross-domain following
  - `.with_progress_callback(Arc<Fn>)`: Worker progress reporting
  - `.with_cross_domain_callback(Arc<Fn>)`: Custom cross-domain decision logic
  - `.crawl(url, threads)`: Execute crawl with specified workers
- **CrawlResult**: Data structure for crawl findings
  - Fields: url, status_code, content_type, links_found, forms_found, scripts_found
- **ScanError**: Error handling with thiserror
- Uses scraper for HTML parsing and link extraction

#### rinzler-core (Library)
- **crawl module** (`rinzler_core::crawl`):
  - `execute_crawl()`: High-level crawl execution with progress callbacks
  - `CrawlOptions`: Configuration struct (urls, threads, max_depth, follow_mode, show_progress_bars)
  - `FollowMode`: Enum for cross-domain behavior (Disabled/Prompt/Auto)
  - `generate_crawl_report()`: Format results with colored status codes
  - `extract_url_path()`: Extract path component from URL
- **fuzz module** (`rinzler_core::fuzz`):
  - `execute_fuzz()`: Async forced browsing with worker distribution
  - `FuzzOptions`: Configuration struct (base_urls, wordlist, threads, show_progress_bars)
  - `FuzzResult`: Data structure for fuzz findings (url, status_code, content_length, content_type)
  - `load_wordlist()`: Load and parse wordlist files (filters comments and empty lines)
  - `generate_fuzz_report()`: Format results grouped by status code
  - `build_test_url()`: Construct URLs from base + wordlist entry
- **data module** (`rinzler_core::data`):
  - `Database::new(path)`: Initialize database with optimized SQLite pragmas (WAL mode, 64MB cache)
  - `Database::exists(path)`: Check if database exists
  - `Database::drop(path)`: Delete database file
  - Schema: crawl_sessions, maps, nodes, edges, findings, technologies, http_transactions
  - Enhanced schema with severity ratings, CWE/OWASP categorization, service types
  - Enums: `Severity` (Critical/High/Medium/Low/Info), `FindingType`, `ServiceType`
  - Structs: `CrawlNode`, `Finding` for structured data
  - Methods: `create_session()`, `insert_node()`, `insert_finding()`, `get_findings_by_severity()`
  - Optimizations: WAL journal mode, normal synchronous, memory temp store
- **security module** (`rinzler_core::security`):
  - `analyze_crawl_result()`: Run all passive security checks on crawl results
  - `check_insecure_transport()`: Detect HTTP vs HTTPS
  - `check_interesting_files()`: Detect sensitive files (.git/, .env, backups, configs)
  - `check_error_messages()`: Identify 5xx server errors
  - Each check returns `Finding` with severity, CWE, OWASP category, impact, remediation
- **report module** (`rinzler_core::report`):
  - `gather_report_data()`: Query database for complete report data
  - `generate_text_report()`: Create formatted text report with headers, executive summary, detailed findings
  - `generate_json_report()`: Create structured JSON report with metadata
  - `save_report()`: Write report to file
  - Structures: `ReportData`, `FindingData`, `SeverityCounts`, `ScanInfo`, `SitemapNode`
  - `ReportFormat` enum: Text, Json, Csv, Html, Markdown (csv/html/markdown stubs)
  - Helper functions for timestamp formatting, text wrapping, sitemap tree generation
- **Banner**: ASCII art banner with version info

#### rinzler (Binary)
- **handlers module** (`rinzler::handlers`):
  - `handle_init()`: Interactive database setup with colorful console output
  - `handle_crawl()`: Async crawl execution with progress tracking, security analysis, database persistence, and report generation
  - `handle_fuzz()`: Async forced browsing with wordlist loading and distributed workers
  - `handle_workspace_*()`: Stubs for workspace management
  - `handle_plugin_*()`: Stubs for plugin management
  - URL loading helpers: `load_urls_from_source()`, `load_urls_from_file()`, `parse_url_line()`
- **Default wordlist**: Embedded in binary with `include_str!()` macro (99 API-focused endpoints)
- **Tests**: Unit tests in `rinzler/tests/handlers_tests.rs`

### Design Patterns
- **Callback Architecture**: Progress and cross-domain callbacks use `Arc<dyn Fn>` for thread-safe function sharing
- **Worker Pools**: Tokio-based async workers with progress tracking per worker
- **Builder Pattern**: Crawler configuration uses builder pattern for flexibility
- **Progress UI**: indicatif MultiProgress for concurrent worker status display
- **URL Normalization**: Automatic http:// prefix addition for URLs without schemes

### Planned Components (Not Yet Implemented)
- **Additional report formats**: CSV, HTML, and Markdown generators
- **Advanced fuzzing**: Parameter fuzzing, HTTP method fuzzing, header injection
- **Graph modeling**: Use petgraph for API endpoint relationship mapping
- **TUI interface**: Interactive terminal UI for scan monitoring/control (rinzler-tui crate removed from workspace)
- **Workspace system**: Project/target isolation for managing multiple targets
- **Plugin system**: Extensibility through custom plugins

### Edition 2024
This project uses Rust edition 2024. Be aware of edition-specific features and syntax when making changes.

## Security Context

This tool is designed for authorized security testing and penetration testing. When working on scanning features:
- Implement proper rate limiting and respect robots.txt
- Add controls for scan scope and depth
- Include clear warnings about authorized use only
- Follow responsible disclosure practices in documentation
