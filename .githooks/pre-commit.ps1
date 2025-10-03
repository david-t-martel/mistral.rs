#
# Pre-commit hook for mistral.rs (PowerShell version)
# This hook runs before each commit to ensure code quality
#
# To install: run scripts/setup/install-git-hooks.ps1
#
# Exit codes:
#   0 - Success
#   1 - Failure (blocks commit)
#

$ErrorActionPreference = "Stop"

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "Pre-commit: Running quality checks" -ForegroundColor Cyan
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
    Write-Host "This project requires Makefile-based builds." -ForegroundColor Red
    exit 1
}

# Check if we can run make (requires Git Bash or WSL)
$makePath = Get-Command make -ErrorAction SilentlyContinue
if (-not $makePath) {
    Write-Host "WARNING: 'make' command not found" -ForegroundColor Yellow
    Write-Host "Please install Git Bash or use WSL for pre-commit hooks" -ForegroundColor Yellow
    Write-Host "Skipping automated checks..." -ForegroundColor Yellow
    exit 0
}

Write-Host ""
Write-Host "[1/3] Formatting code..." -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

# Auto-format code using Makefile
try {
    bash -c "make fmt"
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ Code formatted successfully" -ForegroundColor Green
    } else {
        throw "Formatting failed"
    }
} catch {
    Write-Host "✗ Formatting failed" -ForegroundColor Red
    exit 1
}

# Stage formatted files
git add -u

Write-Host ""
Write-Host "[2/3] Quick compilation check..." -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

# Quick check to ensure code compiles
try {
    bash -c "make check"
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ Code compiles successfully" -ForegroundColor Green
    } else {
        throw "Compilation check failed"
    }
} catch {
    Write-Host "✗ Compilation check failed" -ForegroundColor Red
    Write-Host ""
    Write-Host "Fix compilation errors before committing." -ForegroundColor Yellow
    Write-Host "Run 'make check' for detailed error messages." -ForegroundColor Yellow
    exit 1
}

Write-Host ""
Write-Host "[3/3] Auto-fixing lint issues..." -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

# Try to auto-fix linting issues
try {
    bash -c "make lint-fix"
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ Lint issues fixed" -ForegroundColor Green

        # Stage any fixes made by clippy
        git add -u
    } else {
        throw "Lint fix failed"
    }
} catch {
    Write-Host "⚠ Some lint issues require manual fixes" -ForegroundColor Yellow
    Write-Host "Run 'make lint' to see remaining issues." -ForegroundColor Yellow
    # Don't block commit on lint warnings
}

Write-Host ""
Write-Host "============================================" -ForegroundColor Green
Write-Host "✓ Pre-commit checks passed" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Green
Write-Host ""

exit 0
