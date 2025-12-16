# Rinzler Makefile
# A somewhat intelligent Web API scanner

.PHONY: help build test clean fmt clippy check run install release publish doc ci all install-tools
.PHONY: tag-release

# Default target
.DEFAULT_GOAL := help

# Colors for output
CYAN := \033[0;36m
GREEN := \033[0;32m
YELLOW := \033[0;33m
RED := \033[0;31m
RESET := \033[0m

help: ## Show this help message
	@echo "$(CYAN)Rinzler - Build System$(RESET)"
	@echo ""
	@echo "$(GREEN)Available targets:$(RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-15s$(RESET) %s\n", $$1, $$2}'
	@echo ""

all: fmt clippy test build ## Run format, clippy, tests, and build

build: ## Build the project in debug mode
	@echo "$(GREEN)Building Rinzler...$(RESET)"
	cargo build

release: ## Build the project in release mode
	@echo "$(GREEN)Building Rinzler (release)...$(RESET)"
	cargo build --release

test: ## Run all tests
	@echo "$(GREEN)Running tests...$(RESET)"
	cargo test --all-features --all-targets

test-quiet: ## Run tests with minimal output
	@echo "$(GREEN)Running tests (quiet)...$(RESET)"
	cargo test --quiet --all-features --all-targets

check: ## Fast compile check without building
	@echo "$(GREEN)Checking code...$(RESET)"
	cargo check --all-targets

fmt: ## Format all code
	@echo "$(GREEN)Formatting code...$(RESET)"
	cargo fmt

fmt-check: ## Check code formatting without modifying files
	@echo "$(GREEN)Checking code formatting...$(RESET)"
	cargo fmt -- --check

clippy: ## Run clippy lints
	@echo "$(GREEN)Running clippy...$(RESET)"
	cargo clippy --all-targets --all-features -- -D warnings

clippy-fix: ## Auto-fix clippy warnings
	@echo "$(GREEN)Auto-fixing clippy warnings...$(RESET)"
	cargo clippy --fix --allow-dirty --allow-staged

clean: ## Remove build artifacts
	@echo "$(YELLOW)Cleaning build artifacts...$(RESET)"
	cargo clean
	@echo "$(GREEN)Clean complete!$(RESET)"

run: ## Run the main binary
	@echo "$(GREEN)Running Rinzler...$(RESET)"
	cargo run

run-ui: ## Run the TUI interface
	@echo "$(GREEN)Launching Rinzler TUI...$(RESET)"
	cargo run -- ui

run-release: ## Run the release binary
	@echo "$(GREEN)Running Rinzler (release)...$(RESET)"
	cargo run --release

install: release ## Install the binary to ~/.cargo/bin
	@echo "$(GREEN)Installing Rinzler...$(RESET)"
	cargo install --path ./rinzler

uninstall: ## Uninstall the binary from ~/.cargo/bin
	@echo "$(YELLOW)Uninstalling Rinzler...$(RESET)"
	cargo uninstall rinzler

doc: ## Generate documentation
	@echo "$(GREEN)Generating documentation...$(RESET)"
	cargo doc --no-deps --open

doc-build: ## Generate documentation without opening
	@echo "$(GREEN)Generating documentation...$(RESET)"
	cargo doc --no-deps

publish-dry: ## Dry-run publish to crates.io
	@echo "$(YELLOW)Dry-run publishing to crates.io...$(RESET)"
	@echo "$(YELLOW)Checking rinzler-scanner...$(RESET)"
	cd rinzler-scanner && cargo publish --dry-run --allow-dirty
	@echo "$(YELLOW)Checking rinzler-core...$(RESET)"
	cd rinzler-core && cargo publish --dry-run --allow-dirty
	@echo "$(YELLOW)Checking rinzler...$(RESET)"
	cd rinzler && cargo publish --dry-run --allow-dirty
	@echo "$(GREEN)Dry-run complete!$(RESET)"

publish: ## Publish all crates to crates.io (requires auth)
	@echo "$(RED)WARNING: This will publish to crates.io!$(RESET)"
	@echo "$(YELLOW)Make sure you have:$(RESET)"
	@echo "  1. Updated version numbers in Cargo.toml"
	@echo "  2. Updated CHANGELOG.md"
	@echo "  3. Created a git tag for this version"
	@echo "  4. Run 'make publish-dry' to verify"
	@echo ""
	@read -p "Are you sure you want to publish? [y/N]: " confirm; \
	if [ "$$confirm" = "y" ] || [ "$$confirm" = "Y" ]; then \
		echo "$(GREEN)Publishing rinzler-scanner...$(RESET)"; \
		cd rinzler-scanner && cargo publish; \
		echo "$(YELLOW)Waiting 30s for crates.io to index...$(RESET)"; \
		sleep 30; \
		echo "$(GREEN)Publishing rinzler-core...$(RESET)"; \
		cd rinzler-core && cargo publish; \
		echo "$(YELLOW)Waiting 30s for crates.io to index...$(RESET)"; \
		sleep 30; \
		echo "$(GREEN)Publishing rinzler...$(RESET)"; \
		cd rinzler && cargo publish; \
		echo "$(GREEN)Publish complete!$(RESET)"; \
	else \
		echo "$(YELLOW)Publish cancelled.$(RESET)"; \
	fi

ci: fmt-check clippy test ## Run CI pipeline (fmt, clippy, test)
	@echo "$(GREEN)CI pipeline passed!$(RESET)"

pre-commit: fmt clippy test ## Run pre-commit checks
	@echo "$(GREEN)Pre-commit checks passed!$(RESET)"

watch: ## Watch for changes and rebuild
	@echo "$(GREEN)Watching for changes...$(RESET)"
	@if ! command -v cargo-watch >/dev/null 2>&1; then \
		echo "$(YELLOW)cargo-watch not found. Installing...$(RESET)"; \
		cargo install cargo-watch; \
	fi
	cargo watch -x build

watch-test: ## Watch for changes and run tests
	@echo "$(GREEN)Watching for changes and running tests...$(RESET)"
	@if ! command -v cargo-watch >/dev/null 2>&1; then \
		echo "$(YELLOW)cargo-watch not found. Installing...$(RESET)"; \
		cargo install cargo-watch; \
	fi
	cargo watch -x test

update: ## Update dependencies
	@echo "$(GREEN)Updating dependencies...$(RESET)"
	cargo update

audit: ## Audit dependencies for security vulnerabilities
	@echo "$(GREEN)Auditing dependencies...$(RESET)"
	@if ! command -v cargo-audit >/dev/null 2>&1; then \
		echo "$(YELLOW)cargo-audit not found. Installing...$(RESET)"; \
		cargo install cargo-audit; \
	fi
	cargo audit

bloat: ## Analyze binary size
	@echo "$(GREEN)Analyzing binary size...$(RESET)"
	@if ! command -v cargo-bloat >/dev/null 2>&1; then \
		echo "$(YELLOW)cargo-bloat not found. Installing...$(RESET)"; \
		cargo install cargo-bloat; \
	fi
	cargo bloat --release

bench: ## Run benchmarks
	@echo "$(GREEN)Running benchmarks...$(RESET)"
	cargo bench

version: ## Show current version
	@echo "$(CYAN)Rinzler Version:$(RESET)"
	@grep -m 1 '^version' Cargo.toml | awk -F'"' '{print $$2}'

tag-release: ## Create and push git tag for current version
	@VERSION=$$(grep -m 1 '^version' Cargo.toml | awk -F'"' '{print $$2}'); \
	echo "$(GREEN)Creating tag v$$VERSION...$(RESET)"; \
	git tag -a "v$$VERSION" -m "Release v$$VERSION"; \
	echo "$(GREEN)Pushing tag to remote...$(RESET)"; \
	git push origin "v$$VERSION"; \
	echo "$(GREEN)Tag v$$VERSION created and pushed!$(RESET)"

deps-tree: ## Show dependency tree
	@echo "$(GREEN)Dependency tree:$(RESET)"
	cargo tree

install-tools: ## Install optional cargo tools (audit, bloat, watch)
	@echo "$(GREEN)Installing optional cargo tools...$(RESET)"
	@echo "$(CYAN)Installing cargo-audit...$(RESET)"
	cargo install cargo-audit
	@echo "$(CYAN)Installing cargo-bloat...$(RESET)"
	cargo install cargo-bloat
	@echo "$(CYAN)Installing cargo-watch...$(RESET)"
	cargo install cargo-watch
	@echo "$(GREEN)All tools installed!$(RESET)"
