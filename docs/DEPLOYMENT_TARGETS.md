# Deployment & Validation Targets

This document describes the comprehensive deployment, testing, and validation targets added to the mistral.rs Makefile.

## Quick Reference

```bash
# Pre-Deployment
make pre-deploy              # Full validation (ALL checks + tests)
make pre-deploy-quick        # Quick validation (no full test suite)

# Testing
make test-validate           # Run ALL tests and fail if any fail
make test-integration-real   # Integration tests with real MCP servers
make test-examples           # Validate all examples compile

# Deployment
make deploy-check            # Check deployment readiness
make deploy-prepare          # Prepare deployment artifacts
make deploy-package          # Create deployment package (zip/tar.gz)

# Post-Deployment
make smoke-test              # Post-deployment smoke test
make smoke-test-quick        # Quick smoke test

# MCP Validation
make mcp-validate            # Validate MCP configuration
make mcp-test                # Test MCP servers can start
make mcp-test-tools          # Test tool execution works

# CI/CD
make ci-full                 # Full CI pipeline
make ci-test-matrix          # Run CI test matrix
```

## Overview

The deployment targets provide a production-grade workflow for:

1. **Pre-deployment validation** - Comprehensive checks before deployment
1. **Test validation** - Real test execution with pass/fail criteria
1. **Binary verification** - Ensure binary is valid and executable
1. **MCP server testing** - Validate MCP integration works
1. **Smoke testing** - Post-deployment verification
1. **Deployment readiness** - Complete deployment checklist

## Test Validation Targets

### `make test-validate`

**Full test validation with actual pass/fail criteria.**

Runs in 4 phases:

1. **Rust unit tests** - All workspace unit tests
1. **Integration tests** - PowerShell integration test suite
1. **MCP tests** - MCP server validation tests
1. **Agent tests** - Agent mode autonomous tests

**Exits with error code 1 if ANY test fails.**

```bash
make test-validate
# Output:
# ==================================================
# Running comprehensive test validation
# ==================================================
#
# Phase 1: Rust unit tests...
# ✓ Rust unit tests passed
#
# Phase 2: PowerShell integration tests...
# ✓ Integration tests passed
#
# Phase 3: PowerShell MCP tests...
# ✓ MCP tests passed
#
# Phase 4: Agent mode tests...
# ✓ Agent tests passed
#
# ==================================================
# ✓ ALL TESTS PASSED - 2025-10-03 14:30:00
# ==================================================
```

**Use case:** Pre-commit validation, CI/CD gates, deployment prerequisites.

### `make test-validate-quick`

**Quick test validation (unit tests + quick suite only).**

Faster version that runs:

- Rust library tests only (not integration tests)
- PowerShell quick test suite

**Use case:** Rapid feedback during development.

### `make test-integration-real`

**Integration testing with actual MCP servers.**

This target:

1. Validates binary exists
1. Validates MCP configuration
1. Runs binary health check
1. Tests MCP servers can start and respond
1. Runs full integration test suite

**Requires:**

- Built binary (`make build` first)
- MCP configuration at `tests/mcp/MCP_CONFIG.json`
- Node.js (for MCP servers via `npx`)

```bash
make build
make test-integration-real
```

**Use case:** Pre-deployment integration validation.

### `make test-examples`

**Validate all Rust examples compile.**

This target:

- Builds all examples in workspace
- Checks example binaries exist
- Validates example syntax

**Note:** Examples are not executed (require models).

**Use case:** Ensure example code stays up-to-date.

## Pre-Deployment Targets

### `make pre-deploy`

**Complete pre-deployment validation (9-phase checklist).**

Comprehensive validation pipeline:

```
[1/9] Environment check          ✓
[2/9] Code formatting check       ✓
[3/9] Compilation check           ✓
[4/9] Linting check               ✓
[5/9] Security audit              ✓
[6/9] Building release binary     ✓
[7/9] Binary verification         ✓
[8/9] Running all tests           ✓
[9/9] Integration tests           ✓
```

**Duration:** 15-30 minutes (includes full build + all tests)

**Use case:** Final validation before deployment/release.

### `make pre-deploy-quick`

**Quick pre-deployment check (no full test suite).**

Runs:

- Environment check
- Format check
- Compilation check
- Linting
- Build
- Binary verification
- Quick test validation

**Duration:** 5-10 minutes

**Use case:** Quick validation before pushing to CI.

## Binary Verification Targets

### `make verify-binary`

**Verify built binary is valid and executable.**

Checks:

1. Binary file exists at `target/release/mistral-rs.exe`
1. Binary is executable
1. Binary can run `--version` (warns if fails, doesn't error)

```bash
make verify-binary
# Output:
# Verifying binary...
# ✓ Binary exists: target/release/mistral-rs.exe
# -rwxr-xr-x 1 user 197609 383M Oct  2 19:35 target/release/mistral-rs.exe
#
# Checking binary can execute...
# WARNING: Binary exists but --version failed (may need GPU/models)
# ✓ Binary verification complete
```

**Use case:** Post-build validation, deployment prerequisites.

### `make verify-binary-help`

**Verify binary help output works.**

Tests that `mistral-rs --help` executes successfully (the legacy `mistralrs-server` alias is still supported).

**Use case:** Smoke test for binary functionality.

## Deployment Targets

### `make deploy-check`

**Comprehensive deployment readiness check.**

Validates:

1. Environment
1. Binary (exists, valid, correct size)
1. Configuration (MCP config)
1. Dependencies (cargo tree)
1. Test results (recent test runs)
1. Documentation (required files exist)

Generates deployment readiness summary:

```
==================================================
DEPLOYMENT READINESS SUMMARY
==================================================
Binary: target/release/mistral-rs.exe (alias: target/release/mistralrs-server.exe)
Platform: windows
Features: CUDA support (requires GPU)
MCP Servers: 9 configured
Test Coverage: ~80% (75+ tests)

✓ READY FOR DEPLOYMENT
==================================================
```

**Use case:** Pre-deployment checklist validation.

### `make deploy-prepare`

**Prepare deployment artifacts.**

Creates `deployment/` directory with:

```
deployment/
├── bin/
│   └── mistral-rs.exe
├── config/
│   └── MCP_CONFIG.json
└── docs/
    ├── AGENT_MODE_GUIDE.md
    ├── CLAUDE.md
    └── README.md
```

**Use case:** Package artifacts for distribution.

### `make deploy-package`

**Create deployment package (zip or tar.gz).**

Creates timestamped archive:

- `mistralrs-deployment-20251003-143000.zip` (if zip available)
- `mistralrs-deployment-20251003-143000.tar.gz` (fallback)

**Use case:** Create distributable package.

### `make deploy-verify`

**Verify deployment is ready.**

Runs:

1. `deploy-prepare` - Prepare artifacts
1. `smoke-test` - Verify functionality

**Use case:** Final deployment verification.

## Smoke Test Targets

### `make smoke-test`

**Post-deployment smoke test (4 tests).**

Verifies:

1. Binary exists and is executable
1. Binary health check passes
1. MCP configuration is valid
1. Agent mode basic functionality (warns if fails)

```bash
make smoke-test
# Output:
# ==================================================
# POST-DEPLOYMENT SMOKE TEST
# ==================================================
#
# Test 1: Binary exists and is executable...
# ✓ Binary OK
#
# Test 2: Binary health check...
# ✓ Health check passed
#
# Test 3: MCP configuration validation...
# ✓ MCP config valid
#
# Test 4: Agent mode basic test...
# ✓ Agent test complete
#
# ==================================================
# ✓ SMOKE TEST COMPLETE
# ==================================================
# Deployment verified and operational!
```

**Use case:** Post-deployment verification on target system.

### `make smoke-test-quick`

**Quick smoke test (binary + help only).**

Minimal smoke test:

- Binary verification
- Binary health check

**Duration:** \<1 minute

**Use case:** Rapid post-deployment check.

## MCP Validation Targets

### `make mcp-validate`

**Validate MCP configuration and servers.**

Checks:

1. `tests/mcp/MCP_CONFIG.json` exists
1. JSON is valid
1. Server definitions are correct
1. Required fields are present

**Use case:** Pre-deployment MCP validation.

### `make mcp-test`

**Test MCP servers can start and respond.**

Runs PowerShell MCP server tests:

- Tests each server can start
- Validates server responds to requests
- Checks tool registration

**Requires:** Node.js, npx, MCP server packages

**Use case:** MCP integration validation.

### `make mcp-test-tools`

**Test MCP tool execution works.**

Comprehensive MCP tool testing:

- Tool registration
- Tool execution
- Tool response validation
- Error handling

**Use case:** End-to-end MCP validation.

### `make mcp-health`

**Check MCP server health status.**

Lists all configured MCP servers and their commands.

**Use case:** Diagnostic check for MCP configuration.

## CI/CD Targets

### `make ci-full`

**Full CI pipeline.**

Complete CI workflow:

1. Code quality checks (`make ci`)
1. Build (`make build`)
1. Test validation (`make test-validate`)
1. Deployment check (`make deploy-check`)

**Duration:** 20-40 minutes

**Use case:** GitHub Actions, GitLab CI, Jenkins.

### `make ci-test-matrix`

**Run CI test matrix (all test types).**

Runs:

- `test-validate` - All tests with validation
- `test-integration-real` - Integration tests
- `test-examples` - Example validation

**Use case:** Comprehensive CI testing.

### `make deploy-check-ci`

**CI-specific deployment check (JSON output).**

Generates machine-readable deployment status:

```json
{
  "status": "ready",
  "binary": "target/release/mistralrs-server.exe",
  "platform": "windows"
}
```

**Use case:** CI/CD pipeline integration.

## Performance & Quality Targets

### `make perf-validate`

**Validate performance meets baseline criteria.**

Checks:

- Benchmark results exist
- Performance metrics documented
- Memory usage baseline

**Use case:** Performance regression detection.

### `make perf-regression-check`

**Check for performance regressions.**

Runs benchmarks and compares with baseline:

```bash
make perf-regression-check
cargo bench -- --baseline previous
```

**Use case:** Continuous performance monitoring.

### `make coverage-validate`

**Validate test coverage meets requirements.**

Runs coverage analysis with `cargo-tarpaulin`:

- Generates HTML report: `coverage/index.html`
- Generates Cobertura XML: `coverage/cobertura.xml`
- Target: 80% coverage

**Requires:** `cargo install cargo-tarpaulin`

**Use case:** Coverage tracking, CI integration.

### `make quality-metrics`

**Generate quality metrics report.**

Reports:

- Code lines
- Test count
- Clippy warnings
- TODO/FIXME count

**Use case:** Code quality tracking.

## Troubleshooting Targets

### `make diagnose`

**Run diagnostic checks for troubleshooting.**

Comprehensive diagnostic report:

1. Environment versions
1. Binary status
1. Recent test results
1. Recent logs
1. Dependency status
1. Disk space

**Use case:** Debugging build/test failures.

### `make test-status`

**Show current test status and results.**

Reports:

- Last test run timestamp
- Test coverage availability
- PowerShell test suite count

**Use case:** Quick status check.

## Typical Workflows

### Development Workflow

```bash
# 1. Make changes to code
vim mistralrs-server/src/agent_mode.rs

# 2. Quick validation
make check-server

# 3. Run tests
make test-validate-quick

# 4. Full validation before commit
make validate

# 5. Commit
git commit -m "feat: improve agent mode"
```

### Pre-Deployment Workflow

```bash
# 1. Full validation
make pre-deploy

# 2. Check deployment readiness
make deploy-check

# 3. Prepare artifacts
make deploy-prepare

# 4. Create package
make deploy-package

# 5. Verify
make deploy-verify
```

### Post-Deployment Workflow

```bash
# On target system after deployment:

# 1. Quick smoke test
make smoke-test-quick

# 2. Full smoke test
make smoke-test

# 3. If issues, diagnose
make diagnose
```

### CI/CD Workflow

```bash
# In CI pipeline:

# 1. Full CI
make ci-full

# 2. Deployment check
make deploy-check-ci

# 3. Create artifacts
make deploy-package
```

## Error Handling

All validation targets use **fail-fast** behavior:

- **Exit code 0** = Success
- **Exit code 1** = Failure (stops pipeline)

Example error output:

```bash
make test-validate
# ...
# Phase 2: PowerShell integration tests...
# ERROR: Integration tests failed
# make: *** [Makefile.deployment:XX: test-validate] Error 1
```

## Performance Expectations

### Duration Estimates

| Target                  | Duration  | Notes             |
| ----------------------- | --------- | ----------------- |
| `test-validate-quick`   | 2-5 min   | Quick feedback    |
| `test-validate`         | 10-15 min | Full test suite   |
| `test-integration-real` | 5-10 min  | Requires build    |
| `pre-deploy-quick`      | 5-10 min  | No full tests     |
| `pre-deploy`            | 20-30 min | Complete pipeline |
| `smoke-test-quick`      | \<1 min   | Minimal checks    |
| `smoke-test`            | 2-5 min   | Comprehensive     |
| `ci-full`               | 30-45 min | Full CI pipeline  |

### Resource Usage

- **CPU:** All cores utilized during build (`-j $(NPROC)`)
- **Memory:** Peak ~8GB during compilation
- **Disk:** ~5GB for build artifacts
- **Network:** MCP tests require internet (npx packages)

## Dependencies

### Required Tools

- **Rust toolchain** (rustc, cargo)
- **PowerShell** (Windows built-in)
- **Node.js + npx** (for MCP servers)
- **Git** (for version info)

### Optional Tools

- **cargo-tarpaulin** - Coverage reports
- **cargo-bloat** - Binary size analysis
- **sccache** - Build caching
- **zip/tar** - Package creation

Install optional tools:

```bash
cargo install cargo-tarpaulin cargo-bloat sccache
```

## Configuration

### Environment Variables

Set in shell or CI environment:

```bash
# Parallel jobs (default: CPU count)
export JOBS=8

# Verbose output
export VERBOSE=1

# CUDA paths (if not auto-detected)
export CUDA_PATH="/c/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.9"
export CUDNN_PATH="/c/Program Files/NVIDIA/CUDNN/v9.8"
```

### Customization

Edit `Makefile.deployment` to customize:

- Test timeout values
- Coverage thresholds
- Performance baselines
- CI output formats

## Troubleshooting

### Common Issues

**Issue:** `make test-validate` fails with "PowerShell script not found"

**Solution:**

```bash
# Check tests directory exists
ls tests/run-all-tests.ps1

# If missing, you're in wrong directory
cd T:/projects/rust-mistral/mistral.rs
```

**Issue:** `make mcp-test` fails with "npx not found"

**Solution:**

```bash
# Install Node.js (includes npx)
# Windows: https://nodejs.org/
# Or use existing Node.js installation
where npx  # Verify installation
```

**Issue:** `make test-integration-real` fails with "Binary not found"

**Solution:**

```bash
# Build binary first
make build
# Then run integration tests
make test-integration-real
```

**Issue:** Coverage target fails

**Solution:**

```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin
# Then run coverage
make coverage-validate
```

### Debug Mode

Enable verbose output for debugging:

```bash
# Set verbose flag
export VERBOSE=1

# Run with verbose output
make test-validate

# Or inline
make test-validate VERBOSE=1
```

### Log Files

Targets generate logs in:

```
.logs/
├── build.log           # Build output
├── build-cuda.log      # CUDA build output
└── build-cuda-full.log # Full CUDA build output

tests/results/
├── run-all-tests-YYYYMMDD-HHMMSS.json
├── run-all-tests-YYYYMMDD-HHMMSS.md
└── ...
```

Check logs for detailed error information:

```bash
cat .logs/build.log
cat tests/results/*.json
```

## Best Practices

### Pre-Commit

Always run before committing:

```bash
make validate
```

Or for faster feedback:

```bash
make check-server && make test-validate-quick
```

### Pre-Push

Run before pushing to remote:

```bash
make pre-deploy-quick
```

### Pre-Release

Run before creating release:

```bash
make pre-deploy
make deploy-check
make deploy-package
```

### In CI

Use CI-specific targets:

```bash
make ci-full
make deploy-check-ci
```

## Help & Support

### Getting Help

```bash
# Show deployment help
make help-deploy

# Show all targets
make help

# Show diagnostics
make diagnose

# Show test status
make test-status
```

### Documentation

- **Main README:** `README.md`
- **Agent Guide:** `docs/AGENT_MODE_GUIDE.md`
- **Build Guide:** `.claude/CLAUDE.md`
- **Test Guide:** `tests/README.md`

### Support Channels

- GitHub Issues: <https://github.com/EricLBuehler/mistral.rs/issues>
- Discord: <https://discord.gg/SZrecqK8qw>

## Summary

The deployment targets provide a **production-grade workflow** with:

✅ **Real validation** - No placeholders, actual test execution
✅ **Fail-fast behavior** - Exit code 1 on any failure
✅ **Comprehensive coverage** - Unit, integration, MCP, agent tests
✅ **CI/CD ready** - JSON output, machine-readable status
✅ **Documentation** - Inline comments, help targets
✅ **Best practices** - Based on actual project state

**Key targets to remember:**

```bash
make test-validate        # Run all tests
make pre-deploy          # Full pre-deployment validation
make deploy-check        # Check deployment readiness
make smoke-test          # Post-deployment verification
make ci-full             # Complete CI pipeline
```

**Start with these:**

```bash
# Quick check
make check-server

# Quick tests
make test-validate-quick

# Full validation
make validate

# Deploy readiness
make deploy-check
```

______________________________________________________________________

**Last Updated:** 2025-10-03
**Version:** 1.0.0
**Project:** mistral.rs deployment automation
