# sccache Configuration and Troubleshooting

## Overview

This project uses sccache for faster Rust compilation. However, sccache is **incompatible with code coverage builds** using cargo-llvm-cov.

## Configuration

### Current Setup

**File**: `.cargo/config.toml`

```toml
[build]
target-dir = "target"  # Local target directory
rustc-wrapper = "sccache"  # Use sccache for normal builds

[env]
RUSTC_WRAPPER = "sccache"
SCCACHE_DIR = "T:\\projects\\rust-mistral\\sccache-cache"
SCCACHE_CACHE_SIZE = "20G"
CARGO_INCREMENTAL = "0"  # Required for sccache
```

**Environment Variable Issue**:

- `CARGO_TARGET_DIR=C:\Users\david\.cargo\shared-target` is set globally
- This **overrides** the `target-dir` setting in config.toml
- Coverage builds **require** local target directory

## Problem: Coverage Builds Fail

### Symptoms

1. `cargo llvm-cov` fails with "no object files found"
1. Compilation errors when running `make test-coverage-open`
1. sccache conflicts with coverage instrumentation

### Root Causes

1. **Shared Target Directory**

   - Environment variable `CARGO_TARGET_DIR` points to shared directory
   - cargo-llvm-cov needs local `target/` directory
   - Coverage artifacts go to wrong location

1. **sccache Incompatibility**

   - sccache caches compiled artifacts
   - cargo-llvm-cov adds instrumentation at compile time
   - Cached artifacts don't have coverage instrumentation
   - Result: No coverage data collected

1. **Incremental Compilation**

   - sccache requires `CARGO_INCREMENTAL="0"`
   - cargo-llvm-cov works better with incremental on
   - Conflict between requirements

## Solution

### For Coverage Builds

**Use the PowerShell script** (recommended):

```powershell
# Generate and open HTML coverage report
.\run-coverage.ps1 -Mode open

# Generate LCOV for Codecov
.\run-coverage.ps1 -Mode lcov

# Fast coverage (no pyo3 crates)
.\run-coverage.ps1 -Mode fast

# Clean and regenerate
.\run-coverage.ps1 -Mode open -Clean
```

**Or use Makefile** (PowerShell wrapper):

```bash
make test-coverage-open      # Full workspace
make test-coverage-fast      # Fast (5 crates)
make test-coverage-text      # Text summary
```

**Manual execution**:

```powershell
# PowerShell: Temporarily disable sccache and use local target
$env:CARGO_TARGET_DIR = ""
$env:RUSTC_WRAPPER = ""
$env:CARGO_INCREMENTAL = "1"
cargo llvm-cov --workspace --all-features --open

# Restore environment afterwards
$env:CARGO_TARGET_DIR = "C:\Users\david\.cargo\shared-target"
$env:RUSTC_WRAPPER = "sccache"
$env:CARGO_INCREMENTAL = "0"
```

### For Normal Builds

**sccache works normally**:

```bash
cargo build --release
cargo test --workspace
make ci-full
```

The `.cargo/config.toml` settings apply automatically.

## Environment Variables

### Coverage Builds (Temporary Override)

```powershell
$env:CARGO_TARGET_DIR = ""           # Use local target/
$env:RUSTC_WRAPPER = ""              # Disable sccache
$env:CARGO_INCREMENTAL = "1"         # Enable incremental
```

### Normal Builds (Default)

```powershell
$env:CARGO_TARGET_DIR = "C:\Users\david\.cargo\shared-target"  # Shared
$env:RUSTC_WRAPPER = "sccache"       # Use sccache
$env:CARGO_INCREMENTAL = "0"         # Disable incremental
```

## Verification

### Check sccache Status

```powershell
# Check if sccache is running
sccache --show-stats

# View cache location and size
sccache --show-config

# Clear sccache cache (if needed)
sccache --zero-stats
```

### Check Environment

```powershell
# View current settings
Write-Host "CARGO_TARGET_DIR: $env:CARGO_TARGET_DIR"
Write-Host "RUSTC_WRAPPER: $env:RUSTC_WRAPPER"
Write-Host "CARGO_INCREMENTAL: $env:CARGO_INCREMENTAL"
```

### Test Coverage Build

```powershell
# Clean and test
cargo llvm-cov clean
.\run-coverage.ps1 -Mode text
```

Should show coverage percentages, not "no object files found".

## Troubleshooting

### Issue: "no object files found"

**Cause**: Wrong target directory or sccache enabled

**Solution**:

```powershell
# Use run-coverage.ps1 script (handles environment automatically)
.\run-coverage.ps1 -Mode open

# Or manually:
$env:CARGO_TARGET_DIR = ""
$env:RUSTC_WRAPPER = ""
cargo llvm-cov clean
cargo llvm-cov --workspace --all-features --open
```

### Issue: Compilation errors with pyo3

**Cause**: Python environment not configured

**Solution**: Use fast mode (skips pyo3 crates)

```powershell
.\run-coverage.ps1 -Mode fast
```

### Issue: sccache not working for normal builds

**Cause**: sccache not installed or not in PATH

**Solution**:

```powershell
# Install sccache
cargo install sccache

# Verify it's in PATH
sccache --version

# Start sccache server
sccache --start-server
```

### Issue: Shared target directory fills disk

**Cause**: Multiple projects using same target directory

**Solution**:

```powershell
# Clean shared target directory
Remove-Item -Recurse -Force "C:\Users\david\.cargo\shared-target"

# Or set CARGO_TARGET_DIR to local for this project
[Environment]::SetEnvironmentVariable("CARGO_TARGET_DIR", "", "User")
```

## Best Practices

### Do

✅ Use `run-coverage.ps1` for all coverage builds
✅ Use `make test-coverage-*` targets (they use the script)
✅ Let sccache handle normal builds automatically
✅ Clean coverage data between runs: `cargo llvm-cov clean`
✅ Use `test-coverage-fast` for quick iterations

### Don't

❌ Don't run `cargo llvm-cov` directly (use script instead)
❌ Don't mix sccache with coverage builds
❌ Don't manually set `CARGO_TARGET_DIR` globally for coverage
❌ Don't assume coverage works without environment changes
❌ Don't commit coverage artifacts to git

## CI Configuration

The GitHub Actions CI workflow automatically handles this:

```yaml
coverage:
  steps:
    - name: Generate coverage
      env:
        CARGO_TARGET_DIR: ''      # Use local target
        RUSTC_WRAPPER: ''         # Disable sccache
        CARGO_INCREMENTAL: '1'    # Enable incremental
      run: |
        cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
```

## Quick Reference

### Commands

```powershell
# Full workspace coverage (HTML, opens in browser)
.\run-coverage.ps1

# Fast coverage (5 crates, no pyo3)
.\run-coverage.ps1 -Mode fast

# Text summary only
.\run-coverage.ps1 -Mode text

# LCOV for Codecov
.\run-coverage.ps1 -Mode lcov

# Specific packages
.\run-coverage.ps1 -Packages mistralrs-core,mistralrs-agent-tools

# Clean before running
.\run-coverage.ps1 -Mode open -Clean
```

### Files

- `.cargo/config.toml` - Cargo configuration (sccache enabled)
- `run-coverage.ps1` - Coverage build script (disables sccache)
- `Makefile` - Coverage targets (use PowerShell script)
- `target/` - Local build artifacts
- `target/llvm-cov/` - Coverage artifacts
- `lcov.info` - Coverage data for Codecov

## Summary

**Key Points**:

1. **sccache** is great for normal builds (30-80% faster)
1. **sccache** is incompatible with coverage builds
1. **Solution**: Use `run-coverage.ps1` which handles environment automatically
1. **Coverage** requires local `target/` directory
1. **Coverage** needs sccache disabled temporarily

**Remember**: Always use the provided scripts/Makefile for coverage builds!

______________________________________________________________________

**Document Version**: 1.0\
**Last Updated**: 2025-01-05\
**Author**: Testing Infrastructure Team
