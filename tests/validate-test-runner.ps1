<#
.SYNOPSIS
    Validates that the master test runner script is properly configured

.DESCRIPTION
    Performs syntax checking and basic validation of run-all-tests.ps1
#>

$ErrorActionPreference = "Stop"

Write-Host "Validating master test runner..." -ForegroundColor Cyan

# Check script exists
$scriptPath = "tests/run-all-tests.ps1"
if (-not (Test-Path $scriptPath)) {
    Write-Host "ERROR: Script not found: $scriptPath" -ForegroundColor Red
    exit 1
}
Write-Host "✓ Script exists: $scriptPath" -ForegroundColor Green

# Syntax validation
try {
    $null = [System.Management.Automation.PSParser]::Tokenize((Get-Content $scriptPath -Raw), [ref]$null)
    Write-Host "✓ Syntax validation passed" -ForegroundColor Green
} catch {
    Write-Host "✗ Syntax error: $_" -ForegroundColor Red
    exit 1
}

# Check parameter validation
try {
    Get-Command $scriptPath -ErrorAction Stop | Out-Null
    Write-Host "✓ Parameters validated" -ForegroundColor Green
} catch {
    Write-Host "✗ Parameter validation failed: $_" -ForegroundColor Red
    exit 1
}

# Check required functions exist
$requiredFunctions = @(
    "Test-PreFlightChecks",
    "Find-TestScripts",
    "Invoke-TestScript",
    "Start-MCPServers",
    "Stop-MCPServers",
    "Export-Results"
)

$content = Get-Content $scriptPath -Raw
$missingFunctions = @()

foreach ($func in $requiredFunctions) {
    if ($content -notmatch "function $func") {
        $missingFunctions += $func
    }
}

if ($missingFunctions.Count -gt 0) {
    Write-Host "✗ Missing required functions:" -ForegroundColor Red
    $missingFunctions | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
} else {
    Write-Host "✓ All required functions present" -ForegroundColor Green
}

# Verify test directory structure
$requiredDirs = @(
    "tests",
    "tests/results",
    "tests/integration",
    "tests/mcp"
)

foreach ($dir in $requiredDirs) {
    if (Test-Path $dir) {
        Write-Host "✓ Directory exists: $dir" -ForegroundColor Green
    } else {
        Write-Host "⚠ Directory missing (will be created): $dir" -ForegroundColor Yellow
    }
}

# Check for test scripts
$testCategories = @{
    "Integration" = "tests/integration/*.ps1"
    "MCP" = "tests/mcp/test-*.ps1"
}

foreach ($category in $testCategories.GetEnumerator()) {
    $scripts = @(Get-ChildItem -Path $category.Value -ErrorAction SilentlyContinue)
    if ($scripts.Count -gt 0) {
        Write-Host "✓ Found $($scripts.Count) $($category.Key) test(s)" -ForegroundColor Green
    } else {
        Write-Host "⚠ No $($category.Key) tests found in $($category.Value)" -ForegroundColor Yellow
    }
}

# Verify Makefile integration
if (Test-Path "Makefile") {
    $makefileContent = Get-Content "Makefile" -Raw
    $makeTargets = @("test-ps1", "test-ps1-quick", "test-full")

    $missingTargets = @()
    foreach ($target in $makeTargets) {
        if ($makefileContent -notmatch "\.PHONY: $target") {
            $missingTargets += $target
        }
    }

    if ($missingTargets.Count -eq 0) {
        Write-Host "✓ Makefile integration complete" -ForegroundColor Green
    } else {
        Write-Host "✗ Missing Makefile targets:" -ForegroundColor Red
        $missingTargets | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
        exit 1
    }
} else {
    Write-Host "⚠ Makefile not found" -ForegroundColor Yellow
}

Write-Host "`n✓ All validations passed!" -ForegroundColor Green
Write-Host "`nTo run tests:" -ForegroundColor Cyan
Write-Host "  make test-ps1-quick   # Quick smoke tests" -ForegroundColor White
Write-Host "  make test-ps1         # Full PowerShell suite" -ForegroundColor White
Write-Host "  make test-full        # All tests (Rust + PowerShell)" -ForegroundColor White

exit 0
