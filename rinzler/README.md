# rinzler (binary crate)

Main CLI binary for the Rinzler API scanner.

## Overview

This crate provides the command-line interface and handlers for the Rinzler scanner. It uses clap for argument parsing and delegates actual scanning operations to the rinzler-core and rinzler-scanner libraries.

## Architecture

- **main.rs** - Entry point, command routing, and banner display
- **commands.rs** - Clap command definitions and argument parsing
- **handlers.rs** - Command handler implementations
- **lib.rs** - Re-exports for library usage

## Handler Functions

### Implemented

- `handle_init()` - Database initialization with interactive prompts
- `handle_crawl()` - Async web crawling with progress tracking

### Stubs (coming soon)

- `handle_fuzz()` - Active fuzzing with wordlists
- `handle_workspace_*()` - Workspace management
- `handle_plugin_*()` - Plugin system

## Dependencies

Key dependencies include:

- clap with clap-cargo for CLI styling
- tokio for async runtime
- indicatif for progress UI
- pager for paginated output
- rinzler-core for business logic
- rinzler-scanner for HTTP operations

## Testing

Tests are located in `tests/handlers_tests.rs`:

```bash
cargo test
```
