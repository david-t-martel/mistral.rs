#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Phase 1: Remove duplicate profile definitions from child workspaces

.DESCRIPTION
    Removes [profile.*] sections from child Cargo.toml files.
    These are ignored by Cargo but cause confusion.
    Zero risk - profiles in child workspaces are never used.

.NOTES
    Author: WinUtils Optimization Team
    Date: 2025-01-30
    Safety: ZERO RISK - Child workspace profiles are ignored by Cargo
#>

param(
    [switch]$DryRun,
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"
$scriptRoot = Split-Path -Parent $PSScriptRoot

function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Message)
    Write-Host "[SUCCESS] $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[WARNING] $Message" -ForegroundColor Yellow
}

function Remove-ProfileSection {
    param(
        [string]$FilePath,
        [string]$Description
    )

    if (-not (Test-Path $FilePath)) {
        Write-Warning "File not found: $FilePath"
        return $false
    }

    Write-Info "Processing: $Description"
    Write-Info "  File: $FilePath"

    $content = Get-Content $FilePath -Raw
    $originalLength = $content.Length

    # Remove [profile.release] and its content
    $content = $content -replace '(?ms)\[profile\.release\][^\[]*', ''

    # Remove [profile.release-fast] and its content
    $content = $content -replace '(?ms)\[profile\.release-fast\][^\[]*', ''

    # Remove [profile.release-small] and its content
    $content = $content -replace '(?ms)\[profile\.release-small\][^\[]*', ''

    # Remove [profile.dev] and its content (but not in main workspace)
    if ($FilePath -notlike "*\winutils\Cargo.toml") {
        $content = $content -replace '(?ms)\[profile\.dev\][^\[]*', ''
    }

    # Remove [profile.bench] and its content (but not in main workspace or benchmarks)
    if ($FilePath -notlike "*\winutils\Cargo.toml" -and $FilePath -notlike "*\benchmarks\Cargo.toml") {
        $content = $content -replace '(?ms)\[profile\.bench\][^\[]*', ''
    }

    # Clean up extra newlines
    $content = $content -replace '\n\n\n+', "`n`n"
    $content = $content.TrimEnd() + "`n"

    $newLength = $content.Length
    $bytesRemoved = $originalLength - $newLength

    if ($bytesRemoved -gt 0) {
        Write-Success "  Removed $bytesRemoved bytes of duplicate profile definitions"

        if (-not $DryRun) {
            Set-Content -Path $FilePath -Value $content -NoNewline
            Write-Success "  File updated successfully"
        } else {
            Write-Warning "  [DRY RUN] Would update file"
        }
        return $true
    } else {
        Write-Info "  No profile sections found (already clean)"
        return $false
    }
}

function Backup-Files {
    param([string[]]$Files)

    $backupDir = Join-Path $scriptRoot "backup-phase1-$(Get-Date -Format 'yyyyMMdd-HHmmss')"
    New-Item -ItemType Directory -Path $backupDir -Force | Out-Null

    Write-Info "Creating backup in: $backupDir"

    foreach ($file in $Files) {
        if (Test-Path $file) {
            $relativePath = $file.Replace($scriptRoot + "\", "")
            $backupPath = Join-Path $backupDir $relativePath
            $backupParent = Split-Path -Parent $backupPath

            New-Item -ItemType Directory -Path $backupParent -Force | Out-Null
            Copy-Item $file $backupPath
        }
    }

    Write-Success "Backup created successfully"
    return $backupDir
}

# Main execution
Write-Host ""
Write-Host "============================================" -ForegroundColor Magenta
Write-Host "  Workspace Optimization - Phase 1" -ForegroundColor Magenta
Write-Host "  Profile Cleanup (Zero Risk)" -ForegroundColor Magenta
Write-Host "============================================" -ForegroundColor Magenta
Write-Host ""

if ($DryRun) {
    Write-Warning "DRY RUN MODE - No files will be modified"
    Write-Host ""
}

# Define files to process
$filesToProcess = @(
    @{
        Path = Join-Path $scriptRoot "coreutils\Cargo.toml"
        Description = "Child workspace (coreutils)"
    },
    @{
        Path = Join-Path $scriptRoot "derive-utils\Cargo.toml"
        Description = "Child workspace (derive-utils)"
    },
    @{
        Path = Join-Path $scriptRoot "where\Cargo.toml"
        Description = "Standalone crate (where)"
    },
    @{
        Path = Join-Path $scriptRoot "derive-utils\bash-wrapper\Cargo.toml"
        Description = "Shell wrapper (bash-wrapper)"
    },
    @{
        Path = Join-Path $scriptRoot "derive-utils\cmd-wrapper\Cargo.toml"
        Description = "Shell wrapper (cmd-wrapper)"
    },
    @{
        Path = Join-Path $scriptRoot "derive-utils\pwsh-wrapper\Cargo.toml"
        Description = "Shell wrapper (pwsh-wrapper)"
    }
)

# Create backup (unless dry run)
if (-not $DryRun) {
    $backupDir = Backup-Files -Files ($filesToProcess | ForEach-Object { $_.Path })
    Write-Host ""
}

# Process each file
$filesModified = 0
$totalFiles = $filesToProcess.Count

foreach ($file in $filesToProcess) {
    $modified = Remove-ProfileSection -FilePath $file.Path -Description $file.Description
    if ($modified) {
        $filesModified++
    }
    Write-Host ""
}

# Summary
Write-Host "============================================" -ForegroundColor Magenta
Write-Host "  Phase 1 Summary" -ForegroundColor Magenta
Write-Host "============================================" -ForegroundColor Magenta
Write-Host "Files processed: $totalFiles" -ForegroundColor Cyan
Write-Host "Files modified:  $filesModified" -ForegroundColor Green
Write-Host ""

if ($DryRun) {
    Write-Warning "DRY RUN completed - no files were actually modified"
    Write-Host "Run without -DryRun to apply changes"
} else {
    Write-Success "Phase 1 completed successfully!"
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Yellow
    Write-Host "  1. Review changes: git diff" -ForegroundColor White
    Write-Host "  2. Test build:     make clean && make release" -ForegroundColor White
    Write-Host "  3. Validate:       make validate-all-77" -ForegroundColor White
    Write-Host ""
    Write-Host "Backup location: $backupDir" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "To rollback:" -ForegroundColor Yellow
    Write-Host "  git checkout HEAD -- coreutils/Cargo.toml derive-utils/Cargo.toml ..." -ForegroundColor White
}

Write-Host ""
