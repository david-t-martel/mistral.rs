# Makefile for derive-utils
#
# Windows-optimized utilities with path normalization
# Build system for cross-platform development

.PHONY: all build test clean install check fmt clippy doc bench release help
.DEFAULT_GOAL := help

# Configuration
CARGO := cargo
TARGET_DIR := target
INSTALL_DIR := $(HOME)/.local/bin
RELEASE_DIR := $(TARGET_DIR)/release
PROFILE := release

# Platform detection
ifeq ($(OS),Windows_NT)
    PLATFORM := windows
    EXE_SUFFIX := .exe
    PATH_SEP := ;
else
    PLATFORM := unix
    EXE_SUFFIX :=
    PATH_SEP := :
endif

# Binaries
BINARIES := winpath fd rg cmd-wrapper pwsh-wrapper bash-wrapper

# Colors for output
RED := \033[31m
GREEN := \033[32m
YELLOW := \033[33m
BLUE := \033[34m
RESET := \033[0m

help: ## Show this help message
	@echo "$(BLUE)Derive Utils Build System$(RESET)"
	@echo ""
	@echo "$(GREEN)Available targets:$(RESET)"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(YELLOW)%-15s$(RESET) %s\n", $$1, $$2}' $(MAKEFILE_LIST)

all: build ## Build all utilities

build: ## Build all binaries in release mode
	@echo "$(GREEN)Building all utilities...$(RESET)"
	$(CARGO) build --release --workspace
	@echo "$(GREEN)Build complete!$(RESET)"

build-dev: ## Build all binaries in development mode
	@echo "$(GREEN)Building all utilities (development mode)...$(RESET)"
	$(CARGO) build --workspace
	@echo "$(GREEN)Development build complete!$(RESET)"

build-fast: ## Build with fast profile for development
	@echo "$(GREEN)Building with fast profile...$(RESET)"
	$(CARGO) build --profile release-fast --workspace
	@echo "$(GREEN)Fast build complete!$(RESET)"

build-small: ## Build with size optimization
	@echo "$(GREEN)Building with size optimization...$(RESET)"
	$(CARGO) build --profile release-small --workspace
	@echo "$(GREEN)Size-optimized build complete!$(RESET)"

test: ## Run all tests
	@echo "$(GREEN)Running tests...$(RESET)"
	$(CARGO) test --workspace
	@echo "$(GREEN)All tests passed!$(RESET)"

test-verbose: ## Run tests with verbose output
	@echo "$(GREEN)Running tests (verbose)...$(RESET)"
	$(CARGO) test --workspace -- --nocapture

test-coverage: ## Generate test coverage report
	@echo "$(GREEN)Generating coverage report...$(RESET)"
	$(CARGO) tarpaulin --workspace --out Html --output-dir target/coverage
	@echo "$(GREEN)Coverage report generated in target/coverage/$(RESET)"

bench: ## Run benchmarks
	@echo "$(GREEN)Running benchmarks...$(RESET)"
	$(CARGO) bench --workspace

check: ## Run cargo check
	@echo "$(GREEN)Checking code...$(RESET)"
	$(CARGO) check --workspace --all-targets --all-features

fmt: ## Format code
	@echo "$(GREEN)Formatting code...$(RESET)"
	$(CARGO) fmt --all

fmt-check: ## Check code formatting
	@echo "$(GREEN)Checking code formatting...$(RESET)"
	$(CARGO) fmt --all -- --check

clippy: ## Run clippy lints
	@echo "$(GREEN)Running clippy...$(RESET)"
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings

clippy-fix: ## Run clippy with automatic fixes
	@echo "$(GREEN)Running clippy with fixes...$(RESET)"
	$(CARGO) clippy --workspace --all-targets --all-features --fix --allow-dirty

doc: ## Generate documentation
	@echo "$(GREEN)Generating documentation...$(RESET)"
	$(CARGO) doc --workspace --no-deps --open

doc-private: ## Generate documentation including private items
	@echo "$(GREEN)Generating documentation (including private)...$(RESET)"
	$(CARGO) doc --workspace --no-deps --document-private-items --open

clean: ## Clean build artifacts
	@echo "$(GREEN)Cleaning build artifacts...$(RESET)"
	$(CARGO) clean
	@echo "$(GREEN)Clean complete!$(RESET)"

clean-all: clean ## Clean all artifacts including cache
	@echo "$(GREEN)Cleaning all artifacts...$(RESET)"
	rm -rf target/
	rm -rf Cargo.lock

install: build ## Install binaries to ~/.local/bin
	@echo "$(GREEN)Installing binaries to $(INSTALL_DIR)...$(RESET)"
	@mkdir -p $(INSTALL_DIR)
	@for binary in $(BINARIES); do \
		if [ -f "$(RELEASE_DIR)/$$binary$(EXE_SUFFIX)" ]; then \
			cp "$(RELEASE_DIR)/$$binary$(EXE_SUFFIX)" "$(INSTALL_DIR)/"; \
			echo "  Installed $$binary$(EXE_SUFFIX)"; \
		fi; \
	done
	@echo "$(GREEN)Installation complete!$(RESET)"

install-dev: build-dev ## Install development binaries
	@echo "$(GREEN)Installing development binaries...$(RESET)"
	@mkdir -p $(INSTALL_DIR)
	@for binary in $(BINARIES); do \
		if [ -f "$(TARGET_DIR)/debug/$$binary$(EXE_SUFFIX)" ]; then \
			cp "$(TARGET_DIR)/debug/$$binary$(EXE_SUFFIX)" "$(INSTALL_DIR)/"; \
			echo "  Installed $$binary$(EXE_SUFFIX) (debug)"; \
		fi; \
	done

uninstall: ## Remove installed binaries
	@echo "$(GREEN)Removing installed binaries...$(RESET)"
	@for binary in $(BINARIES); do \
		if [ -f "$(INSTALL_DIR)/$$binary$(EXE_SUFFIX)" ]; then \
			rm "$(INSTALL_DIR)/$$binary$(EXE_SUFFIX)"; \
			echo "  Removed $$binary$(EXE_SUFFIX)"; \
		fi; \
	done

audit: ## Run security audit
	@echo "$(GREEN)Running security audit...$(RESET)"
	$(CARGO) audit

deny: ## Run cargo-deny checks
	@echo "$(GREEN)Running cargo-deny checks...$(RESET)"
	$(CARGO) deny check

update: ## Update dependencies
	@echo "$(GREEN)Updating dependencies...$(RESET)"
	$(CARGO) update --workspace

outdated: ## Check for outdated dependencies
	@echo "$(GREEN)Checking for outdated dependencies...$(RESET)"
	$(CARGO) outdated --workspace

release: ## Build release packages
	@echo "$(GREEN)Building release packages...$(RESET)"
	$(CARGO) build --release --workspace
	@mkdir -p dist/
	@for binary in $(BINARIES); do \
		if [ -f "$(RELEASE_DIR)/$$binary$(EXE_SUFFIX)" ]; then \
			cp "$(RELEASE_DIR)/$$binary$(EXE_SUFFIX)" "dist/"; \
		fi; \
	done
	@echo "$(GREEN)Release packages created in dist/$(RESET)"

package: release ## Create distribution packages
	@echo "$(GREEN)Creating distribution packages...$(RESET)"
	@cd dist && tar -czf ../derive-utils-$(PLATFORM).tar.gz *
	@echo "$(GREEN)Package created: derive-utils-$(PLATFORM).tar.gz$(RESET)"

cross-build: ## Cross-compile for multiple targets
	@echo "$(GREEN)Cross-compiling for multiple targets...$(RESET)"
	$(CARGO) build --release --target x86_64-pc-windows-msvc
	$(CARGO) build --release --target x86_64-unknown-linux-gnu
	@echo "$(GREEN)Cross-compilation complete!$(RESET)"

validate: check fmt-check clippy test ## Run all validation checks
	@echo "$(GREEN)All validation checks passed!$(RESET)"

ci: validate audit ## Run CI pipeline
	@echo "$(GREEN)CI pipeline completed successfully!$(RESET)"

dev-setup: ## Set up development environment
	@echo "$(GREEN)Setting up development environment...$(RESET)"
	rustup component add rustfmt clippy
	$(CARGO) install cargo-audit cargo-deny cargo-outdated cargo-tarpaulin
	@echo "$(GREEN)Development environment ready!$(RESET)"

quick-test: ## Run quick tests for development
	@echo "$(GREEN)Running quick tests...$(RESET)"
	$(CARGO) test --workspace --lib

watch: ## Watch for changes and rebuild
	@echo "$(GREEN)Watching for changes...$(RESET)"
	$(CARGO) watch -x build

watch-test: ## Watch for changes and run tests
	@echo "$(GREEN)Watching for changes and running tests...$(RESET)"
	$(CARGO) watch -x test

profile: ## Profile the binaries
	@echo "$(GREEN)Building profiling binaries...$(RESET)"
	$(CARGO) build --profile profiling --workspace

# Individual binary targets
winpath: ## Build winpath binary
	$(CARGO) build --release -p winpath

fd: ## Build fd-wrapper binary
	$(CARGO) build --release -p fd-wrapper

rg: ## Build rg-wrapper binary
	$(CARGO) build --release -p rg-wrapper

cmd: ## Build cmd-wrapper binary
	$(CARGO) build --release -p cmd-wrapper

pwsh: ## Build pwsh-wrapper binary
	$(CARGO) build --release -p pwsh-wrapper

bash: ## Build bash-wrapper binary
	$(CARGO) build --release -p bash-wrapper

# Test individual packages
test-winpath: ## Test winpath library
	$(CARGO) test -p winpath

test-fd: ## Test fd-wrapper
	$(CARGO) test -p fd-wrapper

test-rg: ## Test rg-wrapper
	$(CARGO) test -p rg-wrapper

test-cmd: ## Test cmd-wrapper
	$(CARGO) test -p cmd-wrapper

test-pwsh: ## Test pwsh-wrapper
	$(CARGO) test -p pwsh-wrapper

test-bash: ## Test bash-wrapper
	$(CARGO) test -p bash-wrapper

# Utility targets
list-binaries: ## List all built binaries
	@echo "$(GREEN)Built binaries:$(RESET)"
	@ls -la $(RELEASE_DIR)/*$(EXE_SUFFIX) 2>/dev/null || echo "No binaries found. Run 'make build' first."

version: ## Show version information
	@echo "$(GREEN)Version Information:$(RESET)"
	@$(CARGO) --version
	@rustc --version
	@echo "Platform: $(PLATFORM)"

size: ## Show binary sizes
	@echo "$(GREEN)Binary Sizes:$(RESET)"
	@for binary in $(BINARIES); do \
		if [ -f "$(RELEASE_DIR)/$$binary$(EXE_SUFFIX)" ]; then \
			size=$$(stat -f%z "$(RELEASE_DIR)/$$binary$(EXE_SUFFIX)" 2>/dev/null || stat -c%s "$(RELEASE_DIR)/$$binary$(EXE_SUFFIX)" 2>/dev/null); \
			printf "  %-20s %10s bytes\n" "$$binary$(EXE_SUFFIX)" "$$size"; \
		fi; \
	done

deps: ## Show dependency tree
	@echo "$(GREEN)Dependency Tree:$(RESET)"
	$(CARGO) tree --workspace

features: ## Show available features
	@echo "$(GREEN)Available Features:$(RESET)"
	@grep -r "^features" */Cargo.toml || echo "No features defined"

# Performance targets
perf-test: ## Run performance tests
	@echo "$(GREEN)Running performance tests...$(RESET)"
	$(CARGO) test --release --workspace perf

stress-test: ## Run stress tests
	@echo "$(GREEN)Running stress tests...$(RESET)"
	$(CARGO) test --release --workspace stress
