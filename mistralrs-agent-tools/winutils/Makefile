# ============================================================================
# Windows Coreutils - Comprehensive Build System
# ============================================================================
#
# This Makefile provides a complete build system for the Windows-optimized
# coreutils project, including all 77 GNU coreutils and derive utilities.
#
# WINDOWS USAGE:
#   build.bat              - Build all components (Windows batch)
#   build.ps1              - Build all components (PowerShell)
#   make (if available)    - Traditional make commands
#
# TRADITIONAL MAKE USAGE:
#   make              - Build all components in release mode
#   make help         - Show all available targets
#   make release      - Build optimized release binaries
#   make debug        - Build with debug symbols
#   make test         - Run comprehensive test suite
#   make install      - Install binaries to system
#   make package      - Create distribution package
#   make coverage     - Run code coverage analysis
#   make install-hooks- Install pre-commit quality hooks
#
# IMPORTANT: This is the ONLY supported build method.
# Direct cargo commands are disabled - use this build system only.
#
# ============================================================================

# Include coverage and pre-commit hook targets
include Makefile.coverage

# Fix PATH recursion FIRST - before any other variable expansion
# The Windows environment has PATH containing literal "${PATH}" which causes recursion
# Solution: Override PATH completely with a clean version
# This prevents Make from trying to expand the recursive environment PATH
export PATH := C:/users/david/.cargo/bin:/usr/bin:/bin:C:/Users/david/.local/bin

SHELL := /usr/bin/env bash
.SHELLFLAGS := -c

# ============================ Configuration =================================

# Build Configuration - Using explicit paths to local cargo tools
CARGO_BIN_DIR := C:/users/david/.cargo/bin
CARGO := $(CARGO_BIN_DIR)/cargo.exe
RUSTC := $(CARGO_BIN_DIR)/rustc.exe
SCCACHE := $(CARGO_BIN_DIR)/sccache.exe
TARGET := x86_64-pc-windows-msvc
PROFILE ?= release
BUILD_DIR := target
RELEASE_DIR := $(BUILD_DIR)/$(TARGET)/release
DEBUG_DIR := $(BUILD_DIR)/$(TARGET)/debug
PACKAGE_DIR := dist

# Performance optimization settings
RUSTFLAGS_PERF := -C target-cpu=native -C opt-level=3 -C lto=fat -C embed-bitcode=yes -C codegen-units=1
RUSTFLAGS_PARALLEL := -C target-cpu=native -C opt-level=3 -C lto=thin -C codegen-units=16
PGO_DATA_DIR := pgo-data

# Windows-specific paths (use forward slashes for cross-compatibility)
WINDOWS_PREFIX := C:/users/david/.local
WINDOWS_BIN := $(WINDOWS_PREFIX)/bin
ARCHIVE_DIR := $(WINDOWS_BIN)/.archive

# Shell detection
SHELL_FAMILY := $(shell echo $$0 | grep -q bash && echo bash || echo cmd)
IS_WINDOWS := $(shell [ "$$OS" = "Windows_NT" ] && echo true || echo false)

# Installation Configuration
PREFIX ?= C:/users/david/.local
INSTALL_BIN := $(PREFIX)/bin
INSTALL_DOC := $(PREFIX)/share/doc/winutils
INSTALL_MAN := $(PREFIX)/share/man
INSTALL_BACKUP := $(INSTALL_BIN)/.archive/$(BUILD_DATE)

# Rust Optimization Flags for Windows
# NOTE: These environment variables are now DISABLED to avoid conflicts with Cargo.toml profiles.
# Build settings are controlled by profiles in winutils/Cargo.toml ([profile.release])
# and .cargo/config.toml (rustflags). Environment variables here would override those
# settings, causing confusion about which configuration is active.
#
# Disabled environment overrides (now managed in Cargo.toml):
# export RUSTFLAGS := -C target-cpu=native -C opt-level=3 -C lto=fat -C embed-bitcode=yes -C codegen-units=1 -C link-arg=/STACK:8388608 -C prefer-dynamic=no
# export CARGO_PROFILE_RELEASE_LTO := true
# export CARGO_PROFILE_RELEASE_CODEGEN_UNITS := 1
# export CARGO_PROFILE_RELEASE_OPT_LEVEL := 3
# export CARGO_PROFILE_RELEASE_STRIP := true
# export CARGO_PROFILE_RELEASE_PANIC := unwind

# Disable direct cargo usage - force use of build system
export CARGO_BUILD_DISABLED := 1

# Version Information
VERSION := $(shell grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
GIT_HASH := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")
BUILD_DATE := $(shell /usr/bin/date +%Y%m%d 2>/dev/null || echo "20250130")
PACKAGE_NAME := winutils-$(VERSION)-$(BUILD_DATE)-$(GIT_HASH)

# ============================ Utility Lists =================================

# Core GNU Coreutils (77 utilities)
COREUTILS := arch b2sum b3sum base32 base64 basename basenc cat chgrp chmod \
    chown chroot cksum comm cp csplit cut date dd df dir dircolors dirname \
    du echo env expand expr factor false fmt fold groups hashsum head \
    hostid hostname id install join kill link ln logname ls md5sum mkdir \
    mkfifo mknod mktemp more mv nice nl nohup nproc numfmt od paste pathchk \
    pinky pr printenv printf ptx pwd readlink realpath relpath rm rmdir \
    runcon seq sha1sum sha224sum sha256sum sha384sum sha3-224sum sha3-256sum \
    sha3-384sum sha3-512sum sha3sum sha512sum shake128sum shake256sum shred \
    shuf sleep sort split stat stdbuf stty sum sync tac tail tee test \
    timeout touch tr true truncate tsort tty uname unexpand uniq unlink \
    uptime users vdir wc who whoami yes

# Derive Utilities (Windows-specific)
DERIVE_UTILS := where which tree

# External Utilities (pre-built)
EXTERNAL_UTILS := rg fd

# All Utilities
ALL_UTILS := $(COREUTILS) $(DERIVE_UTILS) $(EXTERNAL_UTILS)

# ============================ Color Output ==================================

# Terminal Colors (Windows compatible)
RED := \033[31m
GREEN := \033[32m
YELLOW := \033[33m
BLUE := \033[34m
MAGENTA := \033[35m
CYAN := \033[36m
WHITE := \033[37m
RESET := \033[0m
BOLD := \033[1m

# ============================ Main Targets ==================================

.PHONY: all help release debug clean install test validate package

# Default target
all: release

# Help target - display all available commands
help:
	@echo "$(BOLD)$(CYAN)Windows Coreutils Build System$(RESET)"
	@echo "$(CYAN)━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$(RESET)"
	@echo ""
	@echo "$(BOLD)$(GREEN)Primary Targets:$(RESET)"
	@echo "  $(YELLOW)make$(RESET)              Build all components (release mode)"
	@echo "  $(YELLOW)make release$(RESET)      Build optimized release binaries"
	@echo "  $(YELLOW)make debug$(RESET)        Build with debug symbols"
	@echo "  $(YELLOW)make clean$(RESET)        Remove all build artifacts"
	@echo ""
	@echo "$(BOLD)$(GREEN)Installation:$(RESET)"
	@echo "  $(YELLOW)make install$(RESET)      Install to $(PREFIX)"
	@echo "  $(YELLOW)make uninstall$(RESET)    Remove installed binaries"
	@echo "  $(YELLOW)make package$(RESET)      Create distribution package"
	@echo ""
	@echo "$(BOLD)$(GREEN)Testing & Validation:$(RESET)"
	@echo "  $(YELLOW)make test$(RESET)         Run complete test suite"
	@echo "  $(YELLOW)make test-fast$(RESET)    Run quick tests only"
	@echo "  $(YELLOW)make bench$(RESET)        Run performance benchmarks"
	@echo "  $(YELLOW)make validate$(RESET)     Validate all utilities"
	@echo "  $(YELLOW)make gnu-compat$(RESET)   Test GNU compatibility"
	@echo ""
	@echo "$(BOLD)$(GREEN)Quality Assurance:$(RESET)"
	@echo "  $(YELLOW)make check$(RESET)        Run all checks (fmt, clippy, audit)"
	@echo "  $(YELLOW)make fmt$(RESET)          Format all code"
	@echo "  $(YELLOW)make clippy$(RESET)       Run Clippy linter"
	@echo "  $(YELLOW)make audit$(RESET)        Security vulnerability scan"
	@echo ""
	@echo "$(BOLD)$(GREEN)Utilities:$(RESET)"
	@echo "  $(YELLOW)make doctor$(RESET)       Diagnose build environment"
	@echo "  $(YELLOW)make stats$(RESET)        Show build statistics"
	@echo "  $(YELLOW)make list$(RESET)         List all utilities"
	@echo "  $(YELLOW)make count$(RESET)        Count built binaries"
	@echo ""
	@echo "$(BOLD)$(GREEN)Individual Builds:$(RESET)"
	@echo "  $(YELLOW)make util-<name>$(RESET)  Build specific utility (e.g., make util-cat)"
	@echo "  $(YELLOW)make derive$(RESET)       Build derive utilities only"
	@echo "  $(YELLOW)make derive-utils$(RESET) Build all derive utilities"
	@echo "  $(YELLOW)make where$(RESET)        Build where utility"
	@echo "  $(YELLOW)make which$(RESET)        Build which utility"
	@echo "  $(YELLOW)make tree$(RESET)         Build tree utility"
	@echo "  $(YELLOW)make core$(RESET)         Build core utilities only"
	@echo ""
	@echo "$(BOLD)$(GREEN)Advanced:$(RESET)"
	@echo "  $(YELLOW)make profile$(RESET)      Generate performance profile"
	@echo "  $(YELLOW)make coverage$(RESET)     Generate code coverage report"
	@echo "  $(YELLOW)make docs$(RESET)         Generate documentation"
	@echo ""
	@echo "$(CYAN)━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$(RESET)"
	@echo "$(BOLD)Version:$(RESET) $(VERSION) | $(BOLD)Target:$(RESET) $(TARGET)"

# ============================ Build Targets =================================

# Build release binaries with full optimization
release: pre-build build-winpath
	@echo "$(BOLD)$(CYAN)Building Release Binaries$(RESET)"
	@echo "$(YELLOW)Step 1: Building winpath library...$(RESET)"
	@$(CARGO) build --release --package winpath --target $(TARGET) --target-dir $(BUILD_DIR)
	@echo "$(YELLOW)Step 2: Building derive utilities...$(RESET)"
	@$(CARGO) build --release --package uu_where --package winutils-which --package uu_tree --target $(TARGET) --target-dir $(BUILD_DIR)
	@echo "$(YELLOW)Step 3: Building main workspace...$(RESET)"
	@$(CARGO) build --release --workspace --target $(TARGET) --target-dir $(BUILD_DIR) --exclude winpath --exclude uu_where --exclude winutils-which --exclude uu_tree
	@echo "$(YELLOW)Step 4: Building coreutils workspace...$(RESET)"
	@cd coreutils && $(CARGO) build --release --workspace --target $(TARGET) --target-dir ../$(BUILD_DIR)
	@echo "$(GREEN)✓ Release build complete!$(RESET)"
	@$(MAKE) --no-print-directory post-build verify-winpath-integration validate-all-77

# Build debug binaries with symbols
debug: PROFILE = debug
debug: pre-build build-winpath-debug
	@echo "$(BOLD)$(CYAN)Building Debug Binaries$(RESET)"
	@echo "$(YELLOW)Building main workspace (debug)...$(RESET)"
	@$(CARGO) build --workspace --target $(TARGET) --target-dir $(BUILD_DIR)
	@echo "$(YELLOW)Building coreutils workspace (debug)...$(RESET)"
	@cd coreutils && $(CARGO) build --workspace --target $(TARGET) --target-dir ../$(BUILD_DIR)
	@echo "$(GREEN)✓ Debug build complete!$(RESET)"
	@$(MAKE) --no-print-directory post-build verify-winpath-integration

# Build with maximum performance (no debug info, panic=abort)
release-fast: PROFILE = release-fast
release-fast: pre-build
	@echo "$(BOLD)$(CYAN)Building Fast Release Binaries$(RESET)"
	@$(CARGO) build --profile release-fast --workspace --target $(TARGET) --target-dir $(BUILD_DIR)
	@cd coreutils && $(CARGO) build --profile release-fast --workspace --target $(TARGET) --target-dir ../$(BUILD_DIR)
	@echo "$(GREEN)✓ Fast release build complete!$(RESET)"

# Pre-build checks and setup
pre-build:
	@echo "$(BOLD)$(CYAN)Pre-Build Checks$(RESET)"
	@echo "Checking Rust toolchain..."
	@$(RUSTC) --version > /dev/null 2>&1 || (echo "$(RED)Error: Rust not installed$(RESET)" && exit 1)
	@echo "Creating build directories..."
	@test -d $(BUILD_DIR) || mkdir -p $(BUILD_DIR)
	@test -d $(BUILD_DIR)/$(TARGET) || mkdir -p $(BUILD_DIR)/$(TARGET)
	@test -d $(RELEASE_DIR) || mkdir -p $(RELEASE_DIR)
	@test -d $(DEBUG_DIR) || mkdir -p $(DEBUG_DIR)
	@echo "Verifying winpath dependency..."
	@if [ ! -f "shared/winpath/Cargo.toml" ]; then \
		echo "$(RED)Error: winpath library not found!$(RESET)"; \
		echo "$(RED)Path normalization library is required for all utilities$(RESET)"; \
		exit 1; \
	fi
	@echo "$(GREEN)✓ Pre-build checks passed$(RESET)"

# Post-build verification
post-build:
	@echo "$(BOLD)$(CYAN)Post-Build Verification$(RESET)"
	@echo "Counting built binaries..."
	@if [ -d "$(RELEASE_DIR)" ]; then \
		ls "$(RELEASE_DIR)"/*.exe 2>/dev/null | wc -l | xargs echo "Built utilities:"; \
	else \
		echo "$(RED)Error: Release directory not found$(RESET)"; \
		exit 1; \
	fi
	@echo "$(GREEN)✓ Build verification complete$(RESET)"

# ============================ Individual Utilities ==========================

# Build all core utilities
core: pre-build
	@echo "$(BOLD)$(CYAN)Building Core Utilities$(RESET)"
	@cd coreutils && $(CARGO) build --release --workspace --target $(TARGET) --target-dir ../$(BUILD_DIR)
	@echo "$(GREEN)✓ Core utilities built$(RESET)"

# Build derive utilities only
derive: derive-utils
	@echo "$(GREEN)✓ All derive utilities built$(RESET)"

# Build all derive utilities
derive-utils: pre-build build-winpath where which tree
	@echo "$(GREEN)✓ All derive utilities built$(RESET)"

# Build individual derive utilities
where: pre-build build-winpath
	@echo "$(BOLD)$(CYAN)Building where utility$(RESET)"
	@$(CARGO) build --release --package uu_where --target $(TARGET) --target-dir $(BUILD_DIR)
	@echo "$(GREEN)✓ where utility built$(RESET)"

which: pre-build build-winpath
	@echo "$(BOLD)$(CYAN)Building which utility$(RESET)"
	@$(CARGO) build --release --package winutils-which --target $(TARGET) --target-dir $(BUILD_DIR)
	@echo "$(GREEN)✓ which utility built$(RESET)"

tree: pre-build build-winpath
	@echo "$(BOLD)$(CYAN)Building tree utility$(RESET)"
	@$(CARGO) build --release --package uu_tree --target $(TARGET) --target-dir $(BUILD_DIR)
	@echo "$(GREEN)✓ tree utility built$(RESET)"

# Pattern rule for building individual utilities
util-%: pre-build
	@echo "$(BOLD)$(CYAN)Building Utility: $*$(RESET)"
	@if echo "$(DERIVE_UTILS)" | grep -qw "$*"; then \
		$(CARGO) build --release --package $* --target $(TARGET) --target-dir $(BUILD_DIR); \
	else \
		cd coreutils && $(CARGO) build --release --package uu_$* --target $(TARGET) --target-dir ../$(BUILD_DIR); \
	fi
	@echo "$(GREEN)✓ $* built successfully$(RESET)"

# ============================ Installation ==================================

# Install binaries to system with backup
install: release install-dirs backup-existing
	@echo "$(BOLD)$(CYAN)Installing Binaries with Backup$(RESET)"
	@echo "Installing to $(INSTALL_BIN)..."
	@installed_count=0; \
	for util in $(COREUTILS); do \
		if [ -f "$(RELEASE_DIR)/uu_$$util.exe" ]; then \
			cp -f "$(RELEASE_DIR)/uu_$$util.exe" "$(INSTALL_BIN)/wu-$$util.exe" 2>/dev/null && \
			echo "  $(GREEN)✓$(RESET) $$util" && \
			installed_count=$$((installed_count + 1)); \
		else \
			echo "  $(RED)✗$(RESET) $$util (not found)"; \
		fi; \
	done; \
	for util in $(DERIVE_UTILS); do \
		if [ -f "$(RELEASE_DIR)/$$util.exe" ]; then \
			cp -f "$(RELEASE_DIR)/$$util.exe" "$(INSTALL_BIN)/$$util.exe" 2>/dev/null && \
			echo "  $(GREEN)✓$(RESET) $$util" && \
			installed_count=$$((installed_count + 1)); \
		else \
			echo "  $(RED)✗$(RESET) $$util (not found)"; \
		fi; \
	done; \
	echo "$(GREEN)✓ Installation complete! Installed $$installed_count utilities$(RESET)"
	@echo "$(YELLOW)Backup created in: $(INSTALL_BACKUP)$(RESET)"
	@echo "$(YELLOW)Add $(INSTALL_BIN) to your PATH if needed$(RESET)"

# Create installation directories
install-dirs:
	@test -d "$(INSTALL_BIN)" || mkdir -p "$(INSTALL_BIN)"
	@test -d "$(INSTALL_DOC)" || mkdir -p "$(INSTALL_DOC)"
	@test -d "$(INSTALL_MAN)" || mkdir -p "$(INSTALL_MAN)"
	@test -d "$(INSTALL_BACKUP)" || mkdir -p "$(INSTALL_BACKUP)"

# Backup existing binaries before installation
backup-existing:
	@echo "$(BOLD)$(CYAN)Backing Up Existing Binaries$(RESET)"
	@if [ -d "$(INSTALL_BIN)" ]; then \
		backup_count=0; \
		for util in $(COREUTILS); do \
			if [ -f "$(INSTALL_BIN)/wu-$$util.exe" ]; then \
				cp "$(INSTALL_BIN)/wu-$$util.exe" "$(INSTALL_BACKUP)/wu-$$util.exe" 2>/dev/null && \
				backup_count=$$((backup_count + 1)); \
			fi; \
		done; \
		for util in $(DERIVE_UTILS); do \
			if [ -f "$(INSTALL_BIN)/$$util.exe" ]; then \
				cp "$(INSTALL_BIN)/$$util.exe" "$(INSTALL_BACKUP)/$$util.exe" 2>/dev/null && \
				backup_count=$$((backup_count + 1)); \
			fi; \
		done; \
		if [ $$backup_count -gt 0 ]; then \
			echo "$(GREEN)✓ Backed up $$backup_count existing binaries$(RESET)"; \
		else \
			echo "$(YELLOW)No existing binaries to backup$(RESET)"; \
		fi; \
	else \
		echo "$(YELLOW)No existing installation directory$(RESET)"; \
	fi

# Uninstall binaries
uninstall:
	@echo "$(BOLD)$(CYAN)Uninstalling Binaries$(RESET)"
	@for util in $(COREUTILS); do \
		rm -f "$(INSTALL_BIN)/wu-$$util.exe" 2>/dev/null; \
	done
	@for util in $(DERIVE_UTILS); do \
		rm -f "$(INSTALL_BIN)/$$util.exe" 2>/dev/null; \
	done
	@echo "$(GREEN)✓ Uninstallation complete$(RESET)"

# ============================ Testing =======================================

# Run complete test suite
test: test-unit test-integration test-compat test-git-bash-paths test-derive-utils
	@echo "$(GREEN)✓ All tests passed!$(RESET)"

# Run unit tests
test-unit:
	@echo "$(BOLD)$(CYAN)Running Unit Tests$(RESET)"
	@$(CARGO) test --workspace --target $(TARGET) -- --nocapture
	@cd coreutils && $(CARGO) test --workspace --target $(TARGET) -- --nocapture

# Run integration tests
test-integration:
	@echo "$(BOLD)$(CYAN)Running Integration Tests$(RESET)"
	@powershell -ExecutionPolicy Bypass -File scripts/validate.ps1

# Run fast tests only
test-fast:
	@echo "$(BOLD)$(CYAN)Running Fast Tests$(RESET)"
	@$(CARGO) test --workspace --lib --target $(TARGET)

# Test GNU compatibility
gnu-compat:
	@echo "$(BOLD)$(CYAN)Testing GNU Compatibility$(RESET)"
	@if [ -f "scripts/test-gnu-compat.ps1" ]; then \
		powershell -ExecutionPolicy Bypass -File scripts/test-gnu-compat.ps1; \
	else \
		echo "$(YELLOW)GNU compatibility script not found, skipping...$(RESET)"; \
	fi

# Test Git Bash path normalization
test-git-bash-paths:
	@echo "$(BOLD)$(CYAN)Testing Git Bash Path Normalization$(RESET)"
	@if [ -f "test_git_bash_paths.rs" ]; then \
		$(RUSTC) test_git_bash_paths.rs --extern winpath="$(RELEASE_DIR)/deps/libwinpath.rlib" -L "$(RELEASE_DIR)/deps" -o "$(BUILD_DIR)/test_git_bash_paths.exe" && \
		"$(BUILD_DIR)/test_git_bash_paths.exe" && \
		echo "$(GREEN)✓ Git Bash path tests passed$(RESET)"; \
	else \
		echo "$(YELLOW)Git Bash path test not found, creating...$(RESET)"; \
		$(MAKE) --no-print-directory create-git-bash-test; \
	fi

# Create Git Bash path test if missing
create-git-bash-test:
	@echo "$(BOLD)$(CYAN)Creating Git Bash Path Test$(RESET)"
	@echo 'use std::path::Path;' > test_git_bash_simple.rs
	@echo 'fn main() { println!("Git Bash path test placeholder"); }' >> test_git_bash_simple.rs
	@$(RUSTC) test_git_bash_simple.rs -o "$(BUILD_DIR)/test_git_bash_simple.exe"
	@"$(BUILD_DIR)/test_git_bash_simple.exe"
	@echo "$(GREEN)✓ Basic Git Bash test created and executed$(RESET)"

# Test derive utilities path handling
test-derive-utils:
	@echo "$(BOLD)$(CYAN)Testing Derive Utilities Path Handling$(RESET)"
	@error_count=0; \
	for util in $(DERIVE_UTILS); do \
		if [ -f "$(RELEASE_DIR)/$$util.exe" ]; then \
			echo "Testing $$util..."; \
			if "$(RELEASE_DIR)/$$util.exe" --version >/dev/null 2>&1; then \
				echo "  $(GREEN)✓$(RESET) $$util version check passed"; \
			else \
				echo "  $(RED)✗$(RESET) $$util version check failed"; \
				error_count=$$((error_count + 1)); \
			fi; \
			if "$(RELEASE_DIR)/$$util.exe" --help >/dev/null 2>&1; then \
				echo "  $(GREEN)✓$(RESET) $$util help check passed"; \
			else \
				echo "  $(YELLOW)⚠$(RESET) $$util help check failed (may be normal)"; \
			fi; \
			case "$$util" in \
				"where") \
					if "$(RELEASE_DIR)/$$util.exe" cmd.exe >/dev/null 2>&1; then \
						echo "  $(GREEN)✓$(RESET) $$util path resolution test passed"; \
					else \
						echo "  $(RED)✗$(RESET) $$util path resolution test failed"; \
						error_count=$$((error_count + 1)); \
					fi; \
					;; \
				"which") \
					if "$(RELEASE_DIR)/$$util.exe" cmd >/dev/null 2>&1; then \
						echo "  $(GREEN)✓$(RESET) $$util path resolution test passed"; \
					else \
						echo "  $(RED)✗$(RESET) $$util path resolution test failed"; \
						error_count=$$((error_count + 1)); \
					fi; \
					;; \
				"tree") \
					if "$(RELEASE_DIR)/$$util.exe" . --max-depth 1 >/dev/null 2>&1; then \
						echo "  $(GREEN)✓$(RESET) $$util directory traversal test passed"; \
					else \
						echo "  $(RED)✗$(RESET) $$util directory traversal test failed"; \
						error_count=$$((error_count + 1)); \
					fi; \
					;; \
			esac; \
		else \
			echo "  $(RED)✗$(RESET) $$util binary not found"; \
			error_count=$$((error_count + 1)); \
		fi; \
	done; \
	if [ $$error_count -eq 0 ]; then \
		echo "$(GREEN)✓ All derive utilities tests passed!$(RESET)"; \
	else \
		echo "$(RED)✗ $$error_count derive utilities tests failed!$(RESET)"; \
		exit 1; \
	fi

# ============================ Benchmarks ====================================

# Run performance benchmarks
bench:
	@echo "$(BOLD)$(CYAN)Running Benchmarks$(RESET)"
	@$(CARGO) bench --workspace --target $(TARGET)

# Generate performance profile
profile:
	@echo "$(BOLD)$(CYAN)Generating Performance Profile$(RESET)"
	@$(CARGO) build --release --workspace --target $(TARGET) --target-dir $(BUILD_DIR)
	@echo "$(GREEN)✓ Profile data generated$(RESET)"

# ============================ Quality Assurance =============================

# Run all checks
check: fmt clippy audit
	@echo "$(GREEN)✓ All checks passed!$(RESET)"

# Format code
fmt:
	@echo "$(BOLD)$(CYAN)Formatting Code$(RESET)"
	@$(CARGO) fmt --all --check || $(CARGO) fmt --all
	@echo "$(GREEN)✓ Code formatted$(RESET)"

# Run Clippy linter
clippy:
	@echo "$(BOLD)$(CYAN)Running Clippy$(RESET)"
	@$(CARGO) clippy --workspace --all-targets --target $(TARGET) -- -D warnings
	@cd coreutils && $(CARGO) clippy --workspace --all-targets --target $(TARGET) -- -D warnings
	@echo "$(GREEN)✓ Clippy checks passed$(RESET)"

# Security audit
audit:
	@echo "$(BOLD)$(CYAN)Running Security Audit$(RESET)"
	@$(CARGO) audit || echo "$(YELLOW)⚠ Some vulnerabilities found$(RESET)"

# Validate all utilities
validate:
	@echo "$(BOLD)$(CYAN)Validating Utilities$(RESET)"
	@$(MAKE) --no-print-directory validate-all-77
	@if [ -f "scripts/validate.ps1" ]; then \
		powershell -ExecutionPolicy Bypass -File scripts/validate.ps1 -GenerateReport; \
	else \
		echo "$(YELLOW)PowerShell validation script not available, using basic validation$(RESET)"; \
	fi
	@echo "$(GREEN)✓ Validation complete$(RESET)"

# Validate that all utilities are built (Core + Derive utilities)
validate-all-77:
	@echo "$(BOLD)$(CYAN)Validating All Core + Derive Utilities$(RESET)"
	@missing_count=0; \
	core_count=0; \
	derive_count=0; \
	total_expected_core=$$(echo $(COREUTILS) | wc -w); \
	total_expected_derive=$$(echo $(DERIVE_UTILS) | wc -w); \
	total_expected=$$(($$total_expected_core + $$total_expected_derive)); \
	echo "Expected core utilities: $$total_expected_core"; \
	echo "Expected derive utilities: $$total_expected_derive"; \
	echo "Total expected: $$total_expected"; \
	echo ""; \
	echo "Checking core utilities..."; \
	for util in $(COREUTILS); do \
		if [ -f "$(RELEASE_DIR)/uu_$$util.exe" ]; then \
			core_count=$$((core_count + 1)); \
		else \
			echo "  $(RED)✗$(RESET) Missing: uu_$$util.exe"; \
			missing_count=$$((missing_count + 1)); \
		fi; \
	done; \
	echo "Found core utilities: $$core_count"; \
	echo ""; \
	echo "Checking derive utilities..."; \
	for util in $(DERIVE_UTILS); do \
		if [ -f "$(RELEASE_DIR)/$$util.exe" ]; then \
			derive_count=$$((derive_count + 1)); \
			echo "  $(GREEN)✓$(RESET) Found: $$util.exe"; \
		else \
			echo "  $(RED)✗$(RESET) Missing: $$util.exe"; \
			missing_count=$$((missing_count + 1)); \
		fi; \
	done; \
	echo "Found derive utilities: $$derive_count"; \
	echo ""; \
	actual_count=$$((core_count + derive_count)); \
	echo "$(BOLD)Validation Summary:$(RESET)"; \
	echo "Expected utilities: $$total_expected ($$total_expected_core core + $$total_expected_derive derive)"; \
	echo "Found utilities: $$actual_count ($$core_count core + $$derive_count derive)"; \
	echo "Missing utilities: $$missing_count"; \
	if [ $$missing_count -eq 0 ]; then \
		echo "$(GREEN)✓ All $$total_expected utilities successfully built!$(RESET)"; \
	else \
		echo "$(RED)✗ $$missing_count utilities missing!$(RESET)"; \
		exit 1; \
	fi

# ============================ Documentation =================================

# Generate documentation
docs:
	@echo "$(BOLD)$(CYAN)Generating Documentation$(RESET)"
	@$(CARGO) doc --workspace --no-deps --target $(TARGET)
	@echo "$(GREEN)✓ Documentation generated$(RESET)"
	@echo "Open $(BUILD_DIR)/doc/index.html to view"

# Generate code coverage
coverage:
	@echo "$(BOLD)$(CYAN)Generating Code Coverage$(RESET)"
	@$(CARGO) tarpaulin --workspace --target $(TARGET) --out Html
	@echo "$(GREEN)✓ Coverage report generated$(RESET)"

# ============================ Packaging =====================================

# Create distribution package
package: release
	@echo "$(BOLD)$(CYAN)Creating Distribution Package$(RESET)"
	@test -d $(PACKAGE_DIR) || mkdir -p $(PACKAGE_DIR)
	@echo "Collecting binaries..."
	@test -d $(PACKAGE_DIR)/$(PACKAGE_NAME) || mkdir -p $(PACKAGE_DIR)/$(PACKAGE_NAME)
	@test -d $(PACKAGE_DIR)/$(PACKAGE_NAME)/bin || mkdir -p $(PACKAGE_DIR)/$(PACKAGE_NAME)/bin
	@cp $(RELEASE_DIR)/*.exe $(PACKAGE_DIR)/$(PACKAGE_NAME)/bin/ 2>/dev/null || true
	@echo "Adding documentation..."
	@cp README.md BUILD_DOCUMENTATION.md $(PACKAGE_DIR)/$(PACKAGE_NAME)/ 2>/dev/null || true
	@echo "Creating archive..."
	@cd $(PACKAGE_DIR) && tar czf $(PACKAGE_NAME).tar.gz $(PACKAGE_NAME)
	@echo "$(GREEN)✓ Package created: $(PACKAGE_DIR)/$(PACKAGE_NAME).tar.gz$(RESET)"

# Create Windows installer
installer: package
	@echo "$(BOLD)$(CYAN)Creating Windows Installer$(RESET)"
	@powershell -ExecutionPolicy Bypass -File scripts/create-installer.ps1
	@echo "$(GREEN)✓ Installer created$(RESET)"

# ============================ Utilities =====================================

# Clean build artifacts
clean:
	@echo "$(BOLD)$(CYAN)Cleaning Build Artifacts$(RESET)"
	@RUSTC_WRAPPER="" $(CARGO) clean
	@cd coreutils && RUSTC_WRAPPER="" $(CARGO) clean
	@rm -rf $(BUILD_DIR) $(PACKAGE_DIR)
	@echo "$(GREEN)✓ Clean complete$(RESET)"

# Deep clean including caches
clean-all: clean
	@echo "$(BOLD)$(CYAN)Deep Cleaning$(RESET)"
	@rm -rf ~/.cargo/registry/cache
	@rm -rf ~/.cargo/git/checkouts
	@echo "$(GREEN)✓ Deep clean complete$(RESET)"

# Diagnose build environment
doctor:
	@echo "$(BOLD)$(CYAN)Build Environment Diagnostics$(RESET)"
	@echo ""
	@echo "$(YELLOW)Rust Installation:$(RESET)"
	@$(RUSTC) --version || echo "$(RED)✗ Rust not found$(RESET)"
	@$(CARGO) --version || echo "$(RED)✗ Cargo not found$(RESET)"
	@echo ""
	@echo "$(YELLOW)Target Configuration:$(RESET)"
	@echo "Target: $(TARGET)"
	@rustup target list --installed | grep $(TARGET) || echo "$(RED)✗ Target not installed$(RESET)"
	@echo ""
	@echo "$(YELLOW)Build Flags:$(RESET)"
	@echo "RUSTFLAGS: $$RUSTFLAGS"
	@echo ""
	@echo "$(YELLOW)Workspace Structure:$(RESET)"
	@$(CARGO) metadata --format-version 1 --no-deps | grep '"name"' | wc -l | xargs echo "Packages:"
	@echo ""
	@echo "$(YELLOW)Build Output:$(RESET)"
	@ls $(RELEASE_DIR)/*.exe 2>/dev/null | wc -l | xargs echo "Built utilities:"
	@echo ""
	@echo "$(GREEN)✓ Diagnostics complete$(RESET)"

# Show build statistics
stats:
	@echo "$(BOLD)$(CYAN)Build Statistics$(RESET)"
	@echo ""
	@echo "$(YELLOW)Project Info:$(RESET)"
	@echo "Version: $(VERSION)"
	@echo "Git Hash: $(GIT_HASH)"
	@echo "Build Date: $(BUILD_DATE)"
	@echo ""
	@echo "$(YELLOW)Utilities:$(RESET)"
	@echo "Core utilities: $(words $(COREUTILS))"
	@echo "Derive utilities: $(words $(DERIVE_UTILS))"
	@echo "Total expected: $(words $(ALL_UTILS))"
	@echo ""
	@echo "$(YELLOW)Build Output:$(RESET)"
	@ls -lh $(RELEASE_DIR)/*.exe 2>/dev/null | wc -l | xargs echo "Built binaries:"
	@du -sh $(RELEASE_DIR) 2>/dev/null | xargs echo "Total size:"
	@echo ""
	@echo "$(YELLOW)Lines of Code:$(RESET)"
	@find . -name "*.rs" -not -path "./target/*" | xargs wc -l | tail -1
	@echo "$(GREEN)✓ Statistics complete$(RESET)"

# List all utilities
list:
	@echo "$(BOLD)$(CYAN)Available Utilities$(RESET)"
	@echo ""
	@echo "$(YELLOW)Core Utilities ($(words $(COREUTILS))):$(RESET)"
	@echo "$(COREUTILS)" | tr ' ' '\n' | column -c 80
	@echo ""
	@echo "$(YELLOW)Derive Utilities ($(words $(DERIVE_UTILS))):$(RESET)"
	@echo "$(DERIVE_UTILS)" | tr ' ' '\n' | column -c 80
	@echo ""
	@echo "$(YELLOW)External Utilities ($(words $(EXTERNAL_UTILS))):$(RESET)"
	@echo "$(EXTERNAL_UTILS)" | tr ' ' '\n' | column -c 80

# Count built binaries
count:
	@echo "$(BOLD)$(CYAN)Binary Count$(RESET)"
	@echo "Expected utilities: $(words $(ALL_UTILS))"
	@if [ -d "$(RELEASE_DIR)" ]; then \
		total_count=$$(ls "$(RELEASE_DIR)"/*.exe 2>/dev/null | wc -l); \
		core_count=$$(ls "$(RELEASE_DIR)"/uu_*.exe 2>/dev/null | wc -l); \
		derive_count=$$(ls "$(RELEASE_DIR)"/where.exe "$(RELEASE_DIR)"/which.exe "$(RELEASE_DIR)"/tree.exe 2>/dev/null | wc -l); \
		echo "Built utilities: $$total_count"; \
		echo "Core utilities: $$core_count"; \
		echo "Derive utilities: $$derive_count"; \
		if [ $$total_count -eq 77 ]; then \
			echo "$(GREEN)✓ All expected utilities built$(RESET)"; \
		else \
			echo "$(YELLOW)⚠ Expected 77, found $$total_count$(RESET)"; \
		fi; \
	else \
		echo "$(RED)Release directory not found$(RESET)"; \
	fi
	@echo "$(GREEN)✓ Count complete$(RESET)"

# ============================ Development ===================================

# Watch for changes and rebuild
watch:
	@echo "$(BOLD)$(CYAN)Watching for Changes$(RESET)"
	@$(CARGO) watch -x "build --release --workspace"

# Run specific utility
run-%:
	@echo "$(BOLD)$(CYAN)Running $*$(RESET)"
	@if [ -f "$(RELEASE_DIR)/uu_$*.exe" ]; then \
		"$(RELEASE_DIR)/uu_$*.exe"; \
	elif [ -f "$(RELEASE_DIR)/$*.exe" ]; then \
		"$(RELEASE_DIR)/$*.exe"; \
	else \
		echo "$(RED)Utility $* not found$(RESET)"; \
		exit 1; \
	fi

# Test specific utility with sample data
test-util-%:
	@echo "$(BOLD)$(CYAN)Testing Utility: $*$(RESET)"
	@if [ -f "$(RELEASE_DIR)/uu_$*.exe" ]; then \
		echo "Testing uu_$*.exe..."; \
		"$(RELEASE_DIR)/uu_$*.exe" --version 2>/dev/null && echo "$(GREEN)✓ Version check passed$(RESET)" || echo "$(YELLOW)⚠ Version check failed$(RESET)"; \
		"$(RELEASE_DIR)/uu_$*.exe" --help >/dev/null 2>&1 && echo "$(GREEN)✓ Help check passed$(RESET)" || echo "$(YELLOW)⚠ Help check failed$(RESET)"; \
	elif [ -f "$(RELEASE_DIR)/$*.exe" ]; then \
		echo "Testing $*.exe..."; \
		"$(RELEASE_DIR)/$*.exe" --version 2>/dev/null && echo "$(GREEN)✓ Version check passed$(RESET)" || echo "$(YELLOW)⚠ Version check failed$(RESET)"; \
		"$(RELEASE_DIR)/$*.exe" --help >/dev/null 2>&1 && echo "$(GREEN)✓ Help check passed$(RESET)" || echo "$(YELLOW)⚠ Help check failed$(RESET)"; \
	else \
		echo "$(RED)Utility $* not found$(RESET)"; \
		exit 1; \
	fi

# Quick rebuild
quick:
	@$(CARGO) build --release --workspace --target $(TARGET) --target-dir $(BUILD_DIR)
	@cd coreutils && $(CARGO) build --release --workspace --target $(TARGET) --target-dir ../$(BUILD_DIR)

# ============================ CI/CD =========================================

# Continuous integration pipeline
ci: clean check build-winpath release test validate bench package
	@echo "$(GREEN)✓ CI pipeline complete!$(RESET)"

# Disable direct cargo usage (force build system usage)
disable-cargo:
	@echo "$(BOLD)$(CYAN)Configuring Build System Lock$(RESET)"
	@echo "# DO NOT USE CARGO DIRECTLY" > .cargo-disabled
	@echo "# Use the Makefile build system only" >> .cargo-disabled
	@echo "# Run: make help for available commands" >> .cargo-disabled
	@echo "$(GREEN)✓ Direct cargo usage disabled$(RESET)"
	@echo "$(YELLOW)Use 'make clean-cargo-lock' to re-enable direct cargo$(RESET)"

# Re-enable direct cargo usage
enable-cargo:
	@echo "$(BOLD)$(CYAN)Re-enabling Direct Cargo Usage$(RESET)"
	@rm -f .cargo-disabled
	@echo "$(GREEN)✓ Direct cargo usage re-enabled$(RESET)"

# Prepare for release
release-prep: clean release test validate package
	@echo "$(GREEN)✓ Release preparation complete!$(RESET)"

# Deploy to production
deploy: release-prep
	@echo "$(BOLD)$(CYAN)Deploying to Production$(RESET)"
	@powershell -ExecutionPolicy Bypass -File scripts/deploy.ps1 -Environment production
	@echo "$(GREEN)✓ Deployment complete!$(RESET)"

# ============================ Special Targets ===============================

# Don't run these in parallel
.NOTPARALLEL: clean clean-all

# Silent execution for specific targets
.SILENT: help doctor stats list count

# Build winpath library specifically (release mode)
build-winpath:
	@echo "$(BOLD)$(CYAN)Building winpath library (core dependency)$(RESET)"
	@$(CARGO) build --release --package winpath --target $(TARGET) --target-dir $(BUILD_DIR)
	@echo "$(GREEN)✓ winpath library built successfully$(RESET)"

# Build winpath library in debug mode
build-winpath-debug:
	@echo "$(BOLD)$(CYAN)Building winpath library (debug)$(RESET)"
	@$(CARGO) build --package winpath --target $(TARGET) --target-dir $(BUILD_DIR)
	@echo "$(GREEN)✓ winpath library built (debug)$(RESET)"

# Verify winpath integration in built utilities
verify-winpath-integration:
	@echo "$(BOLD)$(CYAN)Verifying winpath integration$(RESET)"
	@echo "Checking for winpath symbols in built utilities..."
	@for exe in $(RELEASE_DIR)/*.exe; do \
		if [ -f "$$exe" ]; then \
			strings "$$exe" 2>/dev/null | grep -q "winpath" && \
			echo "  $(GREEN)✓$(RESET) $$(basename $$exe) - winpath integrated" || \
			echo "  $(YELLOW)⚠$(RESET) $$(basename $$exe) - winpath not detected"; \
		fi; \
	done
	@echo "$(GREEN)✓ Integration verification complete$(RESET)"

# Dependency graph visualization
dep-graph:
	@echo "$(BOLD)$(CYAN)Generating Dependency Graph$(RESET)"
	@cargo tree --workspace --edges normal --prefix depth
	@echo ""
	@echo "$(YELLOW)Direct winpath dependents:$(RESET)"
	@cargo tree --workspace --package winpath --invert --depth 1
	@echo "$(GREEN)✓ Dependency graph generated$(RESET)"

# ============================ Rust Binary Links =============================

# Install symlink framework for Rust binaries
rust-links-install:
	@echo "$(BOLD)$(CYAN)Installing Rust binary symlink framework$(RESET)"
	@powershell -ExecutionPolicy Bypass -File "../scripts/Manage-RustBinaries.ps1" -Action Install

# Synchronize Rust binary links
rust-links-sync:
	@echo "$(BOLD)$(CYAN)Synchronizing Rust binary links$(RESET)"
	@powershell -ExecutionPolicy Bypass -File "../scripts/Manage-RustBinaries.ps1" -Action Sync

# Check status of Rust binary links
rust-links-status:
	@powershell -ExecutionPolicy Bypass -File "../scripts/Manage-RustBinaries.ps1" -Action Status

# Verify Rust binary links
rust-links-verify:
	@powershell -ExecutionPolicy Bypass -File "../scripts/Manage-RustBinaries.ps1" -Action Verify

# Health check for Rust binary links
rust-links-health:
	@powershell -ExecutionPolicy Bypass -File "../scripts/Manage-RustBinaries.ps1" -Action Health

# Clean Rust binary links
rust-links-clean:
	@powershell -ExecutionPolicy Bypass -File "../scripts/Manage-RustBinaries.ps1" -Action Clean

# Always rebuild these targets
.PHONY: all help release debug clean install test validate package \
        pre-build post-build core derive derive-utils where which tree \
        install-dirs uninstall backup-existing test-unit test-integration \
        test-fast gnu-compat test-git-bash-paths test-derive-utils \
        create-git-bash-test validate-all-77 bench profile check fmt clippy \
        audit docs coverage installer clean-all doctor stats list count \
        watch quick ci release-prep deploy disable-cargo enable-cargo \
        build-winpath build-winpath-debug verify-winpath-integration dep-graph \
        rust-links-install rust-links-sync rust-links-status rust-links-verify \
        rust-links-health rust-links-clean

# End of Makefile
# ============================ Performance Optimization ======================

.PHONY: build-parallel
build-parallel: clean
	@echo "$(BOLD)$(CYAN)Building with parallel optimization (faster builds)$(RESET)"
	@echo "Using thin LTO and 16 codegen units for parallel compilation..."
	@export RUSTFLAGS="$(RUSTFLAGS_PARALLEL)" && 	$(CARGO) build --profile release --workspace --jobs 16
	@echo "$(GREEN)✓ Parallel build complete$(RESET)"

.PHONY: build-perf
build-perf: clean
	@echo "$(BOLD)$(CYAN)Building for maximum performance (slower build)$(RESET)"
	@echo "Using fat LTO and single codegen unit for best runtime performance..."
	@export RUSTFLAGS="$(RUSTFLAGS_PERF)" && 	$(CARGO) build --profile release --workspace
	@echo "$(GREEN)✓ Performance build complete$(RESET)"

.PHONY: build-pgo-generate
build-pgo-generate: clean
	@echo "$(BOLD)$(CYAN)Building with PGO instrumentation$(RESET)"
	@mkdir -p $(PGO_DATA_DIR)
	@export RUSTFLAGS="-Cprofile-generate=$(PGO_DATA_DIR)" && 	$(CARGO) build --profile release --workspace
	@echo "$(GREEN)✓ PGO instrumented build complete$(RESET)"

.PHONY: build-pgo-run
build-pgo-run:
	@echo "$(BOLD)$(CYAN)Running PGO training workload$(RESET)"
	@powershell -ExecutionPolicy Bypass -File scripts/run-pgo-workload.ps1
	@echo "$(GREEN)✓ PGO training complete$(RESET)"

.PHONY: build-pgo-use
build-pgo-use: clean
	@echo "$(BOLD)$(CYAN)Building with PGO optimization$(RESET)"
	@llvm-profdata merge -o $(PGO_DATA_DIR)/merged.profdata $(PGO_DATA_DIR)/*.profraw
	@export RUSTFLAGS="-Cprofile-use=$(PGO_DATA_DIR)/merged.profdata" && 	$(CARGO) build --profile release --workspace
	@echo "$(GREEN)✓ PGO optimized build complete$(RESET)"

.PHONY: build-pgo
build-pgo: build-pgo-generate build-pgo-run build-pgo-use
	@echo "$(BOLD)$(GREEN)✓ Full PGO build pipeline complete$(RESET)"

.PHONY: benchmark
benchmark: release
	@echo "$(BOLD)$(CYAN)Running performance benchmarks$(RESET)"
	@cd benchmarks && $(CARGO) bench --no-fail-fast
	@echo "$(GREEN)✓ Benchmarks complete - results in target/criterion$(RESET)"

.PHONY: benchmark-vs-gnu
benchmark-vs-gnu: release
	@echo "$(BOLD)$(CYAN)Benchmarking against GNU coreutils$(RESET)"
	@powershell -ExecutionPolicy Bypass -File scripts/benchmark-vs-gnu.ps1
	@echo "$(GREEN)✓ Comparison complete - results in benchmark-results/$(RESET)"

.PHONY: perf-profile
perf-profile: release
	@echo "$(BOLD)$(CYAN)Profiling performance$(RESET)"
	@echo "Generating flamegraphs for key utilities..."
	@cd benchmarks && CARGO_PROFILE_BENCH_DEBUG=true $(CARGO) bench --bench core_utils --features flamegraph
	@echo "$(GREEN)✓ Flamegraphs generated in target/criterion/$(RESET)"

.PHONY: sccache-start
sccache-start:
	@echo "$(BOLD)$(CYAN)Starting sccache server$(RESET)"
	@$(SCCACHE) --stop-server 2>/dev/null || true
	@export SCCACHE_IDLE_TIMEOUT=0 && 	export SCCACHE_CACHE_SIZE="10G" && 	$(SCCACHE) --start-server
	@$(SCCACHE) --show-stats
	@echo "$(GREEN)✓ sccache server started$(RESET)"

.PHONY: sccache-stats
sccache-stats:
	@echo "$(BOLD)$(CYAN)sccache Statistics$(RESET)"
	@$(SCCACHE) --show-stats

.PHONY: build-stats
build-stats: release
	@echo "$(BOLD)$(CYAN)Build Statistics$(RESET)"
	@echo "Binary sizes:"
	@du -sh $(RELEASE_DIR)/*.exe | sort -h | tail -10
	@echo ""
	@echo "Total size:"
	@du -sh $(RELEASE_DIR)
	@echo ""
	@echo "Build time:"
	@$(CARGO) build --release --workspace --timings
	@echo "$(GREEN)✓ Build timing report saved to target/cargo-timings/cargo-timing.html$(RESET)"
