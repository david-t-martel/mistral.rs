# Makefile Usage Guide

## Overview

A comprehensive Makefile has been created for the mistral.rs project to standardize builds and enforce best practices.

## ⚠️ Important: Shell Compatibility

**Windows Users**: Use PowerShell or cmd.exe to run make commands. Git Bash may have PATH variable conflicts.

```powershell
# In PowerShell or Command Prompt
make check
make build-cuda-full
make test
```

**Linux/macOS Users**: Standard bash works fine.

```bash
make check
make build-cuda-full
make test
```

## Quick Start

### Most Common Commands

```bash
# 1. First time - check environment
make check-env

# 2. Build the server (Windows with CUDA)
make build-cuda-full

# 3. Run tests
make test

# 4. Format code before committing
make fmt

# 5. Full validation
make ci
```

## All Available Targets

Run `make help` to see all targets (on PowerShell/cmd, not Git Bash).

### Development

- `make dev` - Quick development build (debug mode)
- `make check` - Check if code compiles (fast, no binary)
- `make check-server` - Check server package only

### Building

- `make build` - Release build (CPU only)
- `make build-cuda` - Build with CUDA support
- `make build-cuda-full` - Build with CUDA + Flash Attention + cuDNN + MKL
- `make build-metal` - Build with Metal support (macOS only)
- `make build-windows` - Alias for build-cuda-full
- `make full-build` - Clean + full CUDA build
- `make rebuild` - Complete rebuild from scratch

### Testing

- `make test` - Run all tests
- `make test-core` - Test core package
- `make test-server` - Test server package
- `make test-quant` - Test quantization
- `make test-vision` - Test vision models
- `make test-pyo3` - Test Python bindings

### Code Quality

- `make fmt` - Format all code (Rust + Python + C/CUDA)
- `make fmt-check` - Check if code is formatted
- `make lint` - Run clippy lints
- `make lint-fix` - Auto-fix linting issues
- `make audit` - Security audit

### CI/CD

- `make ci` - Full CI pipeline (fmt-check + check + lint + test)
- `make ci-fast` - Fast CI (check + lint, no tests)

### Cleaning

- `make clean` - Clean build artifacts
- `make clean-all` - Deep clean (including logs)
- `make clean-tests` - Clean test artifacts only
- `make clean-logs` - Clean log files only

### Python Bindings

- `make build-python` - Build PyO3 bindings
- `make install-python` - Install Python package locally
- `make wheel` - Create wheel distribution

### Running

- `make run` - Run server (requires MODEL variable)
- `make run-tui` - Run TUI (instructions provided)
- `make run-server` - Run HTTP server on port 8080
- `make run-with-mcp` - Run with MCP integration

### Utilities

- `make check-env` - Validate build environment
- `make check-cuda-env` - Check CUDA setup
- `make check-sccache` - Check if sccache is available
- `make setup-sccache` - Install sccache
- `make info` - Show build configuration
- `make version` - Show tool versions
- `make deps-tree` - Show dependency tree
- `make deps-duplicates` - Check for duplicate deps
- `make deps-update` - Update dependencies
- `make doc` - Generate documentation
- `make doc-open` - Generate and open documentation

## Environment Variables

The Makefile automatically configures most settings, but you can override:

```bash
# Build with specific number of parallel jobs
make build JOBS=4

# Enable verbose output
make build VERBOSE=1

# Use specific model for running
make run MODEL=meta-llama/Llama-3.2-3B-Instruct
```

## Troubleshooting

### "Recursive variable PATH" error

**Problem**: Git Bash on Windows has PATH conflicts with Make

**Solution**: Use PowerShell or cmd.exe instead

```powershell
# PowerShell (recommended)
make build-cuda-full

# Or use Windows cmd
make build-cuda-full
```

### Long build times

**Solution**: Install sccache for caching

```bash
make setup-sccache

# Then set environment variable
export RUSTC_WRAPPER=sccache  # Linux/Mac
$env:RUSTC_WRAPPER="sccache"  # PowerShell
```

### CUDA build fails

**Solution**: Check CUDA environment

```bash
# Verify CUDA is installed
make check-cuda-env

# Check environment variables
echo $CUDA_PATH
echo $CUDNN_PATH
```

## Pre-Commit Workflow

Before every commit:

```bash
# 1. Format code
make fmt

# 2. Run full CI
make ci

# 3. If all passes, commit
git add .
git commit -m "your message"
```

## Build Performance

With `sccache` enabled:

- **First build**: 30-45 minutes (Windows CUDA)
- **Incremental build**: 2-5 minutes
- **No changes**: ~30 seconds

## Files Created

This automation system includes:

1. **Makefile** - Main build automation (540+ lines)
1. **.claude/CLAUDE.md** - Comprehensive Rust best practices guide
1. **CLAUDE.md** (root) - Enhanced with Makefile references
1. **MAKEFILE_USAGE.md** - This usage guide

## References

- Full Rust best practices: See `.claude/CLAUDE.md`
- Project overview: See `CLAUDE.md`
- Original README: See `README.md`

## Summary

**Golden Rule**: Always use `make`, never `cargo` directly.

**Most Common**:

```bash
make dev          # Development build
make check        # Quick check
make build-cuda-full  # Full release build
make test         # Run tests
make ci           # Full validation
```

**Remember**: Use PowerShell/cmd on Windows, not Git Bash.
