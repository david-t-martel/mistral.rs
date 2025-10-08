# Testing Documentation for mistral.rs

## Overview

The mistral.rs testing framework has been completely redesigned to provide comprehensive test coverage, automated validation, and seamless CI/CD integration. This guide serves as the central hub for all testing documentation.

## Testing Philosophy

Our testing approach follows these core principles:

1. **Makefile-First**: All builds and tests MUST use the Makefile, never bare `cargo` commands
1. **Comprehensive Coverage**: Unit, integration, MCP, and build system testing
1. **Fail-Fast Development**: Quick validation cycles with progressive testing depth
1. **Cross-Platform Validation**: Windows (CUDA), Linux (CPU/CUDA), macOS (Metal)
1. **Automated Everything**: From pre-commit hooks to CI/CD pipelines

## Quick Start

### Run All Tests

```powershell
# Windows PowerShell
.\tests\run-all-tests.ps1 -Suite all

# Linux/macOS
make test-all
```

### Quick Validation

```powershell
# Fast compilation check (< 1 minute)
make check

# Quick test suite (< 5 minutes)
.\tests\run-all-tests.ps1 -Suite quick
```

### Specific Test Categories

```powershell
# Integration tests only
.\tests\run-all-tests.ps1 -Suite integration

# MCP server tests
.\tests\run-all-tests.ps1 -Suite mcp

# Build system tests
.\tests\run-all-tests.ps1 -Suite build
```

## Directory Structure

```
mistral.rs/
├── tests/                      # All test code and scripts
│   ├── run-all-tests.ps1      # Master test runner (entry point)
│   ├── integration/            # Integration test scripts
│   │   ├── test-binary-health.ps1
│   │   ├── test-mistralrs.ps1
│   │   ├── test-phase1-completion.ps1
│   │   └── run-tui-test.ps1
│   ├── mcp/                    # MCP server tests
│   │   ├── MCP_CONFIG.json    # MCP server configuration
│   │   ├── test-mcp-config.ps1
│   │   ├── test-mcp-servers.ps1
│   │   ├── test-phase2-mcp-servers.ps1
│   │   └── test-rag-redis.ps1
│   ├── results/                # Test output and reports
│   └── validate-test-runner.ps1
├── scripts/                    # Automation scripts
│   ├── build/                 # Build and test scripts
│   ├── hooks/                 # Git hooks
│   └── tools/                 # Development tools
├── docs/                       # Documentation
│   ├── testing/               # Testing documentation (you are here)
│   │   ├── README.md          # This file
│   │   ├── integration-testing.md
│   │   ├── mcp-testing.md
│   │   ├── build-testing.md
│   │   └── ci-cd-testing.md
│   ├── development/           # Development guides
│   │   └── testing-migration.md
│   └── MODEL_INVENTORY.json   # Available test models
└── .github/                    # GitHub Actions workflows
    └── workflows/
        ├── ci.yml
        ├── rust-ci.yml
        ├── mcp-validation.yml
        └── powershell-tests.yml
```

## Test Categories

### 1. Unit Tests

- **Location**: Within each Rust crate (`src/` directories)
- **Command**: `make test-unit`
- **Purpose**: Test individual functions and modules
- **Coverage Target**: 80%+

### 2. Integration Tests

- **Location**: `tests/integration/`
- **Command**: `.\tests\run-all-tests.ps1 -Suite integration`
- **Purpose**: Test binary functionality, model loading, API endpoints
- **Key Tests**:
  - Binary health checks
  - Model loading validation
  - TUI functionality
  - HTTP API responses

### 3. MCP Server Tests

- **Location**: `tests/mcp/`
- **Command**: `.\tests\run-all-tests.ps1 -Suite mcp`
- **Purpose**: Validate MCP server integration
- **Coverage**: 9 different MCP servers including Memory, Filesystem, GitHub, RAG-Redis

### 4. Build System Tests

- **Location**: `scripts/build/`
- **Command**: `.\tests\run-all-tests.ps1 -Suite build`
- **Purpose**: Validate compilation, feature flags, cross-platform builds
- **Validates**:
  - CUDA compilation
  - Feature combinations
  - Binary optimization
  - Cross-compilation

## Test Models

The project uses specific models for testing, defined in `docs/MODEL_INVENTORY.json`:

| Model                        | Size   | Usage                   | Path                                            |
| ---------------------------- | ------ | ----------------------- | ----------------------------------------------- |
| Qwen2.5-1.5B-Instruct-Q4_K_M | 940MB  | Quick testing (default) | `C:\codedev\llm\.models\qwen2.5-1.5b-it-gguf\`  |
| Gemma 2 2B-it-Q4_K_M         | 1.67GB | Vision testing          | `C:\codedev\llm\.models\gemma-2-2b-it-gguf\`    |
| Qwen2.5-Coder-3B-Instruct    | 1.93GB | Code generation testing | `C:\codedev\llm\.models\qwen2.5-coder-3b-gguf\` |
| Qwen2.5-7B-Instruct-Q4_K_M   | 4.37GB | Performance testing     | `C:\codedev\llm\.models\qwen2.5-7b-it-gguf\`    |

## Master Test Runner

The `tests/run-all-tests.ps1` script is the single entry point for all testing:

### Features

- **Test Discovery**: Automatically finds all test scripts
- **MCP Lifecycle**: Manages MCP server start/stop
- **Parallel Execution**: Run tests concurrently where safe
- **Rich Reporting**: Console, JSON, Markdown, HTML output
- **CI Integration**: Special mode for GitHub Actions

### Usage Examples

```powershell
# Run all tests with JSON output
.\tests\run-all-tests.ps1 -Suite all -OutputFormat json -OutputFile results

# Run MCP tests with verbose output and fail fast
.\tests\run-all-tests.ps1 -Suite mcp -Verbose -FailFast

# CI mode (no prompts, strict validation)
.\tests\run-all-tests.ps1 -Suite all -CI

# Generate HTML report
.\tests\run-all-tests.ps1 -Suite integration -OutputFormat html
```

## Best Practices

### 1. Always Use the Makefile

```bash
# ✅ CORRECT
make test
make build-cuda-full

# ❌ WRONG
cargo test
cargo build --release
```

### 2. Test Before Committing

```bash
# Pre-commit validation
make check        # Quick compile check
make fmt-check    # Format validation
make lint         # Linting

# Full validation
make ci           # Complete CI pipeline locally
```

### 3. Use Appropriate Test Suites

- **During Development**: `quick` suite for rapid feedback
- **Before PR**: `integration` suite for functionality
- **Full Validation**: `all` suite for comprehensive testing

### 4. Monitor Test Performance

```powershell
# Check test duration trends
.\tests\run-all-tests.ps1 -Suite all -OutputFormat json
# Review slowest tests in output
```

### 5. Clean Test Environment

```powershell
# Clean before testing if issues occur
make clean-tests
Remove-Item tests\results\* -Force
```

## Troubleshooting

### Common Issues

#### Tests Fail with "Binary not found"

```bash
# Build the binary first
make build-cuda-full  # Windows/Linux with CUDA
make build-metal      # macOS
```

#### MCP Servers Won't Start

```powershell
# Check for existing processes
Get-Process -Name "node" | Where-Object {$_.CommandLine -like "*mcp*"}

# Kill stale processes
Stop-Process -Name "node" -Force

# Verify MCP config
cat tests\mcp\MCP_CONFIG.json
```

#### Out of Memory During Tests

```powershell
# Use smaller model for testing
$env:TEST_MODEL = "Qwen2.5-1.5B-Instruct-Q4_K_M"

# Limit parallel execution
.\tests\run-all-tests.ps1 -Suite all -Parallel:$false
```

#### Tests Pass Locally but Fail in CI

```powershell
# Run in CI mode locally
.\tests\run-all-tests.ps1 -Suite all -CI

# Check environment differences
make check-env
```

## Coverage Reports

### Generate Coverage

```bash
# Rust code coverage
make test-coverage

# View HTML report
start target/coverage/html/index.html  # Windows
open target/coverage/html/index.html   # macOS
xdg-open target/coverage/html/index.html  # Linux
```

### Coverage Targets

- **Core Libraries**: 85% minimum
- **Server Code**: 80% minimum
- **Integration Points**: 75% minimum
- **Overall Project**: 80% target

## Performance Benchmarks

### Run Benchmarks

```bash
# Quick benchmarks
make bench-quick

# Full benchmark suite
make bench-full

# Compare with baseline
make bench-compare
```

### Key Metrics

- **Model Loading**: < 5 seconds for 2B models
- **Inference Speed**: > 30 tokens/second (CUDA)
- **MCP Latency**: < 100ms per call
- **Memory Usage**: < 2x model size

## Related Documentation

- [Integration Testing Guide](integration-testing.md) - Deep dive into integration tests
- [MCP Testing Guide](mcp-testing.md) - MCP server testing details
- [Build Testing Guide](build-testing.md) - Build system validation
- [CI/CD Testing Guide](ci-cd-testing.md) - GitHub Actions and automation
- [Testing Migration Guide](../development/testing-migration.md) - Migrating from old structure

## Quick Reference

### Essential Commands

```bash
# Development cycle
make check          # Quick compile check
make test-unit      # Unit tests only
make test-integration  # Integration tests

# Full validation
make ci             # Complete CI pipeline
.\tests\run-all-tests.ps1 -Suite all

# Debugging
make test-debug TEST=specific_test
.\tests\run-all-tests.ps1 -Verbose -FailFast

# Cleanup
make clean-tests
Remove-Item tests\results\* -Recurse
```

### Environment Variables

```powershell
# Test configuration
$env:TEST_MODEL = "path/to/model.gguf"
$env:TEST_TIMEOUT = "300"  # seconds
$env:TEST_PARALLEL = "false"
$env:TEST_VERBOSE = "true"

# MCP configuration
$env:MCP_PROTOCOL_VERSION = "2025-06-18"
$env:MCP_TIMEOUT = "180"
```

## Contributing

When adding new tests:

1. **Choose the right location**:

   - Unit tests: In the crate's `src/` directory
   - Integration tests: `tests/integration/`
   - MCP tests: `tests/mcp/`

1. **Follow naming conventions**:

   - Test scripts: `test-*.ps1`
   - Test functions: `test_<functionality>`
   - Test data: `testdata/`

1. **Update documentation**:

   - Add to relevant testing guide
   - Update this README if adding new category

1. **Validate locally**:

   ```bash
   make ci
   .\tests\run-all-tests.ps1 -Suite all
   ```

## Support

For testing issues:

1. Check the [Troubleshooting](#troubleshooting) section
1. Review test logs in `tests/results/`
1. Run with `-Verbose` flag for detailed output
1. Open an issue with test logs attached

______________________________________________________________________

*Last Updated: 2025*
*Version: 1.0.0*
*Maintainer: mistral.rs Testing Team*
