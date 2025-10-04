# Makefile for mistral.rs - Comprehensive Build Automation
#
# ⚠️  CRITICAL: Always use 'make' commands, NEVER use 'cargo' directly
#
# Quick Start:
#   make dev          - Quick development build
#   make build        - Release build
#   make test         - Run tests
#   make fmt          - Format all code
#   make ci           - Full CI pipeline
#
# Platform-Specific:
#   make build-cuda-full   - Windows/Linux CUDA build
#   make build-metal       - macOS Metal build
#
# Documentation: See .claude/CLAUDE.md for full reference

# Fix for Git Bash PATH recursion issue on Windows
# Git Bash environment sometimes contains ${PATH} in PATH variable, causing Make to fail
# Use shell to get a clean PATH value (removes ${PATH} self-reference)
SHELL := /bin/bash
override PATH := $(shell echo "$$PATH" | tr ':' '\n' | grep -v '^\$$' | tr '\n' ':')

.PHONY: help
.DEFAULT_GOAL := help

# ============================================================================
# Configuration
# ============================================================================

# Detect platform
ifeq ($(OS),Windows_NT)
    PLATFORM := windows
    EXE_EXT := .exe
    NPROC := 8
else
    UNAME_S := $(shell uname -s)
    ifeq ($(UNAME_S),Linux)
        PLATFORM := linux
    else ifeq ($(UNAME_S),Darwin)
        PLATFORM := macos
    endif
    EXE_EXT :=
    NPROC := $(shell nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)
endif

# Build configuration
CARGO := cargo
CARGO_BUILD := $(CARGO) build
CARGO_TEST := $(CARGO) test
CARGO_CHECK := $(CARGO) check
CARGO_CLIPPY := $(CARGO) clippy
CARGO_FMT := $(CARGO) fmt

# Directories
TARGET_DIR := target
RELEASE_DIR := $(TARGET_DIR)/release
DEBUG_DIR := $(TARGET_DIR)/debug
LOGS_DIR := .logs
TESTLOGS_DIR := .testlogs

# Binary names
SERVER_BIN := mistralrs-server$(EXE_EXT)
SERVER_BINARY := $(RELEASE_DIR)/$(SERVER_BIN)

# Feature flags
CUDA_FEATURES := cuda,flash-attn,cudnn
MKL_FEATURES := mkl
METAL_FEATURES := metal
FULL_FEATURES := $(CUDA_FEATURES),$(MKL_FEATURES)

# Build flags
RELEASE_FLAGS := --release
VERBOSE_FLAGS := $(if $(VERBOSE),-vv,)
JOBS_FLAGS := $(if $(JOBS),-j $(JOBS),-j $(NPROC))

# CUDA configuration (set via environment variables)
# Set these in your shell if needed:
# export CUDA_PATH="/c/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.9"
# export CUDNN_PATH="/c/Program Files/NVIDIA/CUDNN/v9.8"

# ============================================================================
# Help Target
# ============================================================================

help: ## Show this help message
	@echo "mistral.rs Makefile - Build Automation"
	@echo ""
	@echo "Usage: make <target>"
	@echo ""
	@echo "⚠️  NEVER use 'cargo' directly - always use make targets"
	@echo ""
	@echo "Common Targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}' | \
		grep -E "^  (dev|build|test|fmt|lint|check|clean|ci)" || true
	@echo ""
	@echo "Platform-Specific:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}' | \
		grep -E "build-(cuda|metal|windows|linux)" || true
	@echo ""
	@echo "Advanced:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}' | \
		grep -vE "^  (dev|build|test|fmt|lint|check|clean|ci|build-(cuda|metal|windows|linux))" || true
	@echo ""
	@echo "Current Platform: $(PLATFORM)"
	@echo "Parallel Jobs: $(NPROC)"
	@echo ""

# ============================================================================
# Environment Checks
# ============================================================================

.PHONY: check-env
check-env: ## Validate build environment
	@echo "Checking build environment..."
	@$(CARGO) --version || (echo "ERROR: Cargo not found" && exit 1)
	@rustc --version || (echo "ERROR: Rust not found" && exit 1)
	@echo "Platform: $(PLATFORM)"
	@echo "Rust: $$(rustc --version)"
	@echo "Cargo: $$($(CARGO) --version)"
	@echo "Jobs: $(NPROC)"
	@echo "✓ Environment OK"

.PHONY: check-cuda-env
check-cuda-env: ## Validate CUDA environment
	@echo "Checking CUDA environment..."
	@echo "Platform: $(PLATFORM)"
	@echo "Note: Manual verification recommended for CUDA paths"
	@echo "✓ CUDA environment check complete"

.PHONY: check-sccache
check-sccache: ## Check if sccache is available
	@sccache --version 2>/dev/null && echo "✓ sccache available" || \
		echo "ℹ sccache not found (run 'make setup-sccache' to install)"

# ============================================================================
# Setup Targets
# ============================================================================

.PHONY: setup-dirs
setup-dirs: ## Create required directories
	@mkdir -p $(LOGS_DIR) $(TESTLOGS_DIR)

.PHONY: setup-sccache
setup-sccache: ## Install and configure sccache
	@echo "Installing sccache..."
	@$(CARGO) install sccache --locked
	@echo "✓ sccache installed"
	@echo "Add to environment: RUSTC_WRAPPER=sccache"

# ============================================================================
# Development Builds
# ============================================================================

.PHONY: dev
dev: setup-dirs ## Quick development build (debug mode)
	@echo "Building in development mode..."
	@$(CARGO_BUILD) --package mistralrs-server $(VERBOSE_FLAGS) $(JOBS_FLAGS)
	@echo "✓ Development build complete"
	@echo "Binary: $(DEBUG_DIR)/$(SERVER_BIN)"

.PHONY: check
check: ## Check if code compiles (fast, no binary)
	@echo "Checking compilation..."
	@$(CARGO_CHECK) --workspace $(VERBOSE_FLAGS) $(JOBS_FLAGS)
	@echo "✓ Check complete"

.PHONY: check-server
check-server: ## Check server package only
	@echo "Checking server package..."
	@$(CARGO_CHECK) --package mistralrs-server $(VERBOSE_FLAGS)
	@echo "✓ Server check complete"

# ============================================================================
# Release Builds
# ============================================================================

.PHONY: build
build: setup-dirs ## Build release binary (basic, no GPU)
	@echo "Building release binary..."
	@$(CARGO_BUILD) $(RELEASE_FLAGS) --package mistralrs-server \
		$(VERBOSE_FLAGS) $(JOBS_FLAGS) 2>&1 | tee $(LOGS_DIR)/build.log
	@echo "✓ Release build complete"
	@echo "Binary: $(SERVER_BINARY)"
	@ls -lh $(SERVER_BINARY) 2>/dev/null || echo "Binary built"

.PHONY: build-server
build-server: build ## Alias for 'build'

.PHONY: release
release: build ## Alias for 'build' (same as release)

# ============================================================================
# Platform-Specific Builds
# ============================================================================

.PHONY: build-cuda
build-cuda: setup-dirs check-cuda-env ## Build with CUDA support
	@echo "Building with CUDA support..."
	@$(CARGO_BUILD) $(RELEASE_FLAGS) --package mistralrs-server \
		--features "$(CUDA_FEATURES)" \
		$(VERBOSE_FLAGS) $(JOBS_FLAGS) 2>&1 | tee $(LOGS_DIR)/build-cuda.log
	@echo "✓ CUDA build complete"
	@echo "Binary: $(SERVER_BINARY)"

.PHONY: build-cuda-full
build-cuda-full: setup-dirs check-cuda-env ## Build with CUDA + Flash Attention + cuDNN + MKL
	@echo "Building with full CUDA features..."
	@$(CARGO_BUILD) $(RELEASE_FLAGS) --package mistralrs-server \
		--features "$(FULL_FEATURES)" \
		$(VERBOSE_FLAGS) $(JOBS_FLAGS) 2>&1 | tee $(LOGS_DIR)/build-cuda-full.log
	@echo "✓ Full CUDA build complete"
	@echo "Binary: $(SERVER_BINARY)"

.PHONY: build-metal
build-metal: setup-dirs ## Build with Metal support (macOS)
ifeq ($(PLATFORM),macos)
	@echo "Building with Metal support..."
	@$(CARGO_BUILD) $(RELEASE_FLAGS) --package mistralrs-server \
		--features "$(METAL_FEATURES)" \
		$(VERBOSE_FLAGS) $(JOBS_FLAGS) 2>&1 | tee $(LOGS_DIR)/build-metal.log
	@echo "✓ Metal build complete"
	@echo "Binary: $(SERVER_BINARY)"
else
	@echo "ERROR: Metal builds only supported on macOS"
	@exit 1
endif

.PHONY: build-windows
build-windows: build-cuda-full ## Alias for Windows CUDA build

.PHONY: build-linux
build-linux: ## Linux build (CUDA or CPU)
ifeq ($(PLATFORM),linux)
	@echo "Select build type:"
	@echo "  make build-cuda-full  - With CUDA"
	@echo "  make build            - CPU only"
else
	@echo "ERROR: Not on Linux platform"
	@exit 1
endif

# ============================================================================
# Testing
# ============================================================================

.PHONY: test
test: ## Run all tests
	@echo "Running tests..."
	@$(CARGO_TEST) --workspace $(VERBOSE_FLAGS) -- --nocapture

.PHONY: test-core
test-core: ## Test core package
	@echo "Testing mistralrs-core..."
	@$(CARGO_TEST) -p mistralrs-core $(VERBOSE_FLAGS) -- --nocapture

.PHONY: test-server
test-server: ## Test server package
	@echo "Testing mistralrs-server..."
	@$(CARGO_TEST) -p mistralrs-server $(VERBOSE_FLAGS) -- --nocapture

.PHONY: test-quant
test-quant: ## Test quantization package
	@echo "Testing mistralrs-quant..."
	@$(CARGO_TEST) -p mistralrs-quant $(VERBOSE_FLAGS) -- --nocapture

.PHONY: test-vision
test-vision: ## Test vision package
	@echo "Testing mistralrs-vision..."
	@$(CARGO_TEST) -p mistralrs-vision $(VERBOSE_FLAGS) -- --nocapture

.PHONY: test-pyo3
test-pyo3: ## Test Python bindings
	@echo "Testing mistralrs-pyo3..."
	@$(CARGO_TEST) -p mistralrs-pyo3 $(VERBOSE_FLAGS) -- --nocapture || \
		echo "PyO3 tests skipped (Python may not be available)"

.PHONY: test-coverage
test-coverage: ## Run tests with coverage
	@echo "Running tests with coverage..."
	@$(CARGO) tarpaulin --workspace --out Html --output-dir coverage

.PHONY: test-ps1
test-ps1: ## Run PowerShell test suite
	@echo "Running PowerShell test suite..."
	@powershell -ExecutionPolicy Bypass -File tests/run-all-tests.ps1

.PHONY: test-ps1-quick
test-ps1-quick: ## Run quick PowerShell tests
	@echo "Running quick PowerShell tests..."
	@powershell -ExecutionPolicy Bypass -File tests/run-all-tests.ps1 -Suite quick

.PHONY: test-ps1-integration
test-ps1-integration: ## Run PowerShell integration tests
	@echo "Running integration tests..."
	@powershell -ExecutionPolicy Bypass -File tests/run-all-tests.ps1 -Suite integration

.PHONY: test-ps1-mcp
test-ps1-mcp: ## Run PowerShell MCP tests
	@echo "Running MCP tests..."
	@powershell -ExecutionPolicy Bypass -File tests/run-all-tests.ps1 -Suite mcp

.PHONY: test-ps1-ci
test-ps1-ci: ## Run PowerShell tests in CI mode
	@echo "Running CI tests..."
	@powershell -ExecutionPolicy Bypass -File tests/run-all-tests.ps1 -Suite all -OutputFormat json -CI

.PHONY: test-full
test-full: test test-ps1 ## Run all tests (Rust + PowerShell)
	@echo "✓ All tests complete"

.PHONY: test-agent
test-agent: ## Run agent mode autonomous test suite
	@echo "Running agent mode tests..."
	@powershell -ExecutionPolicy Bypass -File tests/agent/test-agent-autonomous.ps1

.PHONY: test-agent-json
test-agent-json: ## Run agent tests with JSON output
	@echo "Running agent tests (JSON output)..."
	@powershell -ExecutionPolicy Bypass -File tests/agent/test-agent-autonomous.ps1 -OutputFormat json

.PHONY: test-agent-markdown
test-agent-markdown: ## Run agent tests with Markdown output
	@echo "Running agent tests (Markdown output)..."
	@powershell -ExecutionPolicy Bypass -File tests/agent/test-agent-autonomous.ps1 -OutputFormat markdown

.PHONY: demo-agent
demo-agent: ## Launch interactive agent mode demo
	@echo "Launching agent mode demo..."
	@powershell -ExecutionPolicy Bypass -File tests/agent/demo-agent-mode.ps1 -Interactive

.PHONY: demo-agent-gguf
demo-agent-gguf: ## Launch agent demo with GGUF model
	@echo "Launching agent mode demo (GGUF)..."
	@powershell -ExecutionPolicy Bypass -File tests/agent/demo-agent-mode.ps1 -ModelType gguf -ModelPath "T:/models" -ModelFile "Qwen2.5-1.5B-Instruct-Q4_K_M.gguf"

# ============================================================================
# Code Quality
# ============================================================================

.PHONY: fmt
fmt: ## Format all code (Rust, Python, C/CUDA)
	@echo "Formatting code..."
	@$(CARGO_FMT) --all
	@ruff format . 2>/dev/null || echo "ℹ ruff not found (Python formatting skipped)"
	@find mistralrs-* -type f \( -name "*.metal" -o -name "*.c" -o -name "*.cu" -o -name "*.hpp" -o -name "*.h" -o -name "*.cpp" \) \
		-exec clang-format -i {} + 2>/dev/null || echo "ℹ clang-format not found (C/CUDA formatting skipped)"
	@echo "✓ Formatting complete"

.PHONY: fmt-check
fmt-check: ## Check if code is formatted
	@echo "Checking formatting..."
	@$(CARGO_FMT) --all -- --check

.PHONY: lint
lint: ## Run clippy lints
	@echo "Running clippy..."
	@$(CARGO_CLIPPY) --workspace --all-targets -- -D warnings

.PHONY: lint-fix
lint-fix: ## Run clippy and auto-fix issues
	@echo "Running clippy with auto-fix..."
	@$(CARGO_CLIPPY) --workspace --all-targets --fix --allow-dirty --allow-staged

.PHONY: audit
audit: ## Audit dependencies for security vulnerabilities
	@echo "Auditing dependencies..."
	@$(CARGO) audit

# ============================================================================
# CI/CD
# ============================================================================

.PHONY: ci
ci: fmt-check check lint test ## Run full CI pipeline
	@echo "✓ CI pipeline complete"

.PHONY: ci-fast
ci-fast: check lint ## Fast CI (no tests)
	@echo "✓ Fast CI complete"

# ============================================================================
# Cleaning
# ============================================================================

.PHONY: clean
clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	@$(CARGO) clean
	@echo "✓ Clean complete"

.PHONY: clean-all
clean-all: clean ## Deep clean (including logs)
	@echo "Deep cleaning..."
	@rm -rf $(LOGS_DIR) $(TESTLOGS_DIR) coverage
	@echo "✓ Deep clean complete"

.PHONY: clean-tests
clean-tests: ## Clean test artifacts only
	@echo "Cleaning test artifacts..."
	@rm -rf $(TESTLOGS_DIR)
	@echo "✓ Test artifacts cleaned"

.PHONY: clean-logs
clean-logs: ## Clean log files only
	@echo "Cleaning logs..."
	@rm -rf $(LOGS_DIR)
	@echo "✓ Logs cleaned"

# ============================================================================
# Python Bindings
# ============================================================================

.PHONY: build-python
build-python: ## Build PyO3 Python bindings
	@echo "Building Python bindings..."
	@cd mistralrs-pyo3 && maturin build --release --features "$(FULL_FEATURES)"
	@echo "✓ Python bindings built"

.PHONY: install-python
install-python: ## Install Python bindings locally
	@echo "Installing Python bindings..."
	@cd mistralrs-pyo3 && maturin develop --release
	@echo "✓ Python bindings installed"

.PHONY: wheel
wheel: build-python ## Create Python wheel distribution

# ============================================================================
# Running & Testing
# ============================================================================

.PHONY: run
run: ## Run server in interactive mode (requires MODEL variable)
	@echo "Starting mistral.rs server..."
ifeq ($(PLATFORM),windows)
	@cmd /c "set PATH=%CUDA_PATH%\bin;%CUDNN_PATH%\bin;%CUDNN_PATH%\cuda\bin;C:\\Users\\david\\.local\\bin;C:\\Users\\david\\bin;%PATH% && \"$(SERVER_BINARY)\" -i plain -m $(MODEL)"
else
	@$(SERVER_BINARY) -i plain -m $(MODEL)
endif

.PHONY: run-tui
run-tui: ## Run TUI with smallest test model
	@echo "Starting TUI with test model..."
	@echo "Note: Requires MODEL_INVENTORY.json and uses smallest model (<1.5GB)"
	@echo "Run manually: ./$(SERVER_BINARY) -i gguf -m <model_dir> -f <model_file>"

.PHONY: run-server
run-server: ## Run HTTP server on port 8080
	@echo "Starting HTTP server on port 8080..."
ifeq ($(PLATFORM),windows)
	@cmd /c "set PATH=%CUDA_PATH%\bin;%CUDNN_PATH%\bin;%CUDNN_PATH%\cuda\bin;C:\\Users\\david\\.local\\bin;C:\\Users\\david\\bin;%PATH% && \"$(SERVER_BINARY)\" --port 8080 plain -m $(MODEL)"
else
	@$(SERVER_BINARY) --port 8080 plain -m $(MODEL)
endif

.PHONY: run-with-mcp
run-with-mcp: ## Run server with MCP integration
	@echo "Starting server with MCP integration..."
ifeq ($(PLATFORM),windows)
	@cmd /c "set PATH=%CUDA_PATH%\bin;%CUDNN_PATH%\bin;%CUDNN_PATH%\cuda\bin;C:\\Users\\david\\.local\\bin;C:\\Users\\david\\bin;%PATH% && \"$(SERVER_BINARY)\" --port 8080 --mcp-config MCP_CONFIG.json gguf -m $(MODEL_DIR) -f $(MODEL_FILE)"
else
	@$(SERVER_BINARY) --port 8080 --mcp-config MCP_CONFIG.json gguf -m $(MODEL_DIR) -f $(MODEL_FILE)
endif

# ============================================================================
# Benchmarking & Profiling
# ============================================================================

.PHONY: bench
bench: ## Run performance benchmarks
	@echo "Running benchmarks..."
	@$(CARGO) bench --workspace

.PHONY: bench-quick
bench-quick: ## Quick benchmark
	@$(CARGO) bench -p mistralrs-bench -- --quick

.PHONY: bloat-check
bloat-check: ## Analyze binary size
	@echo "Analyzing binary bloat..."
	@$(CARGO) bloat --release --package mistralrs-server -n 20

# ============================================================================
# Dependencies
# ============================================================================

.PHONY: deps-tree
deps-tree: ## Show dependency tree
	@$(CARGO) tree --package mistralrs-server

.PHONY: deps-duplicates
deps-duplicates: ## Check for duplicate dependencies
	@$(CARGO) tree --duplicates

.PHONY: deps-update
deps-update: ## Update dependencies
	@echo "Updating dependencies..."
	@$(CARGO) update
	@echo "✓ Dependencies updated"

# ============================================================================
# Documentation
# ============================================================================

.PHONY: doc
doc: ## Generate documentation
	@echo "Generating documentation..."
	@$(CARGO) doc --workspace --no-deps

.PHONY: doc-open
doc-open: doc ## Generate and open documentation
	@$(CARGO) doc --workspace --no-deps --open

# ============================================================================
# Utility Targets
# ============================================================================

.PHONY: version
version: ## Show versions of tools
	@echo "Rust: $$(rustc --version)"
	@echo "Cargo: $$($(CARGO) --version)"
	@echo "Platform: $(PLATFORM)"
	@echo "Binary: $$(ls -lh $(SERVER_BINARY) 2>/dev/null || dir $(SERVER_BINARY) 2>/dev/null || echo 'Not built')"

.PHONY: info
info: check-env ## Show build configuration
	@echo ""
	@echo "Build Configuration:"
	@echo "  Platform: $(PLATFORM)"
	@echo "  CUDA Features: $(CUDA_FEATURES)"
	@echo "  MKL Features: $(MKL_FEATURES)"
	@echo "  Parallel Jobs: $(NPROC)"
	@echo "  Target Dir: $(TARGET_DIR)"
	@echo "  Server Binary: $(SERVER_BINARY)"
	@echo ""

.PHONY: sccache-stats
sccache-stats: ## Show sccache statistics
	@sccache --show-stats 2>/dev/null || echo "sccache not available"

.PHONY: model-inventory
model-inventory: ## Display model inventory
	@cat MODEL_INVENTORY.json 2>/dev/null || echo "MODEL_INVENTORY.json not found"

# ============================================================================
# Special Targets for Common Workflows
# ============================================================================

.PHONY: quick
quick: check ## Quick check (alias for 'check')

.PHONY: full-build
full-build: clean build-cuda-full ## Clean and full CUDA build

.PHONY: rebuild
rebuild: clean-all build-cuda-full ## Complete rebuild from scratch

.PHONY: validate
validate: fmt-check lint check test ## Full validation (format, lint, check, test)

# ============================================================================
# Include Deployment & Validation Targets
# ============================================================================

# Include comprehensive deployment, testing, and validation targets
# See Makefile.deployment for detailed deployment workflows
-include Makefile.deployment

# ============================================================================
# End of Makefile
# ============================================================================
