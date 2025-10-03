#
# Install Git Hooks for mistral.rs
# This script copies hook scripts to .git/hooks/ and makes them executable
#
# Usage: .\scripts\setup\install-git-hooks.ps1
#

$ErrorActionPreference = "Stop"

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "Installing Git Hooks for mistral.rs" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# Change to repository root
$RepoRoot = git rev-parse --show-toplevel 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Not in a git repository" -ForegroundColor Red
    exit 1
}
Set-Location $RepoRoot

# Check if .githooks directory exists
$HooksSourceDir = Join-Path $RepoRoot ".githooks"
if (-not (Test-Path $HooksSourceDir)) {
    Write-Host "ERROR: .githooks directory not found!" -ForegroundColor Red
    Write-Host "Expected at: $HooksSourceDir" -ForegroundColor Red
    exit 1
}

# Check if .git/hooks directory exists
$HooksTargetDir = Join-Path $RepoRoot ".git\hooks"
if (-not (Test-Path $HooksTargetDir)) {
    Write-Host "ERROR: .git/hooks directory not found!" -ForegroundColor Red
    Write-Host "This doesn't appear to be a git repository." -ForegroundColor Red
    exit 1
}

# Verify Makefile exists
if (-not (Test-Path (Join-Path $RepoRoot "Makefile"))) {
    Write-Host "ERROR: Makefile not found!" -ForegroundColor Red
    Write-Host "Git hooks require Makefile-based build system." -ForegroundColor Red
    exit 1
}

Write-Host "Repository root: $RepoRoot" -ForegroundColor Gray
Write-Host "Hooks source: $HooksSourceDir" -ForegroundColor Gray
Write-Host "Hooks target: $HooksTargetDir" -ForegroundColor Gray
Write-Host ""

# Hooks to install
$hooks = @(
    @{Name="pre-commit"; Description="Format, check, and lint code"},
    @{Name="pre-push"; Description="Run tests before pushing"},
    @{Name="commit-msg"; Description="Validate commit message format"}
)

$installed = 0
$skipped = 0
$errors = 0

foreach ($hook in $hooks) {
    $hookName = $hook.Name
    $hookDesc = $hook.Description

    Write-Host "Installing $hookName..." -ForegroundColor Yellow
    Write-Host "  Purpose: $hookDesc" -ForegroundColor Gray

    # Try both bash and PowerShell versions
    $sourceBash = Join-Path $HooksSourceDir $hookName
    $sourcePS = Join-Path $HooksSourceDir "$hookName.ps1"
    $target = Join-Path $HooksTargetDir $hookName

    # Backup existing hook
    if (Test-Path $target) {
        $backup = "$target.backup.$(Get-Date -Format 'yyyyMMdd-HHmmss')"
        Write-Host "  Backing up existing hook to: $backup" -ForegroundColor Gray
        Move-Item -Path $target -Destination $backup -Force
    }

    # Determine which version to install
    $installed_version = $null

    # On Windows, prefer PowerShell version if Git Bash not available
    if ($IsWindows -or $env:OS -match "Windows") {
        if (Test-Path $sourcePS) {
            # Create wrapper that calls PowerShell script
            $wrapperContent = @"
#!/usr/bin/env bash
# Git hook wrapper for Windows PowerShell
# Calls the PowerShell version of the hook

HOOK_NAME="$hookName"
HOOK_DIR="`$(git rev-parse --show-toplevel)/.githooks"

if command -v pwsh &> /dev/null; then
    pwsh -NoProfile -File "`$HOOK_DIR/`$HOOK_NAME.ps1" "`$@"
    exit `$?
elif command -v powershell &> /dev/null; then
    powershell -NoProfile -File "`$HOOK_DIR/`$HOOK_NAME.ps1" "`$@"
    exit `$?
else
    # Fallback to bash version if PowerShell not available
    if [ -f "`$HOOK_DIR/`$HOOK_NAME" ]; then
        bash "`$HOOK_DIR/`$HOOK_NAME" "`$@"
        exit `$?
    fi
    echo "ERROR: Neither PowerShell nor bash version available"
    exit 1
fi
"@
            Set-Content -Path $target -Value $wrapperContent -Force
            $installed_version = "PowerShell (via wrapper)"
        }
        elseif (Test-Path $sourceBash) {
            Copy-Item -Path $sourceBash -Destination $target -Force
            $installed_version = "Bash"
        }
    }
    else {
        # On Linux/macOS, prefer bash version
        if (Test-Path $sourceBash) {
            Copy-Item -Path $sourceBash -Destination $target -Force
            $installed_version = "Bash"
        }
        elseif (Test-Path $sourcePS) {
            # Create wrapper for PowerShell on Unix
            $wrapperContent = @"
#!/usr/bin/env bash
HOOK_DIR="`$(git rev-parse --show-toplevel)/.githooks"
pwsh -NoProfile -File "`$HOOK_DIR/$hookName.ps1" "`$@"
"@
            Set-Content -Path $target -Value $wrapperContent -Force
            $installed_version = "PowerShell (via wrapper)"
        }
    }

    if ($installed_version) {
        # Make executable (Unix-style)
        if (Get-Command chmod -ErrorAction SilentlyContinue) {
            chmod +x $target
        }

        Write-Host "  ✓ Installed ($installed_version)" -ForegroundColor Green
        $installed++
    }
    else {
        Write-Host "  ✗ Source hook not found" -ForegroundColor Red
        $errors++
    }

    Write-Host ""
}

# Summary
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "Installation Summary" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  Installed: $installed" -ForegroundColor Green
if ($skipped -gt 0) {
    Write-Host "  Skipped: $skipped" -ForegroundColor Yellow
}
if ($errors -gt 0) {
    Write-Host "  Errors: $errors" -ForegroundColor Red
}
Write-Host ""

# Test hooks
Write-Host "Testing hook installation..." -ForegroundColor Yellow
Write-Host ""

$testPassed = 0
$testFailed = 0

foreach ($hook in $hooks) {
    $hookPath = Join-Path $HooksTargetDir $hook.Name

    if (Test-Path $hookPath) {
        Write-Host "  Testing $($hook.Name)..." -ForegroundColor Gray

        # Try to execute hook with --help or similar
        try {
            # For commit-msg hook, create a dummy message file
            if ($hook.Name -eq "commit-msg") {
                $tempMsg = New-TemporaryFile
                "feat(test): test commit message" | Out-File $tempMsg

                if (Get-Command bash -ErrorAction SilentlyContinue) {
                    bash $hookPath $tempMsg.FullName 2>&1 | Out-Null
                }

                Remove-Item $tempMsg
            }

            Write-Host "    ✓ Hook is executable" -ForegroundColor Green
            $testPassed++
        }
        catch {
            Write-Host "    ⚠ Hook may not be properly configured" -ForegroundColor Yellow
            $testFailed++
        }
    }
    else {
        Write-Host "  ✗ $($hook.Name) not found" -ForegroundColor Red
        $testFailed++
    }
}

Write-Host ""

if ($errors -gt 0 -or $testFailed -gt 0) {
    Write-Host "⚠ Installation completed with issues" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Troubleshooting:" -ForegroundColor Yellow
    Write-Host "  1. Ensure Git Bash or PowerShell is available" -ForegroundColor Gray
    Write-Host "  2. Check that .githooks/ directory contains hook scripts" -ForegroundColor Gray
    Write-Host "  3. Verify Makefile exists in repository root" -ForegroundColor Gray
    Write-Host ""
}
else {
    Write-Host "✓ Git hooks installed successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Hooks installed:" -ForegroundColor Gray
    foreach ($hook in $hooks) {
        Write-Host "  • $($hook.Name): $($hook.Description)" -ForegroundColor Gray
    }
    Write-Host ""
    Write-Host "These hooks will run automatically on:" -ForegroundColor Gray
    Write-Host "  • git commit  (pre-commit, commit-msg)" -ForegroundColor Gray
    Write-Host "  • git push    (pre-push)" -ForegroundColor Gray
    Write-Host ""
}

Write-Host "To bypass hooks (not recommended):" -ForegroundColor Yellow
Write-Host "  git commit --no-verify" -ForegroundColor Gray
Write-Host "  git push --no-verify" -ForegroundColor Gray
Write-Host ""

exit 0
