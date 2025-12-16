# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rinzler is a Web API scanning tool under active development. It's designed to be a "somewhat intelligent API scanner" for security testing and reconnaissance of web APIs. The core web crawling functionality is now implemented and functional, with database initialization and result reporting working.

## Workspace Structure

This is a Rust workspace with three crates:

- **rinzler**: Main binary crate - CLI entry point with clap-based command parsing and handlers
- **rinzler-core**: Core library with crawl orchestration, database management, and reporting
- **rinzler-scanner**: Scanner implementation library with HTML crawling and link extraction

All workspace crates use shared version `0.1.19-alpha-251216023011` and Rust edition 2024.

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

# Fuzz commands (not yet implemented)
cargo run -- fuzz --url http://example.com           # Fuzz a single URL
cargo run -- fuzz -H hosts.txt -w wordlist.txt       # Fuzz with custom wordlist

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
- Max depth: 3 levels (hardcoded)
- Features:
  - Multi-threaded async crawling with worker pools
  - Progress bars showing per-worker status
  - Cross-domain link detection with three modes (disabled/prompt/auto)
  - HTML parsing to extract links, forms, and scripts
  - Colored output report grouped by host
  - Paginated results using less

#### `fuzz` - Active Fuzzing (STUB)
- `--url/-u <URL>`: Target URL
- `--hosts-file/-H <PATH>`: Hosts file
- `--wordlist-file/-w <PATH>`: Wordlist (default: `~/.config/rinzler/wordlist`)
- `--threads/-t <NUM>`: Worker threads (default: 10)

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
- **data module** (`rinzler_core::data`):
  - `Database::new(path)`: Initialize database with optimized SQLite pragmas (WAL mode, 64MB cache)
  - `Database::exists(path)`: Check if database exists
  - `Database::drop(path)`: Delete database file
  - Schema: crawl_sessions, maps, nodes, edges (for graph modeling)
  - Optimizations: WAL journal mode, normal synchronous, memory temp store
- **Banner**: ASCII art banner with version info

#### rinzler (Binary)
- **handlers module** (`rinzler::handlers`):
  - `handle_init()`: Interactive database setup with spinner animations
  - `handle_crawl()`: Async crawl execution with progress tracking and report generation
  - `handle_fuzz()`: Stub for future implementation
  - `handle_workspace_*()`: Stubs for workspace management
  - `handle_plugin_*()`: Stubs for plugin management
  - URL loading helpers: `load_urls_from_source()`, `load_urls_from_file()`, `parse_url_line()`
- **Default wordlist**: Embedded in binary with `include_str!()` macro
- **Tests**: Unit tests in `rinzler/tests/handlers_tests.rs`

### Design Patterns
- **Callback Architecture**: Progress and cross-domain callbacks use `Arc<dyn Fn>` for thread-safe function sharing
- **Worker Pools**: Tokio-based async workers with progress tracking per worker
- **Builder Pattern**: Crawler configuration uses builder pattern for flexibility
- **Progress UI**: indicatif MultiProgress for concurrent worker status display
- **URL Normalization**: Automatic http:// prefix addition for URLs without schemes

### Planned Components (Not Yet Implemented)
- **Fuzzing engine**: Dictionary-based endpoint discovery
- **Graph modeling**: Use petgraph for API endpoint relationship mapping
- **TUI interface**: Interactive terminal UI for scan monitoring/control (rinzler-tui crate removed from workspace)
- **Workspace system**: Project/target isolation
- **Plugin system**: Extensibility through custom plugins

### Edition 2024
This project uses Rust edition 2024. Be aware of edition-specific features and syntax when making changes.

## Security Context

This tool is designed for authorized security testing and penetration testing. When working on scanning features:
- Implement proper rate limiting and respect robots.txt
- Add controls for scan scope and depth
- Include clear warnings about authorized use only
- Follow responsible disclosure practices in documentation
