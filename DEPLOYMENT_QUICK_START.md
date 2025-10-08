# Deployment Targets - Quick Start Guide

_Reference: Review the [Repository Guidelines](AGENTS.md) for shared contribution standards before executing these deployment tasks._

**TL;DR:** 35 new Makefile targets for testing, deployment, and validation.

## Most Common Commands

```bash
# Show all deployment targets
make help-deploy

# Quick validation
make verify-binary              # Check binary exists (<5s)
make diagnose                   # System diagnostics (10-30s)

# Development
make check-server               # Quick compile check (30s)
make test-validate-quick        # Quick tests (2-5 min)
make validate                   # Full validation (5-10 min)

# Pre-Deployment
make pre-deploy-quick           # Quick check (5-10 min)
make pre-deploy                 # Full validation (20-30 min)
make deploy-check               # Readiness check (1-2 min)

# Deployment
make deploy-prepare             # Prepare artifacts (2-5 min)
make deploy-package             # Create package (1-2 min)

# Post-Deployment
make smoke-test-quick           # Quick check (<1 min)
make smoke-test                 # Full smoke test (2-5 min)

# MCP Validation
make mcp-validate               # Validate config (<5s)
make mcp-health                 # Check servers (<5s)
make mcp-test                   # Test servers (1-2 min)

# CI/CD
make ci-full                    # Full CI (30-45 min)
make ci-test-matrix             # Test matrix (15-20 min)
```

## Typical Workflows

### During Development

```bash
# Quick feedback loop
make check-server && make test-validate-quick
```

### Before Committing

```bash
# Full validation
make validate
```

### Before Pushing to CI

```bash
# Quick pre-deployment check
make pre-deploy-quick
```

### Before Release/Deploy

```bash
# Complete validation
make pre-deploy

# Check readiness
make deploy-check

# Create package
make deploy-package
```

### After Deployment

```bash
# On target system
make smoke-test
```

### Troubleshooting

```bash
# Diagnostics
make diagnose

# Check test results
make test-status

# Check MCP configuration
make mcp-health
```

## Target Categories

### Test Validation

- `test-validate` - Run ALL tests
- `test-validate-quick` - Quick tests
- `test-integration-real` - Integration with MCP
- `test-examples` - Validate examples

### Pre-Deployment

- `pre-deploy` - Complete validation
- `pre-deploy-quick` - Quick check
- `verify-binary` - Verify binary
- `verify-binary-help` - Test help output

### Deployment

- `deploy-check` - Readiness check
- `deploy-check-ci` - CI check (JSON output)
- `deploy-prepare` - Prepare artifacts
- `deploy-package` - Create package
- `deploy-verify` - Verify ready

### Smoke Tests

- `smoke-test` - Post-deployment test
- `smoke-test-quick` - Quick smoke test

### MCP Validation

- `mcp-validate` - Validate config
- `mcp-test` - Test servers
- `mcp-test-tools` - Test tool execution
- `mcp-health` - Check health

### CI/CD

- `ci-full` - Full CI pipeline
- `ci-test-matrix` - Test matrix

### Performance & Quality

- `perf-validate` - Validate performance
- `perf-regression-check` - Check regressions
- `coverage-validate` - Test coverage
- `quality-metrics` - Quality metrics

### Troubleshooting

- `diagnose` - Diagnostics
- `test-status` - Test status
- `help-deploy` - Show help

## Performance Guide

| Target                | Time     | Use When        |
| --------------------- | -------- | --------------- |
| `verify-binary`       | \<5s     | Quick check     |
| `diagnose`            | 10-30s   | Troubleshooting |
| `mcp-health`          | \<5s     | Config check    |
| `test-validate-quick` | 2-5min   | Development     |
| `test-validate`       | 10-15min | Pre-commit      |
| `pre-deploy-quick`    | 5-10min  | Before push     |
| `pre-deploy`          | 20-30min | Before release  |
| `smoke-test-quick`    | \<1min   | Quick verify    |
| `smoke-test`          | 2-5min   | Post-deploy     |
| `ci-full`             | 30-45min | CI pipeline     |

## Error Handling

All targets use **fail-fast** behavior:

- Exit code 0 = Success
- Exit code 1 = Failure (stops pipeline)

## Documentation

- **Quick Start:** This file
- **Complete Guide:** `docs/DEPLOYMENT_TARGETS.md`
- **Implementation:** `MAKEFILE_TARGETS_SUMMARY.md`
- **Main Build Guide:** `.claude/CLAUDE.md`

## Getting Help

```bash
# Show deployment targets
make help-deploy

# Show all targets
make help

# Show diagnostics
make diagnose
```

## Dependencies

**Required:**

- Rust toolchain
- PowerShell (Windows)
- Node.js + npx (for MCP)

**Optional:**

- cargo-tarpaulin (coverage)
- cargo-bloat (binary analysis)
- sccache (build caching)

## Quick Troubleshooting

**Tests failing?**

```bash
make diagnose
make test-status
```

**Binary issues?**

```bash
make verify-binary
```

**MCP problems?**

```bash
make mcp-validate
make mcp-health
```

**Build issues?**

```bash
make check-env
cat .logs/build.log
```

## Files Created

- `Makefile.deployment` - 555 lines of targets
- `docs/DEPLOYMENT_TARGETS.md` - 843 lines of docs
- `MAKEFILE_TARGETS_SUMMARY.md` - Implementation summary
- `DEPLOYMENT_IMPLEMENTATION_COMPLETE.md` - Final report
- `DEPLOYMENT_QUICK_START.md` - This file

## Status

✅ **COMPLETE AND VALIDATED**

35 production-grade targets ready for use.

______________________________________________________________________

**Last Updated:** 2025-10-03
**Version:** 1.0.0
