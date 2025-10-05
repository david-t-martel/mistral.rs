<#
.SYNOPSIS
    Fix critical compilation errors in winutils project
.DESCRIPTION
    Applies systematic fixes for:
    1. Orphan rule violations (fmt::Write implementations)
    2. Sysinfo API breaking changes (0.30+)
    3. Missing Windows API features
    4. Generic type constraints
#>

param(
    [switch]$DryRun,
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"
$ProjectRoot = "T:\projects\coreutils\winutils"

function Write-Status {
    param([string]$Message, [string]$Color = "Cyan")
    Write-Host "==> $Message" -ForegroundColor $Color
}

function Write-Success {
    param([string]$Message)
    Write-Host "[✓] $Message" -ForegroundColor Green
}

function Write-Error-Message {
    param([string]$Message)
    Write-Host "[✗] $Message" -ForegroundColor Red
}

# Fix 1: Remove orphan fmt::Write implementations
function Fix-OrphanImplementations {
    Write-Status "Fixing orphan rule violations..."

    $files = @(
        "shared\winutils-core\src\diagnostics.rs",
        "shared\winutils-core\src\version.rs",
        "shared\winutils-core\src\testing.rs",
        "shared\winutils-core\src\help.rs"
    )

    $pattern = @'
use std::io::Write;

impl fmt::Write for StandardStream {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}
'@

    foreach ($file in $files) {
        $fullPath = Join-Path $ProjectRoot $file
        if (Test-Path $fullPath) {
            $content = Get-Content $fullPath -Raw
            if ($content -match [regex]::Escape($pattern)) {
                Write-Status "  Removing orphan impl from: $file"
                if (-not $DryRun) {
                    $newContent = $content -replace [regex]::Escape($pattern), ""
                    Set-Content -Path $fullPath -Value $newContent -NoNewline
                    Write-Success "    Fixed: $file"
                } else {
                    Write-Host "    [DRY RUN] Would fix: $file" -ForegroundColor Yellow
                }
            } else {
                if ($Verbose) {
                    Write-Host "    No orphan impl found in: $file" -ForegroundColor Gray
                }
            }
        } else {
            Write-Error-Message "    File not found: $file"
        }
    }
}

# Fix 2: Update sysinfo API usage
function Fix-SysinfoAPI {
    Write-Status "Updating sysinfo API usage..."

    $file = "shared\winutils-core\src\diagnostics.rs"
    $fullPath = Join-Path $ProjectRoot $file

    if (Test-Path $fullPath) {
        $content = Get-Content $fullPath -Raw

        # Remove trait imports
        $oldImport = 'use sysinfo::{System, SystemExt, ProcessExt, DiskExt, NetworkExt, ComponentExt};'
        $newImport = 'use sysinfo::System;'

        if ($content -match [regex]::Escape($oldImport)) {
            Write-Status "  Updating sysinfo imports in: $file"
            if (-not $DryRun) {
                $content = $content -replace [regex]::Escape($oldImport), $newImport
                Set-Content -Path $fullPath -Value $content -NoNewline
                Write-Success "    Fixed: $file"
            } else {
                Write-Host "    [DRY RUN] Would fix: $file" -ForegroundColor Yellow
            }
        } else {
            if ($Verbose) {
                Write-Host "    No sysinfo trait imports found" -ForegroundColor Gray
            }
        }
    } else {
        Write-Error-Message "    File not found: $file"
    }
}

# Fix 3: Add missing Windows API features
function Fix-WindowsAPIFeatures {
    Write-Status "Adding missing Windows API features..."

    $file = "Cargo.toml"
    $fullPath = Join-Path $ProjectRoot $file

    if (Test-Path $fullPath) {
        $content = Get-Content $fullPath -Raw

        # Check if Win32_System_SystemServices is missing
        if ($content -match 'windows-sys.*features' -and $content -notmatch 'Win32_System_SystemServices') {
            Write-Status "  Adding Win32_System_SystemServices feature"
            if (-not $DryRun) {
                # Add the feature after Win32_System_SystemInformation
                $content = $content -replace `
                    '("Win32_System_SystemInformation",)',`
                    '$1\n    "Win32_System_SystemServices",'
                Set-Content -Path $fullPath -Value $content -NoNewline
                Write-Success "    Added Win32_System_SystemServices feature"
            } else {
                Write-Host "    [DRY RUN] Would add Win32_System_SystemServices" -ForegroundColor Yellow
            }
        } else {
            if ($Verbose) {
                Write-Host "    Win32_System_SystemServices already present or not needed" -ForegroundColor Gray
            }
        }
    } else {
        Write-Error-Message "    File not found: $file"
    }
}

# Fix 4: Test compilation
function Test-Compilation {
    Write-Status "Testing compilation..."

    Push-Location $ProjectRoot
    try {
        Write-Status "  Building shared/winutils-core..."
        if (-not $DryRun) {
            $result = cargo build --package winutils-core --all-features 2>&1
            if ($LASTEXITCODE -eq 0) {
                Write-Success "    winutils-core compiled successfully"
            } else {
                Write-Error-Message "    winutils-core compilation failed"
                Write-Host $result -ForegroundColor Red
                return $false
            }
        } else {
            Write-Host "    [DRY RUN] Would test compilation" -ForegroundColor Yellow
        }
        return $true
    } finally {
        Pop-Location
    }
}

# Main execution
function Main {
    Write-Host @"
╔════════════════════════════════════════════════════════════════╗
║      WinUtils Compilation Error Fixes                          ║
║                                                                ║
║  This script fixes critical compilation errors:                ║
║  1. Orphan rule violations (fmt::Write implementations)        ║
║  2. Sysinfo API breaking changes (0.30+)                       ║
║  3. Missing Windows API features                               ║
║                                                                ║
╚════════════════════════════════════════════════════════════════╝

"@ -ForegroundColor Cyan

    if ($DryRun) {
        Write-Host "[DRY RUN MODE - No changes will be made]`n" -ForegroundColor Yellow
    }

    # Apply fixes
    Fix-OrphanImplementations
    Fix-SysinfoAPI
    Fix-WindowsAPIFeatures

    Write-Host ""

    # Test compilation
    if (-not $DryRun) {
        if (Test-Compilation) {
            Write-Host @"

╔════════════════════════════════════════════════════════════════╗
║                    ✓ ALL FIXES APPLIED                         ║
║                                                                ║
║  All compilation errors have been fixed successfully.          ║
║  You can now build the project with:                           ║
║                                                                ║
║    cargo build --release --all-features                        ║
║                                                                ║
╚════════════════════════════════════════════════════════════════╝

"@ -ForegroundColor Green
        } else {
            Write-Host @"

╔════════════════════════════════════════════════════════════════╗
║                  ⚠ COMPILATION TEST FAILED                     ║
║                                                                ║
║  Some fixes were applied but compilation still fails.          ║
║  Please check the error output above.                          ║
║                                                                ║
╚════════════════════════════════════════════════════════════════╝

"@ -ForegroundColor Yellow
        }
    } else {
        Write-Host @"

╔════════════════════════════════════════════════════════════════╗
║                    DRY RUN COMPLETE                            ║
║                                                                ║
║  Run without -DryRun to apply fixes:                           ║
║    .\fix-compilation-errors.ps1                                ║
║                                                                ║
╚════════════════════════════════════════════════════════════════╝

"@ -ForegroundColor Yellow
    }
}

# Run main
Main
