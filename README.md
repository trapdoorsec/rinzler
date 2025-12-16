# rinzler

A somewhat intelligent Web API scanner for security testing and reconnaissance.

> Under active development. Core features implemented: crawling with security analysis, forced browsing/fuzzing, database persistence, and multi-format reporting.

## Features

- **Web Crawling**: Multi-threaded async crawling with configurable depth and worker pools
- **Forced Browsing**: Dictionary-based directory enumeration with distributed workers
- **Security Analysis**: Passive detection of insecure transport, sensitive files, and server errors
- **Cross-domain Control**: Stay on target or follow external links with prompt/auto modes
- **Progress Tracking**: Real-time worker status with progress bars for each thread
- **Multi-format Reports**: Generate reports in text or JSON format with optional sitemaps
- **SQLite Backend**: Persistent storage with severity ratings, CWE/OWASP categorization
- **Embedded Wordlists**: Default API endpoint wordlist with 99 entries included

## Quick Start

Initialize the database and configuration:

```bash
cargo run -- init
```

Crawl a target:

```bash
cargo run -- crawl --url http://example.com
```

Crawl multiple hosts from a file:

```bash
cargo run -- crawl --hosts-file targets.txt --threads 20
```

Enable cross-domain following:

```bash
cargo run -- crawl --url http://example.com --auto-follow
```

## Installation

```bash
git clone https://github.com/trapdoorsec/rinzler
cd rinzler
cargo build --release
```

The binary will be in `target/release/rinzler`.

## Commands

- `init` - Initialize database and configuration directory
- `crawl` - Passively crawl targets and extract API endpoints
- `fuzz` - Actively fuzz targets with wordlists for forced browsing
- `workspace` - Manage scan workspaces (coming soon)
- `plugin` - Manage plugins (coming soon)

Run `rinzler --help` or `rinzler <command> --help` for detailed usage.

## Project Structure

This is a Rust workspace with three crates:

- **rinzler** - CLI binary with command handlers
- **rinzler-core** - Core library with crawl orchestration and database
- **rinzler-scanner** - Scanner engine with HTTP client and HTML parsing

## Development

```bash
# Run tests
cargo test

# Check formatting
cargo fmt -- --check

# Run lints
cargo clippy

# Build documentation
cargo doc --no-deps
```

## Security Notice

This tool is designed for authorized security testing only. Always obtain proper authorization before scanning any systems you do not own or have explicit permission to test.

## License

AGPL-3.0-or-later
