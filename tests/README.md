# Test Suite Documentation

## Overview

The mistral.rs project uses a comprehensive testing framework with a master test runner that orchestrates all testing workflows.

## Quick Start

```bash
# Run all tests (fastest way)
make test-full

# Run only PowerShell tests
make test-ps1

# Run quick smoke tests
make test-ps1-quick

# Run specific suite
make test-ps1-integration
make test-ps1-mcp
```

## Master Test Runner

**Location**: `tests/run-all-tests.ps1`

The master test runner is the single entry point for all PowerShell-based testing workflows.

### Usage

```powershell
# Run all tests (default)
.\tests\run-all-tests.ps1

# Run specific suite
.\tests\run-all-tests.ps1 -Suite quick
.\tests\run-all-tests.ps1 -Suite integration
.\tests\run-all-tests.ps1 -Suite mcp
.\tests\run-all-tests.ps1 -Suite build

# Generate specific output format
.\tests\run-all-tests.ps1 -OutputFormat json
.\tests\run-all-tests.ps1 -OutputFormat markdown
.\tests\run-all-tests.ps1 -OutputFormat html

# Advanced options
.\tests\run-all-tests.ps1 -Verbose          # Detailed output
.\tests\run-all-tests.ps1 -FailFast         # Stop on first failure
.\tests\run-all-tests.ps1 -CI               # CI mode (no prompts)
.\tests\run-all-tests.ps1 -Parallel         # Parallel execution
```

### Test Suites

#### Quick Suite (`-Suite quick`)
- **Duration**: ~1 minute
- **Purpose**: Fast compilation check
- **Use Case**: Pre-commit validation
- **Commands**:
  - `make check` - Verify code compiles

#### Integration Suite (`-Suite integration`)
- **Duration**: ~5-10 minutes
- **Purpose**: End-to-end integration tests
- **Location**: `tests/integration/*.ps1`
- **Tests**:
  - TUI interaction tests
  - HTTP API tests
  - Model loading tests
  - Binary execution tests

#### MCP Suite (`-Suite mcp`)
- **Duration**: ~5-10 minutes
- **Purpose**: MCP server integration tests
- **Location**: `tests/mcp/*.ps1`
- **Requires**: MCP servers from `tests/mcp/MCP_CONFIG.json`
- **Tests**:
  - Server startup/shutdown
  - Tool availability
  - Request/response validation
  - Protocol compliance

#### Build Suite (`-Suite build`)
- **Duration**: ~10-15 minutes (depending on cache)
- **Purpose**: Build system validation
- **Location**: `scripts/build/test-*.ps1`
- **Tests**:
  - Cross-platform builds
  - Feature flag combinations
  - Optimization profiles
  - Binary verification

#### All Suite (`-Suite all`, default)
- **Duration**: ~15-20 minutes
- **Purpose**: Comprehensive validation
- **Runs**: All of the above in optimized order

### Output Formats

#### Console (default)
```powershell
.\tests\run-all-tests.ps1 -OutputFormat console
```
- Colored terminal output
- Real-time progress
- Summary table at end

#### JSON
```powershell
.\tests\run-all-tests.ps1 -OutputFormat json -OutputFile results
```
- Machine-readable format
- CI/CD integration friendly
- Structured test results
- Output: `results.json`

#### Markdown
```powershell
.\tests\run-all-tests.ps1 -OutputFormat markdown -OutputFile results
```
- Human-readable report
- GitHub/GitLab compatible
- Tables and formatting
- Output: `results.md`

#### HTML
```powershell
.\tests\run-all-tests.ps1 -OutputFormat html -OutputFile results
```
- Interactive web report
- Charts and visualizations
- Auto-opens in browser (non-CI)
- Output: `results.html`

## Test Organization

```
tests/
├── run-all-tests.ps1          # Master test runner
├── README.md                  # This file
├── results/                   # Test results and logs
│   ├── run-all-tests-*.json
│   ├── run-all-tests-*.md
│   ├── run-all-tests-*.html
│   └── archive/              # Old results
├── integration/               # Integration tests
│   ├── test-tui.ps1
│   ├── test-http-api.ps1
│   └── test-model-loading.ps1
└── mcp/                       # MCP integration tests
    ├── MCP_CONFIG.json
    ├── test-mcp-servers.ps1
    └── test-mcp-protocol.ps1
```

## MCP Server Management

The test runner automatically manages MCP server lifecycle during tests:

### Automatic Lifecycle
1. **Startup**: Servers are started before MCP tests
2. **Health Check**: Waits for servers to be ready
3. **Testing**: Runs MCP test suite
4. **Cleanup**: Gracefully stops all servers
5. **Force Kill**: Forcefully terminates if shutdown fails

### Manual Management
```powershell
# Start servers only
Start-MCPServers

# Stop specific servers
Stop-MCPServers -ServerPIDs @(1234, 5678)

# Check running servers
Get-Process -Name "node" | Where-Object { $_.CommandLine -like "*@modelcontextprotocol*" }
```

## CI/CD Integration

### GitHub Actions Example
```yaml
name: Test Suite

on: [push, pull_request]

jobs:
  test:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build
        run: make build

      - name: Run Tests
        run: make test-ps1-ci

      - name: Upload Results
        uses: actions/upload-artifact@v4
        with:
          name: test-results
          path: tests/results/*.json
```

### Exit Codes
- `0`: All tests passed
- `1`: One or more tests failed
- Other: Fatal error (see logs)

## Troubleshooting

### Common Issues

#### MCP Servers Won't Start
```powershell
# Check for conflicting processes
Get-Process -Name "node" | Stop-Process -Force

# Verify MCP config
Test-Path tests/mcp/MCP_CONFIG.json

# Check Node.js installation
node --version
npx --version
```

#### Tests Hang or Timeout
```powershell
# Run with FailFast to identify problem
.\tests\run-all-tests.ps1 -FailFast -Verbose

# Check for zombie processes
Get-Process | Where-Object { $_.ProcessName -like "*mistral*" }

# Kill stuck processes
taskkill /F /IM mistralrs-server.exe
```

#### Permission Denied
```powershell
# Run as Administrator
Start-Process powershell -Verb RunAs -ArgumentList "-ExecutionPolicy Bypass -File tests/run-all-tests.ps1"

# Or adjust execution policy
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

#### Binary Not Found
```bash
# Rebuild binary
make build-cuda-full

# Verify binary exists
ls -l target/release/mistralrs-server.exe

# Check PATH
echo $env:PATH
```

## Performance Tips

### Speed Up Tests
1. **Use Quick Suite**: `make test-ps1-quick` for fast checks
2. **Parallel Execution**: Use `-Parallel` flag (experimental)
3. **Cache Builds**: Ensure `sccache` is configured
4. **Skip Suites**: Run only needed suites (`-Suite integration`)

### Reduce Resource Usage
1. **Sequential Mode**: Default (safer, less memory)
2. **Single Suite**: Don't run `-Suite all` unnecessarily
3. **Cleanup**: Archive old results regularly
4. **Close Apps**: Free up VRAM before running

## Best Practices

### Pre-Commit
```bash
# Always run quick check before committing
make test-ps1-quick
```

### Pre-Push
```bash
# Run full suite before pushing
make test-full
```

### Pre-Release
```bash
# Comprehensive validation
.\tests\run-all-tests.ps1 -Suite all -OutputFormat html -Verbose
```

### CI Pipeline
```bash
# Strict mode with JSON output
.\tests\run-all-tests.ps1 -Suite all -CI -FailFast -OutputFormat json
```

## Contributing

When adding new tests:

1. **Choose Category**: integration, mcp, or build
2. **Create Script**: `tests/<category>/test-<name>.ps1`
3. **Follow Pattern**:
   ```powershell
   # Exit with 0 on success, non-zero on failure
   # Write structured output if possible
   # Handle cleanup in finally block
   ```
4. **Test Discovery**: Runner auto-discovers `*.ps1` in test directories
5. **Documentation**: Update this README with test details

## Support

For issues or questions:
- Check logs in `tests/results/`
- Review `.logs/build.log` for build issues
- Check GitHub issues: https://github.com/EricLBuehler/mistral.rs/issues
- Review project CLAUDE.md files for guidance

## Summary

**Quick Commands**:
```bash
make test-full         # Everything (Rust + PowerShell)
make test-ps1          # All PowerShell tests
make test-ps1-quick    # Fast smoke tests
make test-ps1-ci       # CI mode
```

**Direct Execution**:
```powershell
.\tests\run-all-tests.ps1                    # Default (all)
.\tests\run-all-tests.ps1 -Suite quick       # Fast
.\tests\run-all-tests.ps1 -OutputFormat html # Visual report
```

**Remember**: The master test runner handles all complexity - just run it! 🚀
