#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Tags TODO/FIXME/XXX/HACK comments with @codex/@gemini for external review

.DESCRIPTION
    Scans Rust source files for TODO/FIXME/XXX/HACK comments and adds review annotations.
    Skips comments that already have @codex or @gemini tags.

.PARAMETER DryRun
    Show what would be changed without modifying files

.PARAMETER Pattern
    Additional patterns to search for (comma-separated)

.EXAMPLE
    .\tag-issues.ps1

.EXAMPLE
    .\tag-issues.ps1 -DryRun

.EXAMPLE
    .\tag-issues.ps1 -Pattern "URGENT,REVIEW"
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $false)]
    [switch]$DryRun,

    [Parameter(Mandatory = $false)]
    [string]$Pattern
)

$ErrorActionPreference = "Stop"

# Color output functions
function Write-Success { param([string]$msg) Write-Host "✓ $msg" -ForegroundColor Green }
function Write-Error { param([string]$msg) Write-Host "✗ $msg" -ForegroundColor Red }
function Write-Info { param([string]$msg) Write-Host "ℹ $msg" -ForegroundColor Cyan }
function Write-Warning { param([string]$msg) Write-Host "⚠ $msg" -ForegroundColor Yellow }
function Write-Header { param([string]$msg) Write-Host "`n========================================" -ForegroundColor Magenta; Write-Host $msg -ForegroundColor Magenta; Write-Host "========================================`n" -ForegroundColor Magenta }

# Get repository root
$repoRoot = git rev-parse --show-toplevel 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Error "Not in a git repository!"
    exit 1
}
Set-Location $repoRoot

Write-Header "Tagging outstanding issues for external review"

# Define search patterns
$defaultPatterns = @("TODO", "FIXME", "XXX", "HACK")
if ($Pattern) {
    $customPatterns = $Pattern -split ','
    $allPatterns = $defaultPatterns + $customPatterns
} else {
    $allPatterns = $defaultPatterns
}

Write-Info "Searching for patterns: $($allPatterns -join ', ')"

# Find all Rust source files (excluding target/ and .cargo/)
$rustFiles = Get-ChildItem -Path $repoRoot -Recurse -Filter "*.rs" -File |
    Where-Object {
        $_.FullName -notmatch '\\target\\' -and
        $_.FullName -notmatch '\\.cargo\\' -and
        $_.FullName -notmatch '\\\.git\\'
    }

Write-Info "Found $($rustFiles.Count) Rust source files to scan"

$totalChanges = 0
$changedFiles = @()

foreach ($file in $rustFiles) {
    $filePath = $file.FullName
    $relativePath = $filePath.Substring($repoRoot.Length + 1)

    $content = Get-Content -Path $filePath -Raw -Encoding UTF8
    if (-not $content) { continue }

    $modified = $false
    $lines = $content -split "`r?`n"
    $newLines = @()

    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = $lines[$i]
        $newLine = $line

        # Check if line contains any of our patterns
        foreach ($pattern in $allPatterns) {
            # Skip if already tagged with @codex or @gemini
            if ($line -match "@codex" -or $line -match "@gemini") {
                continue
            }

            # Match pattern in comments
            if ($line -match "//.*\b$pattern\b") {
                # Determine which tag to use (alternate between @codex and @gemini)
                $tag = if ($totalChanges % 2 -eq 0) { "@codex" } else { "@gemini" }

                # Add tag after the pattern
                $newLine = $line -replace "(\b$pattern\b)", "`$1 $tag"

                if ($newLine -ne $line) {
                    $modified = $true
                    $totalChanges++
                    $lineNum = $i + 1
                    Write-Info "  [$relativePath`:$lineNum] Tagged $pattern with $tag"
                }
                break  # Only tag once per line
            }
        }

        $newLines += $newLine
    }

    # Write changes if modified
    if ($modified) {
        $changedFiles += $relativePath

        if ($DryRun) {
            Write-Warning "  [DRY RUN] Would modify: $relativePath"
        } else {
            $newContent = $newLines -join "`n"
            [System.IO.File]::WriteAllText($filePath, $newContent, [System.Text.UTF8Encoding]::new($false))
            Write-Success "  Modified: $relativePath"
        }
    }
}

Write-Header "Tagging Summary"
Write-Info "Total tags added: $totalChanges"
Write-Info "Files modified: $($changedFiles.Count)"

if ($changedFiles.Count -gt 0) {
    Write-Info "`nModified files:"
    foreach ($file in $changedFiles) {
        Write-Info "  - $file"
    }
}

if ($DryRun) {
    Write-Warning "`n[DRY RUN] No files were actually modified"
    Write-Info "Run without -DryRun flag to apply changes"
} else {
    if ($totalChanges -gt 0) {
        Write-Success "`nSuccessfully tagged $totalChanges issue comments"
    } else {
        Write-Info "No new issues found to tag"
    }
}

exit 0
