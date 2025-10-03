#
# Test CI/CD Setup for mistral.rs
# Validates that all CI/CD components are properly configured
#
# Usage: .\scripts\setup\test-ci-setup.ps1
#

$ErrorActionPreference = "Continue"

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "Testing CI/CD Setup for mistral.rs" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

$RepoRoot = git rev-parse --show-toplevel 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Not in a git repository" -ForegroundColor Red
    exit 1
}
Set-Location $RepoRoot

$results = @{
    Passed = 0
    Failed = 0
    Warnings = 0
}

function Test-Component {
    param($Name, $Path, $Required = $true)

    Write-Host "Testing: $Name..." -ForegroundColor Gray

    if (Test-Path $Path) {
        Write-Host "  ✓ Found: $Path" -ForegroundColor Green
        $script:results.Passed++
        return $true
    }
    else {
        if ($Required) {
            Write-Host "  ✗ Missing: $Path" -ForegroundColor Red
            $script:results.Failed++
        }
        else {
            Write-Host "  ⚠ Missing (optional): $Path" -ForegroundColor Yellow
            $script:results.Warnings++
        }
        return $false
    }
}

function Test-FileContent {
    param($Name, $Path, $Pattern)

    Write-Host "Testing: $Name content..." -ForegroundColor Gray

    if (-not (Test-Path $Path)) {
        Write-Host "  ✗ File not found: $Path" -ForegroundColor Red
        $script:results.Failed++
        return $false
    }

    $content = Get-Content $Path -Raw
    if ($content -match $Pattern) {
        Write-Host "  ✓ Pattern found in $Name" -ForegroundColor Green
        $script:results.Passed++
        return $true
    }
    else {
        Write-Host "  ✗ Pattern not found in $Name" -ForegroundColor Red
        $script:results.Failed++
        return $false
    }
}

Write-Host "[1] GitHub Actions Workflows" -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

Test-Component "Rust CI/CD Workflow" ".github/workflows/rust-ci.yml"
Test-Component "MCP Validation Workflow" ".github/workflows/mcp-validation.yml"
Test-Component "PowerShell Tests Workflow" ".github/workflows/powershell-tests.yml"

Write-Host ""
Write-Host "[2] Git Hooks" -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

Test-Component "Pre-commit Hook (Bash)" ".githooks/pre-commit"
Test-Component "Pre-commit Hook (PowerShell)" ".githooks/pre-commit.ps1"
Test-Component "Pre-push Hook (Bash)" ".githooks/pre-push"
Test-Component "Pre-push Hook (PowerShell)" ".githooks/pre-push.ps1"
Test-Component "Commit Message Hook" ".githooks/commit-msg"

Write-Host ""
Write-Host "[3] Installation Scripts" -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

Test-Component "Hook Installation Script" "scripts/setup/install-git-hooks.ps1"

Write-Host ""
Write-Host "[4] Documentation" -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

Test-Component "CI/CD Guide" ".github/CI_CD_GUIDE.md"
Test-Component "Setup Summary" "CI_CD_SETUP_SUMMARY.md"

Write-Host ""
Write-Host "[5] Workflow Configuration" -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

# Check if workflows use Makefile targets
Test-FileContent "Rust CI uses Makefile" ".github/workflows/rust-ci.yml" "make (check|test|lint)"
Test-FileContent "MCP Validation uses proper triggers" ".github/workflows/mcp-validation.yml" "schedule:|cron:"
Test-FileContent "PowerShell Tests validate scripts" ".github/workflows/powershell-tests.yml" "PSScriptAnalyzer"

Write-Host ""
Write-Host "[6] Hook Configuration" -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

Test-FileContent "Pre-commit uses Makefile" ".githooks/pre-commit" "make (fmt|check|lint)"
Test-FileContent "Pre-push runs tests" ".githooks/pre-push" "make test"
Test-FileContent "Commit-msg validates format" ".githooks/commit-msg" "feat|fix|docs|style"

Write-Host ""
Write-Host "[7] Build System Integration" -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

if (Test-Path "Makefile") {
    Write-Host "Testing: Makefile targets..." -ForegroundColor Gray

    $makeContent = Get-Content "Makefile" -Raw

    $requiredTargets = @("check", "test", "fmt", "lint", "ci", "build")
    $foundTargets = 0

    foreach ($target in $requiredTargets) {
        if ($makeContent -match "^\.PHONY: $target|^$target:") {
            $foundTargets++
        }
    }

    if ($foundTargets -eq $requiredTargets.Count) {
        Write-Host "  ✓ All required Makefile targets found" -ForegroundColor Green
        $script:results.Passed++
    }
    else {
        Write-Host "  ⚠ Some Makefile targets missing ($foundTargets/$($requiredTargets.Count))" -ForegroundColor Yellow
        $script:results.Warnings++
    }
}
else {
    Write-Host "  ✗ Makefile not found!" -ForegroundColor Red
    $script:results.Failed++
}

Write-Host ""
Write-Host "[8] Dependencies" -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

# Check Git
Write-Host "Testing: Git..." -ForegroundColor Gray
if (Get-Command git -ErrorAction SilentlyContinue) {
    $gitVersion = git --version
    Write-Host "  ✓ Git available: $gitVersion" -ForegroundColor Green
    $script:results.Passed++
}
else {
    Write-Host "  ✗ Git not found" -ForegroundColor Red
    $script:results.Failed++
}

# Check Make (optional for Windows)
Write-Host "Testing: Make..." -ForegroundColor Gray
if (Get-Command make -ErrorAction SilentlyContinue) {
    Write-Host "  ✓ Make available" -ForegroundColor Green
    $script:results.Passed++
}
else {
    Write-Host "  ⚠ Make not found (install Git Bash or use WSL)" -ForegroundColor Yellow
    $script:results.Warnings++
}

# Check Bun (for MCP)
Write-Host "Testing: Bun..." -ForegroundColor Gray
if (Get-Command bun -ErrorAction SilentlyContinue) {
    $bunVersion = bun --version
    Write-Host "  ✓ Bun available: v$bunVersion" -ForegroundColor Green
    $script:results.Passed++
}
else {
    Write-Host "  ⚠ Bun not found (required for MCP servers)" -ForegroundColor Yellow
    $script:results.Warnings++
}

# Check Redis (for MCP RAG-Redis)
Write-Host "Testing: Redis..." -ForegroundColor Gray
try {
    $redisResponse = redis-cli ping 2>&1
    if ($redisResponse -eq "PONG") {
        Write-Host "  ✓ Redis available and responding" -ForegroundColor Green
        $script:results.Passed++
    }
    else {
        Write-Host "  ⚠ Redis not responding" -ForegroundColor Yellow
        $script:results.Warnings++
    }
}
catch {
    Write-Host "  ⚠ Redis not found (optional for RAG-Redis MCP)" -ForegroundColor Yellow
    $script:results.Warnings++
}

Write-Host ""
Write-Host "[9] Git Configuration" -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

# Check if hooks are installed in .git/hooks
Write-Host "Testing: Installed hooks in .git/hooks..." -ForegroundColor Gray

$installedHooks = @()
$hooksDir = ".git/hooks"

if (Test-Path $hooksDir) {
    $expectedHooks = @("pre-commit", "pre-push", "commit-msg")

    foreach ($hook in $expectedHooks) {
        $hookPath = Join-Path $hooksDir $hook
        if (Test-Path $hookPath) {
            $installedHooks += $hook
        }
    }

    if ($installedHooks.Count -eq $expectedHooks.Count) {
        Write-Host "  ✓ All hooks installed ($($installedHooks.Count)/$($expectedHooks.Count))" -ForegroundColor Green
        $script:results.Passed++
    }
    elseif ($installedHooks.Count -gt 0) {
        Write-Host "  ⚠ Some hooks installed ($($installedHooks.Count)/$($expectedHooks.Count))" -ForegroundColor Yellow
        Write-Host "    Installed: $($installedHooks -join ', ')" -ForegroundColor Gray
        Write-Host "    Run: .\scripts\setup\install-git-hooks.ps1" -ForegroundColor Gray
        $script:results.Warnings++
    }
    else {
        Write-Host "  ⚠ No hooks installed" -ForegroundColor Yellow
        Write-Host "    Run: .\scripts\setup\install-git-hooks.ps1" -ForegroundColor Gray
        $script:results.Warnings++
    }
}
else {
    Write-Host "  ✗ .git/hooks directory not found" -ForegroundColor Red
    $script:results.Failed++
}

Write-Host ""
Write-Host "[10] Validation Summary" -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

# Calculate totals
$total = $results.Passed + $results.Failed + $results.Warnings

Write-Host ""
Write-Host "Test Results:" -ForegroundColor Cyan
Write-Host "  Total Checks: $total" -ForegroundColor White
Write-Host "  ✓ Passed: $($results.Passed)" -ForegroundColor Green
Write-Host "  ✗ Failed: $($results.Failed)" -ForegroundColor Red
Write-Host "  ⚠ Warnings: $($results.Warnings)" -ForegroundColor Yellow
Write-Host ""

# Recommendations
if ($results.Failed -gt 0) {
    Write-Host "❌ CI/CD setup incomplete" -ForegroundColor Red
    Write-Host ""
    Write-Host "Action Required:" -ForegroundColor Yellow
    Write-Host "  1. Review failed checks above" -ForegroundColor Gray
    Write-Host "  2. Re-run installation scripts" -ForegroundColor Gray
    Write-Host "  3. Verify file permissions" -ForegroundColor Gray
    Write-Host ""
    exit 1
}
elseif ($results.Warnings -gt 0) {
    Write-Host "⚠ CI/CD setup complete with warnings" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Recommended Actions:" -ForegroundColor Yellow

    if (-not (Get-Command make -ErrorAction SilentlyContinue)) {
        Write-Host "  • Install Git Bash or use WSL for 'make' command" -ForegroundColor Gray
    }

    if (-not (Test-Path ".git/hooks/pre-commit")) {
        Write-Host "  • Run: .\scripts\setup\install-git-hooks.ps1" -ForegroundColor Gray
    }

    if (-not (Get-Command bun -ErrorAction SilentlyContinue)) {
        Write-Host "  • Install Bun for MCP server testing" -ForegroundColor Gray
    }

    Write-Host ""
}
else {
    Write-Host "✅ CI/CD setup is complete and ready!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Next Steps:" -ForegroundColor Cyan
    Write-Host "  1. Make a test commit to verify pre-commit hook" -ForegroundColor Gray
    Write-Host "  2. Run 'make ci' to simulate full CI pipeline" -ForegroundColor Gray
    Write-Host "  3. Push changes to trigger GitHub Actions" -ForegroundColor Gray
    Write-Host "  4. Monitor workflows in GitHub Actions tab" -ForegroundColor Gray
    Write-Host ""
}

Write-Host "Documentation:" -ForegroundColor Cyan
Write-Host "  • CI/CD Guide: .github/CI_CD_GUIDE.md" -ForegroundColor Gray
Write-Host "  • Setup Summary: CI_CD_SETUP_SUMMARY.md" -ForegroundColor Gray
Write-Host "  • Build Guide: .claude/CLAUDE.md" -ForegroundColor Gray
Write-Host ""

exit $(if ($results.Failed -gt 0) { 1 } else { 0 })
