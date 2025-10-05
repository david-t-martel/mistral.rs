# sccache and Code Coverage Compatibility Guide

This guide explains the incompatibility between sccache and cargo-llvm-cov, and provides solutions for running code coverage builds in the mistral.rs project.

## Table of Contents

- [Problem Overview](#problem-overview)
- [Root Cause](#root-cause)
- [Solution](#solution)
- [Usage](#usage)
- [CI/CD Configuration](#cicd-configuration)
- [Troubleshooting](#troubleshooting)

## Problem Overview

### The Incompatibility

sccache and cargo-llvm-cov are fundamentally incompatible:

1. **sccache requirement**: Explicitly prohibits incremental compilation

   - sccache error: "increment compilation is prohibited"
   - sccache needs clean, cacheable compilation units

1. **cargo-llvm-cov requirement**: Requires incremental compilation

   - Uses LLVM profiling data from incremental builds
   - Instruments code for coverage analysis

1. **Result**: Cannot use both simultaneously

### Additional Environment Issues

The project has additional complexity:

1. **Shared target directory**: `CARGO_TARGET_DIR` set to `C:\Users\david\.cargo\shared-target`

   - Conflicts with local `target/` directory in `.cargo/config.toml`
   - Can cause cache corruption and build failures

1. **Config file settings**: `.cargo/config.toml` hardcodes sccache:

   ```toml
   [build]
   rustc-wrapper = "sccache"

   [env]
   RUSTC_WRAPPER = "sccache"
   ```

## Root Cause

### Configuration Precedence

Cargo's configuration precedence (highest to lowest):

1. Environment variables (when **set**, even to empty string)
1. `.cargo/config.toml` settings
1. Default values

### The Critical Insight

❌ **What doesn't work**:

```powershell
Remove-Item Env:\RUSTC_WRAPPER -ErrorAction SilentlyContinue
```

Removing the variable makes cargo fall back to `.cargo/config.toml` where `rustc-wrapper = "sccache"` is defined.

✅ **What works**:

```powershell
$env:RUSTC_WRAPPER = ""
```

Setting to empty string **overrides** the config file setting.

### Error Message

When configuration is wrong, you'll see:

```
error: process didn't exit successfully: `sccache C:\Users\david\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\rustc.exe -vV` (exit code: 1)
--- stdout
sccache: increment compilation is prohibited.
```

## Solution

### Environment Variable Configuration

For coverage builds, configure three environment variables:

1. **Disable sccache**:

   ```powershell
   $env:RUSTC_WRAPPER = ""  # Empty string, not removed!
   ```

1. **Use local target directory**:

   ```powershell
   Remove-Item Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue
   ```

1. **Enable incremental compilation**:

   ```powershell
   $env:CARGO_INCREMENTAL = "1"
   ```

### Why These Settings Work

- **`RUSTC_WRAPPER = ""`**: Overrides `.cargo/config.toml`, tells cargo not to use a wrapper
- **No `CARGO_TARGET_DIR`**: Uses local `target/` specified in `.cargo/config.toml`
- **`CARGO_INCREMENTAL = "1"`**: Overrides the `"0"` setting in `.cargo/config.toml`

## Usage

### Option 1: PowerShell Script (Recommended)

Use the provided `run-coverage.ps1` script:

```powershell
# Generate HTML report and open in browser
.\run-coverage.ps1 -Mode open

# Generate text summary for specific package
.\run-coverage.ps1 -Mode text -Packages mistralrs-agent-tools

# Generate LCOV report for CI
.\run-coverage.ps1 -Mode lcov

# Generate JSON report
.\run-coverage.ps1 -Mode json

# Fast coverage (skip pyo3 crates)
.\run-coverage.ps1 -Mode fast

# Multiple packages
.\run-coverage.ps1 -Mode html -Packages mistralrs-core,mistralrs-quant

# Clean before running
.\run-coverage.ps1 -Mode open -Clean
```

#### Script Features

- ✅ Automatically configures environment correctly
- ✅ Saves and restores original environment in `finally` block
- ✅ Supports all cargo-llvm-cov output modes
- ✅ Package and feature filtering
- ✅ Clean coverage data option

### Option 2: Makefile Targets

Use make targets for common workflows:

```bash
# Generate and open HTML report
make test-coverage-open

# Generate HTML report (don't open)
make test-coverage

# Text summary in terminal
make test-coverage-text

# LCOV format for CI
make test-coverage-lcov

# JSON format
make test-coverage-json

# Fast coverage (skip pyo3 crates)
make test-coverage-fast

# CI format (LCOV)
make test-coverage-ci
```

### Option 3: Manual Command

Run cargo-llvm-cov directly with proper environment:

```powershell
# PowerShell
$env:RUSTC_WRAPPER = ""
Remove-Item Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue
$env:CARGO_INCREMENTAL = "1"
cargo llvm-cov --workspace --all-features --html --open
```

```bash
# Bash
RUSTC_WRAPPER="" CARGO_INCREMENTAL=1 cargo llvm-cov --workspace --all-features --html --open
```

## CI/CD Configuration

### GitHub Actions

```yaml
name: Code Coverage

on:
  pull_request:
  push:
    branches: [main, master]

jobs:
  coverage:
    runs-on: windows-latest
    
    steps:
      - uses: actions/checkout@v4
      
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview
      
      - name: Install cargo-llvm-cov
        run: cargo install cargo-llvm-cov
      
      - name: Generate Coverage
        env:
          RUSTC_WRAPPER: ""          # Disable sccache
          CARGO_INCREMENTAL: "1"     # Enable incremental
          # CARGO_TARGET_DIR not set = use local target/
        run: |
          cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
      
      - name: Upload to codecov.io
        uses: codecov/codecov-action@v4
        with:
          token: ${{ secrets.CODECOV_TOKEN }}
          files: lcov.info
          fail_ci_if_error: true
```

### GitLab CI

```yaml
coverage:
  stage: test
  image: rust:latest
  variables:
    RUSTC_WRAPPER: ""
    CARGO_INCREMENTAL: "1"
  before_script:
    - rustup component add llvm-tools-preview
    - cargo install cargo-llvm-cov
  script:
    - cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
  coverage: '/^\s*lines\.*:\s*\d+\.\d+%/'
  artifacts:
    reports:
      coverage_report:
        coverage_format: cobertura
        path: lcov.info
```

## Troubleshooting

### Error: "sccache: increment compilation is prohibited"

**Cause**: sccache is still being used for coverage build.

**Solution**: Ensure `RUSTC_WRAPPER` is set to empty string, not removed:

```powershell
$env:RUSTC_WRAPPER = ""  # Correct
Remove-Item Env:\RUSTC_WRAPPER  # Wrong - falls back to config file
```

### Error: "no such file or directory"

**Cause**: Using shared target directory instead of local.

**Solution**: Remove `CARGO_TARGET_DIR`:

```powershell
Remove-Item Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue
```

### Error: Build succeeds but coverage is 0%

**Cause**: Incremental compilation might be disabled.

**Solution**: Ensure `CARGO_INCREMENTAL=1`:

```powershell
$env:CARGO_INCREMENTAL = "1"
```

### Slow coverage builds

**Cause**: Building all crates including heavy pyo3 bindings.

**Solutions**:

1. Use fast coverage (skip pyo3):

   ```bash
   make test-coverage-fast
   ```

   Or:

   ```powershell
   .\run-coverage.ps1 -Mode fast
   ```

1. Coverage for specific packages:

   ```powershell
   .\run-coverage.ps1 -Mode html -Packages mistralrs-core,mistralrs-quant
   ```

### Environment not restored after failure

**Cause**: Script crashed before `finally` block.

**Manual fix**:

```powershell
# Check current values
$env:RUSTC_WRAPPER
$env:CARGO_TARGET_DIR
$env:CARGO_INCREMENTAL

# Restore original values (adjust as needed)
$env:RUSTC_WRAPPER = "sccache"
$env:CARGO_TARGET_DIR = "C:\Users\david\.cargo\shared-target"
$env:CARGO_INCREMENTAL = "0"
```

### Coverage shows "cfg(coverage)" warnings

**Cause**: cargo-llvm-cov sets `cfg(coverage)` for conditional compilation.

**Solution**: This is normal. To disable, use `--no-cfg-coverage`:

```powershell
cargo llvm-cov --no-cfg-coverage --workspace --all-features --html
```

## Best Practices

### Development Workflow

1. **Regular builds**: Use sccache for fast iterative development

   ```bash
   cargo build --all-features
   ```

1. **Coverage analysis**: Disable sccache, use PowerShell script

   ```powershell
   .\run-coverage.ps1 -Mode open
   ```

1. **CI/CD**: Always disable sccache for coverage jobs

   ```yaml
   env:
     RUSTC_WRAPPER: ""
     CARGO_INCREMENTAL: "1"
   ```

### Performance Tips

1. **Use fast coverage during development**:

   ```bash
   make test-coverage-fast  # Skip pyo3 crates
   ```

1. **Full coverage only in CI**:

   ```bash
   make test-coverage-ci    # All crates, LCOV format
   ```

1. **Package-specific coverage for debugging**:

   ```powershell
   .\run-coverage.ps1 -Mode text -Packages mistralrs-core
   ```

### Don't Mix Environments

❌ **Don't** try to use sccache and coverage together:

```powershell
# This will fail
$env:RUSTC_WRAPPER = "sccache"
cargo llvm-cov --workspace
```

✅ **Do** keep them separate:

```powershell
# Regular build with sccache
cargo build --all-features

# Coverage build without sccache
.\run-coverage.ps1 -Mode html
```

## Technical Details

### Why sccache Prohibits Incremental

sccache caches individual compilation units. Incremental compilation:

- Reuses partial compilation artifacts
- Stores state across builds
- Invalidates sccache's caching strategy

Result: sccache explicitly checks for and blocks incremental compilation.

### Why cargo-llvm-cov Needs Incremental

LLVM coverage uses:

- Profile-guided instrumentation
- Incremental metadata for coverage maps
- Runtime profiling data linked to source

Result: cargo-llvm-cov requires `CARGO_INCREMENTAL=1`.

### Environment Variable Precedence

Cargo resolution order:

1. Command-line flags (`cargo --config env.VAR=value`)
1. Environment variables (if **set**, even to "")
1. `.cargo/config.toml` `[env]` section
1. `.cargo/config.toml` `[build]` section
1. Default values

**Key insight**: An environment variable set to "" overrides config files, but a removed/unset variable does not.

## Reference

### Files

- `run-coverage.ps1` - PowerShell script for coverage builds
- `Makefile` - Make targets for coverage
- `.cargo/config.toml` - Project configuration (includes sccache)
- `COVERAGE_FIX_STATUS.md` - Status and progress report

### Commands

```powershell
# Check environment
$env:RUSTC_WRAPPER; $env:CARGO_TARGET_DIR; $env:CARGO_INCREMENTAL

# Verify sccache status
sccache --show-stats

# Clean coverage data
cargo llvm-cov clean

# Check cargo configuration
cargo config get build.rustc-wrapper
cargo config get env.RUSTC_WRAPPER
```

### Links

- [cargo-llvm-cov documentation](https://github.com/taiki-e/cargo-llvm-cov)
- [sccache documentation](https://github.com/mozilla/sccache)
- [Cargo environment variables](https://doc.rust-lang.org/cargo/reference/environment-variables.html)
- [Cargo configuration](https://doc.rust-lang.org/cargo/reference/config.html)

______________________________________________________________________

**Last Updated**: 2025-01-XX\
**Version**: 1.1\
**Status**: Verified working solution
