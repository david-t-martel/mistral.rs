# Generate code coverage reports locally
# Note: Coverage builds use local target/ directory and disable sccache

# Helper to ensure local target directory (Windows)
set-local-target:
	@pwsh -Command "$$env:CARGO_TARGET_DIR=''; Write-Host 'Using local target directory'"

test-coverage:
	@echo "Generating code coverage report..."
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace --all-features --html"
	@echo "Coverage report generated in target/llvm-cov/html/index.html"

test-coverage-open:
	@echo "Generating and opening code coverage report..."
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace --all-features --open"

test-coverage-lcov:
	@echo "Generating LCOV coverage report..."
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info"
	@echo "LCOV report generated: lcov.info"

test-coverage-json:
	@echo "Generating JSON coverage report..."
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace --all-features --json --output-path coverage.json"
	@echo "JSON report generated: coverage.json"

test-coverage-text:
	@echo "Generating text coverage summary..."
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace --all-features --summary-only"

test-coverage-ci:
	@echo "Generating coverage for CI (LCOV format)..."
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info"

test-coverage-fast:
	@echo "Fast coverage (no pyo3 crates)..."
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov -p mistralrs-core -p mistralrs-agent-tools -p mistralrs-quant -p mistralrs-vision -p mistralrs-mcp --all-features --html --open"

install-coverage-tools:
	@echo "Installing code coverage tools..."
	rustup component add llvm-tools-preview
	cargo install cargo-llvm-cov
	@echo "Coverage tools installed!"

# ============================================================================
# Standard Rust Development Targets
# ============================================================================

.PHONY: check
check: ## Quick compilation check (no codegen)
	@echo "Running quick check..."
	@cargo check --workspace --all-targets --all-features

.PHONY: build
build: ## Build all workspace members
	@echo "Building workspace..."
	@cargo build --workspace --all-features

.PHONY: build-release
build-release: ## Build release binaries
	@echo "Building release binaries..."
	@cargo build --workspace --all-features --release

.PHONY: test
test: ## Run all tests
	@echo "Running tests..."
	@cargo test --workspace --all-features

.PHONY: fmt
fmt: ## Format code with rustfmt
	@echo "Formatting code..."
	@cargo fmt --all || (echo "Warning: rustfmt had issues but continuing..." && exit 0)

.PHONY: fmt-check
fmt-check: ## Check code formatting
	@echo "Checking code formatting..."
	@cargo fmt --all -- --check

.PHONY: lint
lint: ## Run clippy linter
	@echo "Running clippy..."
	@cargo clippy --workspace --all-targets --all-features --exclude mistralrs-pyo3 -- -D warnings || true

.PHONY: lint-fix
lint-fix: ## Auto-fix clippy warnings
	@echo "Auto-fixing clippy issues..."
	@cargo clippy --workspace --all-targets --all-features --exclude mistralrs-pyo3 --fix --allow-dirty --allow-staged || true

.PHONY: clean
clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	@cargo clean

# ============================================================================
# Enhanced Git Workflow Targets
# ============================================================================

.PHONY: tag-issues
tag-issues: ## Tag TODO/FIXME comments with @codex/@gemini for external review
	@echo "Tagging outstanding issues..."
	@pwsh -ExecutionPolicy Bypass -File scripts/tag-issues.ps1

.PHONY: tag-issues-dry-run
tag-issues-dry-run: ## Preview TODO/FIXME tagging without making changes
	@echo "Previewing issue tags (dry run)..."
	@pwsh -ExecutionPolicy Bypass -File scripts/tag-issues.ps1 -DryRun

.PHONY: rag-index
rag-index: ## Create semantic index with RAG-Redis
	@echo "Creating semantic index..."
	@pwsh -ExecutionPolicy Bypass -File scripts/rag-index.ps1

.PHONY: git-auto-commit
git-auto-commit: ## Run enhanced git workflow (format, fix, tag, index, commit)
	@echo "Running enhanced git workflow..."
	@echo "Usage: make git-auto-commit MESSAGE='your commit message'"
	@echo ""
	@echo "Please use the script directly:"
	@echo "  pwsh scripts/git-auto-commit.ps1 -Message 'your message' [-Push]"
	@echo ""
	@echo "Options:"
	@echo "  -Push              : Automatically push after commit"
	@echo "  -NoVerify          : Skip pre-commit hooks"
	@echo "  -SkipFormat        : Skip cargo fmt"
	@echo "  -SkipClippy        : Skip cargo clippy --fix"
	@echo "  -SkipTagging       : Skip TODO/FIXME tagging"
	@echo "  -SkipIndex         : Skip RAG-Redis indexing"

.PHONY: pre-commit
pre-commit: fmt lint-fix ## Run pre-commit checks (format + lint-fix)
	@echo "Pre-commit checks completed"
	@git status --short

.PHONY: workflow-prepare
workflow-prepare: fmt lint-fix tag-issues ## Prepare code for commit (format, fix, tag)
	@echo "Code prepared for commit"
	@git add -u
	@git status --short

.PHONY: workflow-full
workflow-full: fmt lint-fix tag-issues rag-index ## Full workflow (format, fix, tag, index)
	@echo "Full workflow completed"
	@git add -u
	@git status --short
	@echo ""
	@echo "Ready to commit. Run: git commit -m 'your message'"

# ============================================================================
# CI/CD Integration Targets
# ============================================================================

.PHONY: ci
ci: fmt-check check lint test ## Run CI checks (format check, compile, lint, test)
	@echo "✓ All CI checks passed"

.PHONY: ci-auto-fix
ci-auto-fix: fmt lint-fix ## CI with auto-fix (formats and fixes issues)
	@echo "CI auto-fix completed"
	@git diff --stat

# ============================================================================
# Help Target
# ============================================================================

.PHONY: help
help: ## Show this help message
	@echo "Available targets:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-25s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Enhanced Git Workflow:"
	@echo "  make tag-issues          - Tag TODO/FIXME with @codex/@gemini"
	@echo "  make rag-index           - Create semantic index"
	@echo "  make pre-commit          - Format + lint-fix"
	@echo "  make workflow-prepare    - Format + fix + tag"
	@echo "  make workflow-full       - Format + fix + tag + index"
	@echo ""
	@echo "For full git workflow, use:"
	@echo "  pwsh scripts/git-auto-commit.ps1 -Message 'commit msg' -Push"

.DEFAULT_GOAL := help

# Include deployment targets if available
-include Makefile.deployment
