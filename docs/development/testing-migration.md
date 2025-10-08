# Testing Migration Guide

## Overview

This guide helps developers migrate from the old testing structure to the new consolidated testing framework introduced in mistral.rs v0.6.0+. The new structure provides better organization, unified execution, and comprehensive CI/CD integration.

## Migration Timeline

| Phase   | Date    | Status         | Description                     |
| ------- | ------- | -------------- | ------------------------------- |
| Phase 1 | 2025 Q1 | ✅ Complete    | New structure created           |
| Phase 2 | 2025 Q1 | 🔄 In Progress | Migration of existing tests     |
| Phase 3 | 2025 Q2 | ⏳ Planned     | Deprecation of old structure    |
| Phase 4 | 2025 Q3 | ⏳ Planned     | Complete removal of legacy code |

## Old vs New Structure

### Directory Changes

#### Old Structure (Deprecated)

```
mistral.rs/
├── test-*.ps1                    # Scattered test scripts in root
├── run-*.ps1                      # Various runners in root
├── check-*.ps1                    # Validation scripts in root
├── .testlogs/                     # Test output directory
├── *_RESULTS.json                 # Result files in root
├── *_REPORT.md                    # Report files in root
└── tests/                         # Mixed Rust tests
    └── *.rs
```

#### New Structure (Current)

```
mistral.rs/
├── tests/                         # All test code centralized
│   ├── run-all-tests.ps1        # Master test runner
│   ├── integration/              # Integration tests
│   │   ├── test-*.ps1
│   │   └── run-*.ps1
│   ├── mcp/                      # MCP server tests
│   │   ├── MCP_CONFIG.json
│   │   └── test-*.ps1
│   ├── results/                  # All test output
│   └── validate-test-runner.ps1
├── scripts/                       # Automation scripts
│   ├── build/                    # Build-related tests
│   │   └── test-*.ps1
│   ├── hooks/                    # Git hooks
│   └── ci/                       # CI/CD scripts
└── .github/                       # GitHub Actions
    └── workflows/
        ├── ci.yml
        ├── mcp-validation.yml
        └── powershell-tests.yml
```

## Migration Steps

### Step 1: Identify Existing Tests

First, catalog all existing test scripts:

```powershell
# Find all test scripts in old locations
$oldTests = @()

# Root directory test scripts
$oldTests += Get-ChildItem -Path . -Filter "*test*.ps1" -File

# Other runners and validators
$oldTests += Get-ChildItem -Path . -Filter "run-*.ps1" -File
$oldTests += Get-ChildItem -Path . -Filter "check-*.ps1" -File

Write-Host "Found $($oldTests.Count) test scripts to migrate:"
$oldTests | ForEach-Object { Write-Host "  - $($_.Name)" }
```

### Step 2: Categorize Tests

Determine where each test belongs in the new structure:

| Old Location                 | Test Type   | New Location                                   |
| ---------------------------- | ----------- | ---------------------------------------------- |
| `test-mistralrs.ps1`         | Integration | `tests/integration/test-mistralrs.ps1`         |
| `test-mcp-servers.ps1`       | MCP         | `tests/mcp/test-mcp-servers.ps1`               |
| `test-phase1-completion.ps1` | Integration | `tests/integration/test-phase1-completion.ps1` |
| `run-tui-test.ps1`           | Integration | `tests/integration/run-tui-test.ps1`           |
| `test-optimized-build.ps1`   | Build       | `scripts/build/test-optimized-build.ps1`       |
| `check-binary.ps1`           | Integration | `tests/integration/test-binary-health.ps1`     |

### Step 3: Update Test Scripts

#### Update Paths

Old script paths need updating:

```powershell
# Old path references
$binary = ".\target\release\mistralrs-server.exe"
$config = ".\MCP_CONFIG.json"
$results = ".\.testlogs\results.json"

# New path references
$binary = "target\release\mistralrs-server.exe"  # No .\ prefix
$config = "tests\mcp\MCP_CONFIG.json"           # Moved to tests/mcp
$results = "tests\results\test-name.json"        # Centralized results
```

#### Update Result Output

Old result handling:

```powershell
# Old: Results scattered in root
$results | ConvertTo-Json | Out-File "TEST_RESULTS.json"
```

New result handling:

```powershell
# New: Results in tests/results with timestamp
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$results | ConvertTo-Json | Out-File "tests/results/test-name-$timestamp.json"
```

#### Update Script Headers

Add proper documentation headers:

```powershell
<#
.SYNOPSIS
    Brief description of test purpose

.DESCRIPTION
    Detailed explanation of what this test validates

.PARAMETER Verbose
    Enable verbose output

.PARAMETER FailFast
    Stop on first failure

.EXAMPLE
    .\test-example.ps1 -Verbose

.NOTES
    Migrated from: old-test-name.ps1
    Migration Date: 2025-01-XX
#>
```

### Step 4: Integrate with Master Runner

Register tests with the master test runner:

1. **Ensure proper naming**: Test scripts should follow `test-*.ps1` pattern
1. **Place in correct directory**: Based on categorization
1. **Test discovery**: The runner auto-discovers based on location

```powershell
# The master runner will automatically find:
# - tests/integration/*.ps1
# - tests/mcp/test-*.ps1
# - scripts/build/test-*.ps1
```

### Step 5: Update CI/CD References

Update any CI/CD workflows that reference old test locations:

#### Old GitHub Actions

```yaml
# Old reference
- run: pwsh -File test-mistralrs.ps1
```

#### New GitHub Actions

```yaml
# New reference using master runner
- run: pwsh -File tests/run-all-tests.ps1 -Suite all -CI
```

### Step 6: Migrate Test Data

Move test data and configurations:

```powershell
# Create new directories if needed
New-Item -ItemType Directory -Path "tests\testdata" -Force
New-Item -ItemType Directory -Path "tests\mcp" -Force

# Move configurations
Move-Item "MCP_CONFIG.json" "tests\mcp\MCP_CONFIG.json"
Move-Item "MODEL_INVENTORY.json" "docs\MODEL_INVENTORY.json"

# Move test data
Move-Item ".testlogs\*" "tests\results\" -Force
```

## Backward Compatibility

### Compatibility Shims

During migration, maintain backward compatibility with shims:

**File**: `test-mistralrs.ps1` (root - temporary shim)

```powershell
<#
.SYNOPSIS
    DEPRECATED: Backward compatibility shim
    This script will be removed in v0.7.0
#>

Write-Warning "This script location is deprecated. Use tests/integration/test-mistralrs.ps1"
Write-Warning "Or better: Use tests/run-all-tests.ps1 -Suite integration"

# Forward to new location
& "tests\integration\test-mistralrs.ps1" @args
```

### Environment Variables

Support both old and new environment variable names:

```powershell
# Support both old and new variable names
$modelPath = $env:TEST_MODEL_PATH ?? $env:MODEL_PATH ?? $env:MISTRALRS_MODEL_PATH

# Warning for deprecated usage
if ($env:MODEL_PATH) {
    Write-Warning "MODEL_PATH is deprecated. Use TEST_MODEL_PATH instead."
}
```

## Feature Comparison

### Old Testing Features

| Feature           | Implementation         | Limitations              |
| ----------------- | ---------------------- | ------------------------ |
| Test execution    | Individual script runs | No unified execution     |
| Result collection | Manual JSON files      | Scattered locations      |
| CI integration    | Direct script calls    | No abstraction layer     |
| MCP testing       | Standalone scripts     | No lifecycle management  |
| Reporting         | Basic console output   | No HTML/Markdown reports |

### New Testing Features

| Feature            | Implementation               | Benefits                 |
| ------------------ | ---------------------------- | ------------------------ |
| Test execution     | Master runner orchestration  | Single entry point       |
| Result collection  | Centralized in tests/results | Easy artifact management |
| CI integration     | Suite-based execution        | Flexible CI pipelines    |
| MCP testing        | Integrated lifecycle         | Automatic start/stop     |
| Reporting          | Multiple formats             | HTML, JSON, Markdown     |
| Parallel execution | Job-based parallelization    | Faster test runs         |
| Test discovery     | Auto-discovery by location   | No registration needed   |

## Migration Examples

### Example 1: Simple Test Migration

#### Old Test

```powershell
# check-binary.ps1 (in root)
$binary = ".\target\release\mistralrs-server.exe"

if (Test-Path $binary) {
    Write-Host "Binary found"
    & $binary --version
} else {
    Write-Error "Binary not found"
    exit 1
}
```

#### Migrated Test

```powershell
# tests/integration/test-binary-health.ps1
<#
.SYNOPSIS
    Validates mistralrs binary health and functionality
#>

param(
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"
$binary = "target\release\mistralrs-server.exe"

Write-Host "Testing binary health..." -ForegroundColor Cyan

# Pre-flight check
if (-not (Test-Path $binary)) {
    throw "Binary not found at: $binary"
}

# Version check
$version = & $binary --version 2>&1
if ($version -match "mistralrs-server (\d+\.\d+\.\d+)") {
    Write-Host "✓ Version: $($matches[1])" -ForegroundColor Green
} else {
    throw "Invalid version output: $version"
}

# Help check
$help = & $binary --help 2>&1
if ($help -match "USAGE:") {
    Write-Host "✓ Help command works" -ForegroundColor Green
} else {
    throw "Help command failed"
}

Write-Host "✓ Binary health check passed" -ForegroundColor Green
exit 0
```

### Example 2: MCP Test Migration

#### Old Test

```powershell
# run-mcp-tests.ps1 (in root)
Write-Host "Starting MCP tests"

# Manual server start
$memory = Start-Process npx -ArgumentList "@modelcontextprotocol/server-memory" -PassThru

# Run test
# ... test code ...

# Manual cleanup
Stop-Process -Id $memory.Id
```

#### Migrated Test

```powershell
# tests/mcp/test-mcp-servers.ps1
<#
.SYNOPSIS
    Tests MCP server integration
#>

param(
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

Write-Host "Testing MCP servers..." -ForegroundColor Cyan

# Load configuration
$config = Get-Content "tests\mcp\MCP_CONFIG.json" -Raw | ConvertFrom-Json

# Note: Server lifecycle is managed by master runner
# Just test the functionality

foreach ($server in $config.servers) {
    Write-Host "Testing $($server.name)..." -ForegroundColor Yellow

    # Validate server configuration
    if (-not $server.source.command) {
        throw "Invalid configuration for $($server.name)"
    }

    # Test-specific validation
    # ... test code ...

    Write-Host "✓ $($server.name) test passed" -ForegroundColor Green
}

exit 0
```

### Example 3: Using the New Master Runner

Replace multiple test script calls with suite execution:

#### Old Approach

```powershell
# Manual test execution
.\test-mistralrs.ps1
.\test-mcp-servers.ps1
.\test-phase1-completion.ps1
.\run-tui-test.ps1
```

#### New Approach

```powershell
# Single command runs all tests
.\tests\run-all-tests.ps1 -Suite all

# Or specific suites
.\tests\run-all-tests.ps1 -Suite integration
.\tests\run-all-tests.ps1 -Suite mcp

# With options
.\tests\run-all-tests.ps1 -Suite all -Verbose -OutputFormat html
```

## Deprecation Notices

### Deprecated in v0.6.0

The following will be removed in v0.7.0:

1. **Root-level test scripts**: All `test-*.ps1`, `run-*.ps1`, `check-*.ps1` in root
1. **Old result locations**: JSON/MD files in root directory
1. **`.testlogs/` directory**: Replaced by `tests/results/`
1. **Individual test runners**: Replaced by master runner

### Migration Warnings

Add warnings to deprecated scripts:

```powershell
Write-Warning @"
================================================================================
DEPRECATION WARNING: This script location is deprecated.

Old location: $(Get-Location)\$($MyInvocation.MyCommand.Name)
New location: tests\integration\$($MyInvocation.MyCommand.Name)

This script will be removed in v0.7.0.
Please update your scripts to use the new location or the master runner:
  .\tests\run-all-tests.ps1 -Suite all

================================================================================
"@
```

## Troubleshooting Migration

### Issue: "Test not found by master runner"

**Cause**: Test not in expected location or wrong naming

**Solution**:

```powershell
# Ensure test follows naming convention
Rename-Item "my-test.ps1" "test-my-functionality.ps1"

# Place in correct directory
Move-Item "test-my-functionality.ps1" "tests\integration\"
```

### Issue: "Path not found" errors

**Cause**: Relative paths changed with new structure

**Solution**:

```powershell
# Update relative paths
# Old: .\target\release\mistralrs-server.exe
# New: target\release\mistralrs-server.exe

# Or use absolute paths
$projectRoot = Split-Path -Parent $PSScriptRoot
$binary = Join-Path $projectRoot "target\release\mistralrs-server.exe"
```

### Issue: "Results not being collected"

**Cause**: Output going to old locations

**Solution**:

```powershell
# Update result output paths
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$resultPath = "tests\results\$testName-$timestamp.json"

# Ensure directory exists
New-Item -ItemType Directory -Path "tests\results" -Force
```

### Issue: "CI/CD pipeline failures"

**Cause**: Workflows using old test locations

**Solution**:

```yaml
# Update GitHub Actions workflow
- name: Run tests
  run: |
    # Old: pwsh -File test-mistralrs.ps1
    # New:
    pwsh -File tests/run-all-tests.ps1 -Suite all -CI
```

## Benefits of Migration

### For Developers

1. **Single entry point**: No need to remember multiple test scripts
1. **Consistent structure**: Easy to find and add tests
1. **Better reporting**: Multiple output formats
1. **Faster execution**: Parallel test support

### For CI/CD

1. **Simplified pipelines**: One command for all tests
1. **Better artifacts**: Centralized result collection
1. **Flexible suites**: Run specific test categories
1. **Improved caching**: Structured output paths

### For Maintenance

1. **Clear organization**: Tests grouped by type
1. **Easy discovery**: Auto-discovery by location
1. **Version control**: Better git history tracking
1. **Documentation**: Comprehensive guides

## Next Steps

After migration:

1. **Remove deprecated scripts**: Clean up root directory
1. **Update documentation**: Reference new locations
1. **Update team workflows**: Train on new structure
1. **Monitor CI/CD**: Ensure pipelines work correctly
1. **Optimize test execution**: Leverage parallel execution

## Getting Help

If you encounter issues during migration:

1. **Check existing examples**: Review migrated tests in `tests/`
1. **Run validation**: `.\tests\validate-test-runner.ps1`
1. **Review logs**: Check `tests/results/` for error details
1. **Consult documentation**: See testing guides in `docs/testing/`
1. **Open an issue**: Report problems with migration

## Summary

The new testing structure provides:

- ✅ **Centralized test management**
- ✅ **Unified execution through master runner**
- ✅ **Better CI/CD integration**
- ✅ **Comprehensive reporting**
- ✅ **Automatic test discovery**
- ✅ **MCP server lifecycle management**
- ✅ **Parallel execution support**

Migrate your tests today to take advantage of these improvements!

______________________________________________________________________

*Last Updated: 2025*
*Version: 1.0.0*
*Migration Guide for: mistral.rs v0.6.0+*
