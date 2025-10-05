#Requires -Version 5.1
<#
.SYNOPSIS
    WinUtils Management and Monitoring Tool

.DESCRIPTION
    Interactive management tool for deployed WinUtils utilities with real-time
    health monitoring, performance tracking, and update capabilities.

.PARAMETER Action
    Management action: Monitor, Update, Switch, Clean, Report

.PARAMETER Continuous
    Enable continuous monitoring mode (refreshes every N seconds)

.PARAMETER RefreshInterval
    Refresh interval for continuous monitoring (default: 5 seconds)

.EXAMPLE
    .\WinUtils-Manager.ps1 -Action Monitor
    Display current deployment status and health

.EXAMPLE
    .\WinUtils-Manager.ps1 -Action Monitor -Continuous -RefreshInterval 10
    Continuous monitoring with 10-second refresh

.EXAMPLE
    .\WinUtils-Manager.ps1 -Action Switch -Mode Individual
    Switch deployment mode to Individual binaries

.AUTHOR
    Claude Code - Deployment Engineer

.VERSION
    1.0.0
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $false)]
    [ValidateSet("Monitor", "Update", "Switch", "Clean", "Report", "Interactive")]
    [string]$Action = "Interactive",

    [Parameter(Mandatory = $false)]
    [switch]$Continuous,

    [Parameter(Mandatory = $false)]
    [int]$RefreshInterval = 5,

    [Parameter(Mandatory = $false)]
    [ValidateSet("Individual", "Monolithic", "Hybrid")]
    [string]$Mode,

    [Parameter(Mandatory = $false)]
    [string]$OutputPath = "T:\projects\coreutils\reports"
)

$ErrorActionPreference = 'Stop'

# Configuration
$script:Config = @{
    TargetDir = "C:\users\david\.local\bin"
    SourceDir = "T:\projects\coreutils\winutils\target\x86_64-pc-windows-msvc\release"
    LogDir = "T:\projects\coreutils\logs"
    MonolithicBinary = "coreutils.exe"
}

# ============================================================================
# Display Functions
# ============================================================================

function Show-Header {
    param([string]$Title)

    Clear-Host
    Write-Host "╔════════════════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║  $($Title.PadRight(74)) ║" -ForegroundColor Cyan
    Write-Host "╚════════════════════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
    Write-Host ""
}

function Show-DeploymentDashboard {
    Show-Header "WinUtils Deployment Dashboard"

    # Gather metrics
    $deployed = Get-ChildItem -Path $script:Config.TargetDir -Filter "*.exe" -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -ne $script:Config.MonolithicBinary -and $_.Name -notlike "uu-*.exe" }

    $symlinks = Get-ChildItem -Path $script:Config.TargetDir -Filter "uu-*.exe" -ErrorAction SilentlyContinue

    $coreutilsPath = Join-Path $script:Config.TargetDir $script:Config.MonolithicBinary
    $monolithicExists = Test-Path $coreutilsPath

    $totalSize = ($deployed | Measure-Object -Property Length -Sum).Sum
    $totalSizeMB = [math]::Round($totalSize / 1MB, 2)

    # Display metrics
    Write-Host "┌─ Deployment Overview ──────────────────────────────────────────────────┐" -ForegroundColor Green
    Write-Host "│" -ForegroundColor Green -NoNewline
    Write-Host "  Individual Utilities: " -NoNewline
    Write-Host "$($deployed.Count)".PadRight(55) -ForegroundColor Yellow -NoNewline
    Write-Host "│" -ForegroundColor Green

    Write-Host "│" -ForegroundColor Green -NoNewline
    Write-Host "  Symlinks (uu-*): " -NoNewline
    Write-Host "$($symlinks.Count)".PadRight(60) -ForegroundColor Yellow -NoNewline
    Write-Host "│" -ForegroundColor Green

    Write-Host "│" -ForegroundColor Green -NoNewline
    Write-Host "  Monolithic Binary: " -NoNewline
    $status = if ($monolithicExists) { "Deployed" } else { "Not Deployed" }
    $color = if ($monolithicExists) { "Green" } else { "Red" }
    Write-Host "$status".PadRight(58) -ForegroundColor $color -NoNewline
    Write-Host "│" -ForegroundColor Green

    Write-Host "│" -ForegroundColor Green -NoNewline
    Write-Host "  Total Disk Usage: " -NoNewline
    Write-Host "${totalSizeMB} MB".PadRight(59) -ForegroundColor Cyan -NoNewline
    Write-Host "│" -ForegroundColor Green

    Write-Host "└────────────────────────────────────────────────────────────────────────┘" -ForegroundColor Green
    Write-Host ""

    # Health checks
    Write-Host "┌─ Health Status ────────────────────────────────────────────────────────┐" -ForegroundColor Cyan

    $healthy = 0
    $unhealthy = 0
    $sampleSize = [math]::Min(5, $deployed.Count)
    $samples = $deployed | Get-Random -Count $sampleSize

    foreach ($util in $samples) {
        $health = Test-UtilityHealth -Path $util.FullName -Name $util.BaseName

        Write-Host "│" -ForegroundColor Cyan -NoNewline
        if ($health.Healthy) {
            Write-Host "  ✓ " -ForegroundColor Green -NoNewline
            $healthy++
        }
        else {
            Write-Host "  ✗ " -ForegroundColor Red -NoNewline
            $unhealthy++
        }
        Write-Host "$($util.BaseName)".PadRight(70) -NoNewline
        Write-Host "│" -ForegroundColor Cyan
    }

    if ($deployed.Count -gt $sampleSize) {
        Write-Host "│" -ForegroundColor Cyan -NoNewline
        Write-Host "  ... and $($deployed.Count - $sampleSize) more utilities".PadRight(74) -ForegroundColor Gray -NoNewline
        Write-Host "│" -ForegroundColor Cyan
    }

    Write-Host "└────────────────────────────────────────────────────────────────────────┘" -ForegroundColor Cyan
    Write-Host ""

    # Performance metrics
    Show-PerformanceMetrics -Utilities ($samples | Select-Object -First 3)

    # Timestamp
    Write-Host ""
    Write-Host "Last updated: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" -ForegroundColor Gray
}

function Test-UtilityHealth {
    param(
        [string]$Path,
        [string]$Name
    )

    try {
        $output = & $Path --version 2>&1 | Select-Object -First 1
        return @{
            Healthy = $true
            Message = "OK"
        }
    }
    catch {
        return @{
            Healthy = $false
            Message = $_.Exception.Message
        }
    }
}

function Show-PerformanceMetrics {
    param([object[]]$Utilities)

    if ($Utilities.Count -eq 0) {
        return
    }

    Write-Host "┌─ Quick Performance Check ──────────────────────────────────────────────┐" -ForegroundColor Yellow

    foreach ($util in $Utilities) {
        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

        try {
            & $util.FullName --version 2>&1 | Out-Null
            $stopwatch.Stop()
            $ms = $stopwatch.ElapsedMilliseconds

            Write-Host "│" -ForegroundColor Yellow -NoNewline
            Write-Host "  $($util.BaseName.PadRight(20))" -NoNewline

            $color = if ($ms -lt 10) { "Green" }
                    elseif ($ms -lt 50) { "Cyan" }
                    elseif ($ms -lt 200) { "Yellow" }
                    else { "Red" }

            Write-Host " ${ms}ms".PadRight(53) -ForegroundColor $color -NoNewline
            Write-Host "│" -ForegroundColor Yellow
        }
        catch {
            Write-Host "│" -ForegroundColor Yellow -NoNewline
            Write-Host "  $($util.BaseName.PadRight(20)) " -NoNewline
            Write-Host "FAILED".PadRight(52) -ForegroundColor Red -NoNewline
            Write-Host "│" -ForegroundColor Yellow
        }
    }

    Write-Host "└────────────────────────────────────────────────────────────────────────┘" -ForegroundColor Yellow
}

# ============================================================================
# Management Actions
# ============================================================================

function Invoke-UpdateCheck {
    Show-Header "Update Check"

    Write-Host "Checking for updates..." -ForegroundColor Cyan

    $deployed = Get-ChildItem -Path $script:Config.TargetDir -Filter "*.exe" -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -ne $script:Config.MonolithicBinary }

    $updates = @()

    foreach ($util in $deployed) {
        $sourcePath = Join-Path $script:Config.SourceDir $util.Name

        if (Test-Path $sourcePath) {
            $sourceFile = Get-Item $sourcePath
            $deployedFile = $util

            if ($sourceFile.LastWriteTime -gt $deployedFile.LastWriteTime) {
                $updates += [PSCustomObject]@{
                    Name = $util.BaseName
                    CurrentVersion = $deployedFile.LastWriteTime
                    NewVersion = $sourceFile.LastWriteTime
                    SizeDiff = $sourceFile.Length - $deployedFile.Length
                }
            }
        }
    }

    if ($updates.Count -eq 0) {
        Write-Host ""
        Write-Host "✓ All utilities are up-to-date!" -ForegroundColor Green
    }
    else {
        Write-Host ""
        Write-Host "Updates available for $($updates.Count) utilities:" -ForegroundColor Yellow
        Write-Host ""

        $updates | Format-Table -AutoSize -Property Name, CurrentVersion, NewVersion, SizeDiff

        Write-Host ""
        $confirm = Read-Host "Apply updates? (Y/N)"

        if ($confirm -like "Y*") {
            Write-Host ""
            Write-Host "Applying updates..." -ForegroundColor Cyan

            foreach ($update in $updates) {
                $sourcePath = Join-Path $script:Config.SourceDir "$($update.Name).exe"
                $targetPath = Join-Path $script:Config.TargetDir "$($update.Name).exe"

                try {
                    Copy-Item -Path $sourcePath -Destination $targetPath -Force
                    Write-Host "  ✓ Updated: $($update.Name)" -ForegroundColor Green
                }
                catch {
                    Write-Host "  ✗ Failed: $($update.Name) - $_" -ForegroundColor Red
                }
            }

            Write-Host ""
            Write-Host "Update completed!" -ForegroundColor Green
        }
    }

    Write-Host ""
    Read-Host "Press Enter to continue"
}

function Invoke-DeploymentSwitch {
    param([string]$TargetMode)

    Show-Header "Switch Deployment Mode"

    Write-Host "Current mode detection..." -ForegroundColor Cyan
    Write-Host ""

    $individualCount = (Get-ChildItem -Path $script:Config.TargetDir -Filter "*.exe" |
                       Where-Object { $_.Name -ne $script:Config.MonolithicBinary -and $_.Name -notlike "uu-*.exe" }).Count

    $monolithicExists = Test-Path (Join-Path $script:Config.TargetDir $script:Config.MonolithicBinary)

    $currentMode = if ($individualCount -gt 0 -and $monolithicExists) { "Hybrid" }
                   elseif ($individualCount -gt 0) { "Individual" }
                   elseif ($monolithicExists) { "Monolithic" }
                   else { "None" }

    Write-Host "Current mode: $currentMode" -ForegroundColor Yellow
    Write-Host "Target mode: $TargetMode" -ForegroundColor Green
    Write-Host ""

    if ($currentMode -eq $TargetMode) {
        Write-Host "Already in $TargetMode mode!" -ForegroundColor Yellow
        Read-Host "Press Enter to continue"
        return
    }

    $confirm = Read-Host "Proceed with mode switch? (Y/N)"

    if ($confirm -notlike "Y*") {
        Write-Host "Cancelled." -ForegroundColor Yellow
        Read-Host "Press Enter to continue"
        return
    }

    Write-Host ""
    Write-Host "Switching to $TargetMode mode..." -ForegroundColor Cyan

    # Call main deployment framework
    $frameworkPath = Join-Path (Split-Path $PSScriptRoot -Parent) "deploy\WinUtils-Deployment-Framework.ps1"

    if (Test-Path $frameworkPath) {
        & $frameworkPath -Action Deploy -Mode $TargetMode -Force
    }
    else {
        Write-Host "Error: Deployment framework not found!" -ForegroundColor Red
    }

    Write-Host ""
    Read-Host "Press Enter to continue"
}

function Invoke-Cleanup {
    Show-Header "Cleanup Deployment"

    Write-Host "This will remove orphaned symlinks and temporary files." -ForegroundColor Yellow
    Write-Host ""

    $confirm = Read-Host "Continue? (Y/N)"

    if ($confirm -notlike "Y*") {
        Write-Host "Cancelled." -ForegroundColor Yellow
        Read-Host "Press Enter to continue"
        return
    }

    Write-Host ""
    Write-Host "Cleaning up..." -ForegroundColor Cyan

    # Find orphaned symlinks (uu-* files with no corresponding base file)
    $symlinks = Get-ChildItem -Path $script:Config.TargetDir -Filter "uu-*.exe" -ErrorAction SilentlyContinue
    $orphaned = @()

    foreach ($symlink in $symlinks) {
        $baseName = $symlink.Name -replace "^uu-", ""
        $basePath = Join-Path $script:Config.TargetDir $baseName

        if (-not (Test-Path $basePath)) {
            $orphaned += $symlink
        }
    }

    if ($orphaned.Count -gt 0) {
        Write-Host "Found $($orphaned.Count) orphaned symlinks:" -ForegroundColor Yellow
        foreach ($file in $orphaned) {
            Write-Host "  - $($file.Name)" -ForegroundColor Gray
            Remove-Item $file.FullName -Force
        }
        Write-Host ""
        Write-Host "✓ Removed $($orphaned.Count) orphaned symlinks" -ForegroundColor Green
    }
    else {
        Write-Host "✓ No orphaned symlinks found" -ForegroundColor Green
    }

    Write-Host ""
    Read-Host "Press Enter to continue"
}

function New-DeploymentReport {
    Show-Header "Generate Deployment Report"

    $reportPath = Join-Path $OutputPath "deployment-report-$(Get-Date -Format 'yyyyMMdd-HHmmss').html"

    # Ensure output directory exists
    if (-not (Test-Path $OutputPath)) {
        New-Item -ItemType Directory -Path $OutputPath -Force | Out-Null
    }

    Write-Host "Generating comprehensive deployment report..." -ForegroundColor Cyan
    Write-Host ""

    # Gather data
    $deployed = Get-ChildItem -Path $script:Config.TargetDir -Filter "*.exe" -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -ne $script:Config.MonolithicBinary -and $_.Name -notlike "uu-*.exe" }

    $report = @"
<!DOCTYPE html>
<html>
<head>
    <title>WinUtils Deployment Report</title>
    <style>
        body { font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; margin: 40px; background: #f5f5f5; }
        h1 { color: #0066cc; border-bottom: 3px solid #0066cc; padding-bottom: 10px; }
        h2 { color: #333; margin-top: 30px; }
        table { border-collapse: collapse; width: 100%; background: white; margin: 20px 0; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        th { background: #0066cc; color: white; padding: 12px; text-align: left; }
        td { padding: 10px; border-bottom: 1px solid #ddd; }
        tr:hover { background: #f9f9f9; }
        .metric { display: inline-block; background: white; padding: 20px; margin: 10px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .metric-value { font-size: 36px; font-weight: bold; color: #0066cc; }
        .metric-label { font-size: 14px; color: #666; margin-top: 5px; }
        .healthy { color: green; font-weight: bold; }
        .unhealthy { color: red; font-weight: bold; }
    </style>
</head>
<body>
    <h1>WinUtils Deployment Report</h1>
    <p>Generated: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")</p>

    <h2>Deployment Metrics</h2>
    <div>
        <div class="metric">
            <div class="metric-value">$($deployed.Count)</div>
            <div class="metric-label">Deployed Utilities</div>
        </div>
        <div class="metric">
            <div class="metric-value">$([math]::Round(($deployed | Measure-Object -Property Length -Sum).Sum / 1MB, 2)) MB</div>
            <div class="metric-label">Total Size</div>
        </div>
    </div>

    <h2>Deployed Utilities</h2>
    <table>
        <tr>
            <th>Utility</th>
            <th>Size</th>
            <th>Last Modified</th>
            <th>Health</th>
        </tr>
"@

    foreach ($util in $deployed | Sort-Object Name) {
        $health = Test-UtilityHealth -Path $util.FullName -Name $util.BaseName
        $healthClass = if ($health.Healthy) { "healthy" } else { "unhealthy" }
        $healthText = if ($health.Healthy) { "✓ Healthy" } else { "✗ Unhealthy" }

        $report += @"
        <tr>
            <td>$($util.BaseName)</td>
            <td>$([math]::Round($util.Length / 1KB, 2)) KB</td>
            <td>$($util.LastWriteTime.ToString("yyyy-MM-dd HH:mm:ss"))</td>
            <td class="$healthClass">$healthText</td>
        </tr>
"@
    }

    $report += @"
    </table>
</body>
</html>
"@

    $report | Set-Content $reportPath -Encoding UTF8

    Write-Host "✓ Report generated: $reportPath" -ForegroundColor Green
    Write-Host ""

    $openReport = Read-Host "Open report in browser? (Y/N)"
    if ($openReport -like "Y*") {
        Start-Process $reportPath
    }

    Write-Host ""
    Read-Host "Press Enter to continue"
}

# ============================================================================
# Interactive Menu
# ============================================================================

function Show-InteractiveMenu {
    while ($true) {
        Show-DeploymentDashboard

        Write-Host ""
        Write-Host "═══════════════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
        Write-Host "  Actions:" -ForegroundColor Cyan
        Write-Host "  [1] Update Check       [2] Switch Mode        [3] Cleanup" -ForegroundColor White
        Write-Host "  [4] Generate Report    [5] Refresh            [Q] Quit" -ForegroundColor White
        Write-Host "═══════════════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
        Write-Host ""

        $choice = Read-Host "Select action"

        switch ($choice) {
            "1" { Invoke-UpdateCheck }
            "2" {
                Write-Host ""
                Write-Host "Select target mode:"
                Write-Host "  [1] Individual"
                Write-Host "  [2] Monolithic"
                Write-Host "  [3] Hybrid"
                Write-Host ""
                $modeChoice = Read-Host "Mode"
                $targetMode = switch ($modeChoice) {
                    "1" { "Individual" }
                    "2" { "Monolithic" }
                    "3" { "Hybrid" }
                    default { $null }
                }
                if ($targetMode) {
                    Invoke-DeploymentSwitch -TargetMode $targetMode
                }
            }
            "3" { Invoke-Cleanup }
            "4" { New-DeploymentReport }
            "5" { continue }
            "Q" { return }
            "q" { return }
            default { Write-Host "Invalid choice" -ForegroundColor Red; Start-Sleep -Seconds 1 }
        }
    }
}

# ============================================================================
# Main Entry Point
# ============================================================================

try {
    switch ($Action) {
        "Monitor" {
            if ($Continuous) {
                while ($true) {
                    Show-DeploymentDashboard
                    Start-Sleep -Seconds $RefreshInterval
                }
            }
            else {
                Show-DeploymentDashboard
                Write-Host ""
                Read-Host "Press Enter to exit"
            }
        }

        "Update" {
            Invoke-UpdateCheck
        }

        "Switch" {
            if (-not $Mode) {
                Write-Host "Error: -Mode parameter required for Switch action" -ForegroundColor Red
                exit 1
            }
            Invoke-DeploymentSwitch -TargetMode $Mode
        }

        "Clean" {
            Invoke-Cleanup
        }

        "Report" {
            New-DeploymentReport
        }

        "Interactive" {
            Show-InteractiveMenu
        }
    }
}
catch {
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}
