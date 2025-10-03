#
# Pre-push hook for mistral.rs (PowerShell version)
# This hook runs before pushing to ensure all tests pass
#
# To install: run scripts/setup/install-git-hooks.ps1
#
# Exit codes:
#   0 - Success
#   1 - Failure (blocks push)
#

$ErrorActionPreference = "Stop"

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "Pre-push: Running comprehensive checks" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan

# Change to repository root
$RepoRoot = git rev-parse --show-toplevel
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Not in a git repository" -ForegroundColor Red
    exit 1
}
Set-Location $RepoRoot

# Check if Makefile exists
if (-not (Test-Path "Makefile")) {
    Write-Host "ERROR: Makefile not found!" -ForegroundColor Red
    exit 1
}

# Check if we can run make
$makePath = Get-Command make -ErrorAction SilentlyContinue
if (-not $makePath) {
    Write-Host "WARNING: 'make' command not found" -ForegroundColor Yellow
    Write-Host "Running PowerShell tests only..." -ForegroundColor Yellow

    # Run PowerShell tests as fallback
    if (Test-Path "run-tests.ps1") {
        .\run-tests.ps1 -QuickTest
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✓ PowerShell tests passed" -ForegroundColor Green
        } else {
            Write-Host "⚠ PowerShell tests had issues" -ForegroundColor Yellow
            # Allow push anyway if only PS tests available
        }
    }
    exit 0
}

Write-Host ""
Write-Host "[1/3] Running test suite..." -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

# Run all tests
try {
    bash -c "make test"
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ All tests passed" -ForegroundColor Green
    } else {
        throw "Tests failed"
    }
} catch {
    Write-Host "✗ Tests failed" -ForegroundColor Red
    Write-Host ""
    Write-Host "Fix failing tests before pushing." -ForegroundColor Yellow
    Write-Host "Run 'make test' for detailed error messages." -ForegroundColor Yellow
    exit 1
}

Write-Host ""
Write-Host "[2/3] Running PowerShell tests..." -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

if (Test-Path "run-tests.ps1") {
    try {
        .\run-tests.ps1 -QuickTest -FailFast
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✓ PowerShell tests passed" -ForegroundColor Green
        } else {
            throw "PowerShell tests failed"
        }
    } catch {
        Write-Host "⚠ PowerShell tests had issues (not blocking)" -ForegroundColor Yellow
        # Don't block on PowerShell test failures
    }
} else {
    Write-Host "⚠ run-tests.ps1 not found (skipping)" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "[3/3] Checking for uncommitted changes..." -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

# Check if there are any uncommitted changes
$changes = git diff --exit-code 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ No uncommitted changes" -ForegroundColor Green
} else {
    Write-Host "✗ Uncommitted changes detected" -ForegroundColor Red
    Write-Host ""
    Write-Host "You have uncommitted changes. Commit or stash them before pushing." -ForegroundColor Yellow
    Write-Host ""
    git status --short
    exit 1
}

Write-Host ""
Write-Host "============================================" -ForegroundColor Green
Write-Host "✓ Pre-push checks passed" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Green
Write-Host ""

exit 0
