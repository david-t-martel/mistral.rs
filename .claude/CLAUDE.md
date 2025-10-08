# Rust Build Best Practices - mistral.rs

This file provides **mandatory** build and development practices for the mistral.rs project.

## 🚨 CRITICAL RULE: USE MAKEFILE ONLY

**❌ NEVER run `cargo` commands directly**
**✅ ALWAYS use `make` targets**

This ensures:

- Consistent build flags across all developers
- Proper environment variable setup (NVCC_CCBIN, CUDA paths)
- Dependency caching and incremental compilation
- Cross-platform compatibility
- Error handling and logging

### Why This Matters

Rust compilation is **complex** and **time-consuming**. Direct cargo usage leads to:

- ❌ Inconsistent builds (missing feature flags)
- ❌ Wasted compilation time (no caching strategy)
- ❌ Platform-specific errors (missing env vars)
- ❌ Failed CUDA builds (NVCC misconfiguration)
- ❌ Broken PyO3 bindings (Python version conflicts)

## Build System Architecture

### Makefile Hierarchy

```
make <target>
  ↓
Makefile (validates environment)
  ↓
cargo build (with correct flags)
  ↓
Compiled binaries in target/release/
```

### Key Principles

1. **Single Source of Truth**: Makefile defines ALL build commands
1. **Fail Fast**: Environment validation before compilation
1. **Incremental Builds**: Smart caching with sccache
1. **Platform Awareness**: Automatic Windows/Linux/macOS detection
1. **Feature Safety**: No accidental feature omissions

## Common Make Targets

### Development Workflow

```bash
# Check code compiles without building
make check

# Quick development build (debug mode)
make dev

# Format all code (Rust + Python + C/CUDA)
make fmt

# Run linters and fix auto-fixable issues
make lint-fix

# Run all tests
make test

# Full CI pipeline (check, test, lint, format)
make ci
```

### Release Builds

```bash
# Build release binary (production-ready)
make build

# Build with all optimizations (LTO, opt-level=3)
make release

# Build for specific platform
make build-windows
make build-linux

# Cross-compile for multiple platforms
make build-all-platforms
```

### CUDA/GPU Builds

```bash
# Build with CUDA support (validates NVCC setup)
make build-cuda

# Build with CUDA + Flash Attention + cuDNN
make build-cuda-full

# Verify CUDA environment before building
make check-cuda-env
```

### Cleaning

```bash
# Clean build artifacts
make clean

# Deep clean (including cargo cache)
make clean-all

# Remove only test artifacts
make clean-tests
```

### Testing

```bash
# Run core package tests
make test-core

# Run specific package tests
make test-server
make test-pyo3
make test-vision

# Run with coverage reporting
make test-coverage

# Run integration tests only
make test-integration

# Run performance benchmarks
make bench
```

### Python Bindings

```bash
# Build PyO3 bindings
make build-python

# Install Python package locally
make install-python

# Create wheel distribution
make wheel

# Test Python bindings
make test-python
```

### Model Management

```bash
# Download recommended test models
make download-models

# Generate model inventory
make model-inventory

# Verify models are accessible
make check-models
```

### MCP Integration

```bash
# Validate MCP configuration
make mcp-validate

# Test MCP servers
make mcp-test

# Start server with MCP integration
make run-with-mcp
```

## Rust-Specific Best Practices

### 1. Compilation Time Optimization

**Problem**: Rust compiles slowly, especially with many dependencies (660+ packages)

**Solutions** (already in Makefile):

- ✅ Use `sccache` for caching compiled artifacts
- ✅ Link with `lld` (faster linker)
- ✅ Incremental compilation enabled
- ✅ Parallel compilation (`-j $(nproc)`)
- ✅ Only rebuild changed crates

**What this means for you**:

```bash
# First build: 30-45 minutes
make build

# Subsequent builds: 2-5 minutes (with sccache)
make build
```

### 2. Feature Flag Management

**Problem**: mistral.rs has many conditional features (cuda, flash-attn, cudnn, mkl, metal)

**Solutions**:

- ✅ Makefile targets specify correct feature combinations
- ✅ Platform detection auto-selects appropriate features
- ✅ Validation ensures incompatible features aren't mixed

**What this means for you**:

```bash
# ❌ WRONG - Missing features, build succeeds but CUDA won't work
cargo build --release

# ✅ CORRECT - All CUDA features included
make build-cuda-full
```

### 3. Windows CUDA Configuration

**Problem**: NVCC requires MSVC compiler path, not always auto-detected

**Solutions**:

- ✅ Makefile sets `NVCC_CCBIN` automatically
- ✅ Validates Visual Studio installation
- ✅ Checks CUDA toolkit paths

**What this means for you**:

```bash
# ❌ WRONG - NVCC fails with "host compiler not found"
cargo build --features cuda

# ✅ CORRECT - Environment validated first
make build-cuda
```

### 4. Workspace Build Coordination

**Problem**: 14 crates in workspace with complex dependencies

**Solutions**:

- ✅ Makefile builds packages in correct order
- ✅ Skips PyO3 if Python not available
- ✅ Only rebuilds affected packages

**What this means for you**:

```bash
# Build only the server (skips Python bindings)
make build-server

# Build entire workspace (if Python available)
make build-all
```

### 5. Error Diagnostics

**Problem**: Cryptic error messages, especially for linker/CUDA issues

**Solutions**:

- ✅ Pre-flight environment checks
- ✅ Clear error messages for missing dependencies
- ✅ Build logs saved to `.logs/build.log`

**What this means for you**:

```bash
# Check environment before building
make check-env

# If build fails, check the log
cat .logs/build.log
```

## Development Workflow

### Starting a New Feature

```bash
# 1. Ensure codebase is clean
make check

# 2. Create feature branch
git checkout -b feature/my-feature

# 3. Make changes to code

# 4. Verify changes compile
make check

# 5. Run affected tests
make test-core  # or specific package

# 6. Format code
make fmt

# 7. Run linters
make lint

# 8. Full validation
make ci

# 9. Commit changes
git add .
git commit -m "feat: description"
```

### Debugging Build Issues

```bash
# 1. Clean everything
make clean-all

# 2. Check environment
make check-env

# 3. Verbose build to see full output
make build VERBOSE=1

# 4. Check specific dependency
cargo tree -p <dependency-name>

# 5. Check build logs
cat .logs/build.log
```

### Performance Optimization

```bash
# 1. Build with profiling enabled
make build-profiled

# 2. Run benchmarks
make bench

# 3. Compare with baseline
make bench-compare

# 4. Profile binary
make profile

# 5. Check binary size
make bloat-check
```

## Platform-Specific Notes

### Windows (Primary Development Platform)

**Required**:

- Visual Studio 2022 Build Tools
- CUDA Toolkit 12.9 (or 12.1, 12.6, 12.8, 13.0)
- cuDNN 9.8

**Environment**:

```powershell
# Makefile handles these automatically, but for reference:
$env:NVCC_CCBIN = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.40.33807\bin\Hostx64\x64\cl.exe"
$env:CUDA_PATH = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9"
$env:CUDNN_PATH = "C:\Program Files\NVIDIA\CUDNN\v9.8"
```

**Build**:

```bash
make build-cuda-full  # Uses all features
```

### Linux

**Required**:

- GCC/Clang
- CUDA Toolkit (if using GPU)
- pkg-config, libssl-dev

**Build**:

```bash
# CPU only
make build

# With CUDA
make build-cuda-full
```

### macOS

**Required**:

- Xcode Command Line Tools
- Metal (built-in)

**Build**:

```bash
make build-metal
```

## Testing Strategy

### Unit Tests

```bash
# Test individual crates
make test-core      # Core inference engine
make test-quant     # Quantization
make test-vision    # Vision models
make test-server    # Server components
```

### Integration Tests

```bash
# Full integration suite
make test-integration

# With specific model
make test-integration MODEL=qwen2.5-1.5b
```

### TUI Testing

```bash
# Interactive terminal test
make test-tui

# Automated TUI test with script
make test-tui-auto
```

### HTTP API Testing

```bash
# Start test server (background)
make test-server-start

# Run API tests
make test-api

# Stop test server
make test-server-stop
```

### Performance Testing

```bash
# Quick benchmark
make bench-quick

# Full benchmark suite
make bench-full

# Memory profiling
make profile-memory

# VRAM monitoring
make monitor-vram
```

## Common Pitfalls and Solutions

### Pitfall 1: "error: linker `link.exe` not found"

**Cause**: MSVC linker not in PATH

**Solution**:

```bash
# Makefile handles this, but if it fails:
make check-env  # Validates Visual Studio installation
```

### Pitfall 2: "nvcc fatal: Failed to preprocess host compiler properties"

**Cause**: NVCC can't find MSVC compiler

**Solution**:

```bash
# Makefile sets NVCC_CCBIN automatically
make build-cuda
```

### Pitfall 3: "error: could not compile `mistralrs-pyo3`"

**Cause**: Python not available or wrong version

**Solution**:

```bash
# Build server only (skips PyO3)
make build-server
```

### Pitfall 4: Build takes 45+ minutes every time

**Cause**: sccache not configured

**Solution**:

```bash
# Install and configure sccache
make setup-sccache

# Verify it's working
make check-sccache
```

### Pitfall 5: "error: feature X is not enabled"

**Cause**: Building without required features

**Solution**:

```bash
# ❌ Don't use bare cargo
cargo build

# ✅ Use appropriate make target
make build-cuda-full
```

### Pitfall 6: Out of memory during compilation

**Cause**: Parallel compilation using too much RAM

**Solution**:

```bash
# Limit parallel jobs
make build JOBS=4

# Or use low-memory profile
make build-low-memory
```

## Performance Expectations

### First Build (Cold Cache)

- **Windows + CUDA**: 30-45 minutes
- **Linux + CUDA**: 25-35 minutes
- **macOS + Metal**: 20-30 minutes
- **CPU only**: 15-25 minutes

### Incremental Build (Hot Cache)

- **With sccache**: 2-5 minutes
- **Without sccache**: 10-15 minutes
- **Single crate change**: 30-90 seconds

### Binary Size

- **mistralrs-server.exe**: ~380 MB (Windows)
- **mistralrs-server**: ~320 MB (Linux)
- **With LTO + strip**: ~280 MB

### VRAM Usage (Runtime)

- **Qwen2.5-1.5B-Q4**: ~1.2 GB
- **Gemma 2 2B-Q4**: ~2.0 GB
- **Qwen2.5-7B-Q4**: ~5.5 GB
- **Gemma 3 4B-full**: ~9.2 GB

## Debugging Techniques

### Enable Verbose Logging

```bash
# During build
make build VERBOSE=1

# During runtime
MISTRALRS_DEBUG=1 make run
```

### Check Compilation Cache

```bash
# sccache statistics
make sccache-stats

# Clear cache if corrupted
make clean-cache
```

### Inspect Dependencies

```bash
# View dependency tree
make deps-tree

# Check for duplicate dependencies
make deps-duplicates

# Audit for security vulnerabilities
make audit
```

### Profile Compilation Time

```bash
# Generate compilation timing report
make build-timings

# View slowest dependencies
make slowest-deps
```

## Environment Variables Reference

Makefile automatically sets these, but for reference:

| Variable            | Purpose                 | Example                              |
| ------------------- | ----------------------- | ------------------------------------ |
| `NVCC_CCBIN`        | NVCC host compiler      | `cl.exe` path                        |
| `CUDA_PATH`         | CUDA toolkit location   | `C:\Program Files\...\CUDA\v12.9`    |
| `CUDNN_PATH`        | cuDNN library location  | `C:\Program Files\NVIDIA\CUDNN\v9.8` |
| `RUSTC_WRAPPER`     | Compilation cache       | `sccache`                            |
| `CARGO_INCREMENTAL` | Incremental compilation | `1`                                  |
| `CARGO_TARGET_DIR`  | Build output directory  | `target`                             |
| `RUST_BACKTRACE`    | Error stack traces      | `1` or `full`                        |

## Pre-Commit Checklist

Before every commit:

```bash
# 1. Format code
make fmt

# 2. Check compilation
make check

# 3. Run linters
make lint

# 4. Run tests
make test

# 5. Full CI validation
make ci
```

## Troubleshooting

### Build fails with "out of disk space"

```bash
# Clean old artifacts
make clean-all

# Check cargo cache size
du -sh ~/.cargo

# Clean cargo cache if needed
cargo cache --autoclean
```

### Build fails with "connection refused" for crates.io

```bash
# Check network
ping crates.io

# Use alternative registry mirror
make build REGISTRY=mirrors.ustc.edu.cn
```

### Tests fail but binary works

```bash
# Clean test artifacts
make clean-tests

# Rebuild and retest
make test
```

## Getting Help

If build issues persist:

1. Check `.logs/build.log` for detailed errors
1. Run `make check-env` to validate environment
1. Try `make clean-all && make build`
1. Check GitHub issues: https://github.com/EricLBuehler/mistral.rs/issues
1. Review recent commits for breaking changes

## Summary

**Golden Rules**:

1. ✅ **Always use `make`, never use `cargo` directly**
1. ✅ **Run `make check` before committing**
1. ✅ **Use `make ci` before creating pull requests**
1. ✅ **Check `.logs/build.log` when builds fail**
1. ✅ **Use appropriate make target for your platform**

**Most Common Commands**:

```bash
make dev          # Quick development build
make check        # Verify code compiles
make test         # Run tests
make fmt          # Format code
make build        # Release build
make ci           # Full validation
```

______________________________________________________________________

**Remember**: The Makefile is your friend. It handles all the complexity of Rust + CUDA + cross-platform builds so you don't have to.
