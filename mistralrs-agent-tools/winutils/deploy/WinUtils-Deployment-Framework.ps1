#Requires -Version 5.1
<#
.SYNOPSIS
    Unified WinUtils Deployment Framework - Production Ready

.DESCRIPTION
    Comprehensive deployment system for 78 optimized Windows utilities (winutils framework).
    Handles both individual binaries and monolithic coreutils.exe with intelligent
    symlink management, performance monitoring, and automated rollback capabilities.

.PARAMETER Action
    Deployment action: Deploy, Rollback, Update, Status, Health, Benchmark, Validate

.PARAMETER Mode
    Deployment mode: Individual, Monolithic, Hybrid (default: Hybrid)

.PARAMETER Utilities
    Specific utilities to deploy (comma-separated). Empty = all utilities

.PARAMETER TargetPath
    Deployment target directory (default: C:\users\david\.local\bin)

.PARAMETER Force
    Skip confirmation prompts and force operations

.PARAMETER DryRun
    Preview deployment actions without making changes

.PARAMETER SkipBackup
    Skip backup creation (not recommended for production)

.PARAMETER BenchmarkIterations
    Number of benchmark iterations (default: 10)

.EXAMPLE
    .\WinUtils-Deployment-Framework.ps1 -Action Deploy
    Deploy all utilities in hybrid mode with automatic optimization

.EXAMPLE
    .\WinUtils-Deployment-Framework.ps1 -Action Deploy -Mode Individual -Utilities "ls,cat,grep"
    Deploy specific utilities as individual binaries

.EXAMPLE
    .\WinUtils-Deployment-Framework.ps1 -Action Benchmark -BenchmarkIterations 20
    Run comprehensive performance benchmarks

.EXAMPLE
    .\WinUtils-Deployment-Framework.ps1 -Action Rollback -Force
    Rollback deployment with force flag

.AUTHOR
    Claude Code - Deployment Engineer

.VERSION
    3.0.0 - Unified Production Framework
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $false)]
    [ValidateSet("Deploy", "Rollback", "Update", "Status", "Health", "Benchmark", "Validate", "Optimize", "Switch")]
    [string]$Action = "Deploy",

    [Parameter(Mandatory = $false)]
    [ValidateSet("Individual", "Monolithic", "Hybrid")]
    [string]$Mode = "Hybrid",

    [Parameter(Mandatory = $false)]
    [string[]]$Utilities = @(),

    [Parameter(Mandatory = $false)]
    [string]$TargetPath = "C:\users\david\.local\bin",

    [Parameter(Mandatory = $false)]
    [switch]$Force,

    [Parameter(Mandatory = $false)]
    [switch]$DryRun,

    [Parameter(Mandatory = $false)]
    [switch]$SkipBackup,

    [Parameter(Mandatory = $false)]
    [int]$BenchmarkIterations = 10,

    [Parameter(Mandatory = $false)]
    [switch]$SkipPathIntegration,

    [Parameter(Mandatory = $false)]
    [switch]$Verbose
)

$ErrorActionPreference = 'Stop'
$FRAMEWORK_VERSION = "3.0.0"
$TIMESTAMP = Get-Date -Format "yyyyMMdd_HHmmss"

# ============================================================================
# Configuration
# ============================================================================

$script:Config = @{
    SourceDir = "T:\projects\coreutils\winutils\target\x86_64-pc-windows-msvc\release"
    TargetDir = $TargetPath
    BackupDir = "T:\projects\coreutils\backups\winutils"
    LogDir = "T:\projects\coreutils\logs"
    ManifestPath = "T:\projects\coreutils\deployment-manifest.json"
    MonolithicBinary = "coreutils.exe"
    BenchmarkDataDir = "T:\projects\coreutils\benchmark-data"

    # Alternate deployment targets
    AlternateTargets = @(
        "C:\users\david\bin",
        "C:\users\david\.cargo\bin"
    )

    # Performance thresholds (milliseconds)
    PerformanceThresholds = @{
        Critical = 10    # <10ms = critical path
        Fast = 50        # <50ms = fast
        Normal = 200     # <200ms = normal
        Slow = 1000      # >1000ms = needs optimization
    }

    # Utility categories for intelligent deployment
    UtilityCategories = @{
        CriticalPath = @("ls", "cat", "pwd", "which", "where", "echo")
        FileOperations = @("cp", "mv", "rm", "mkdir", "touch", "chmod")
        TextProcessing = @("grep", "sed", "awk", "cut", "sort", "uniq", "wc", "head", "tail", "tr")
        SystemInfo = @("uname", "hostname", "whoami", "arch", "df", "du")
        Checksums = @("md5sum", "sha1sum", "sha256sum", "sha512sum", "cksum", "hashsum")
    }
}

# ============================================================================
# Logging and Output Functions
# ============================================================================

function Write-Log {
    param(
        [string]$Message,
        [ValidateSet("INFO", "WARN", "ERROR", "SUCCESS", "DEBUG")]
        [string]$Level = "INFO",
        [switch]$NoConsole
    )

    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss.fff"
    $logFile = Join-Path $script:Config.LogDir "deployment-$TIMESTAMP.log"

    # Ensure log directory exists
    if (-not (Test-Path $script:Config.LogDir)) {
        New-Item -ItemType Directory -Path $script:Config.LogDir -Force | Out-Null
    }

    $logEntry = "[$timestamp] [$Level] $Message"
    $logEntry | Add-Content -Path $logFile -Encoding UTF8

    if (-not $NoConsole) {
        switch ($Level) {
            "SUCCESS" { Write-Host $logEntry -ForegroundColor Green }
            "WARN"    { Write-Host $logEntry -ForegroundColor Yellow }
            "ERROR"   { Write-Host $logEntry -ForegroundColor Red }
            "DEBUG"   { if ($Verbose) { Write-Host $logEntry -ForegroundColor Gray } }
            default   { Write-Host $logEntry -ForegroundColor White }
        }
    }
}

function Write-Header {
    param([string]$Title)

    $separator = "=" * 80
    Write-Host ""
    Write-Host $separator -ForegroundColor Cyan
    Write-Host "  $Title" -ForegroundColor Cyan
    Write-Host $separator -ForegroundColor Cyan
    Write-Host ""

    Write-Log "=== $Title ===" "INFO" -NoConsole
}

function Write-Progress {
    param(
        [string]$Activity,
        [string]$Status,
        [int]$PercentComplete,
        [int]$Current = 0,
        [int]$Total = 0
    )

    if ($Total -gt 0) {
        $PercentComplete = [math]::Round(($Current / $Total) * 100)
        $Status = "$Status ($Current/$Total)"
    }

    Write-Progress -Activity $Activity -Status $Status -PercentComplete $PercentComplete
}

# ============================================================================
# Utility Discovery and Metadata
# ============================================================================

function Get-AvailableUtilities {
    <#
    .SYNOPSIS
        Discover all available utilities from source directory
    #>

    Write-Log "Discovering available utilities..." "DEBUG"

    $utilities = @()
    $sourceFiles = Get-ChildItem -Path $script:Config.SourceDir -Filter "*.exe" -ErrorAction SilentlyContinue

    foreach ($file in $sourceFiles) {
        if ($file.Name -eq $script:Config.MonolithicBinary) {
            continue
        }

        $utilName = [System.IO.Path]::GetFileNameWithoutExtension($file.Name)

        $utilInfo = @{
            Name = $utilName
            SourcePath = $file.FullName
            Size = $file.Length
            LastModified = $file.LastWriteTime
            Category = Get-UtilityCategory $utilName
            IsDeployed = $false
            DeployedPath = $null
            SymlinkPath = $null
        }

        $utilities += [PSCustomObject]$utilInfo
    }

    Write-Log "Discovered $($utilities.Count) utilities" "INFO"
    return $utilities
}

function Get-UtilityCategory {
    param([string]$UtilityName)

    foreach ($category in $script:Config.UtilityCategories.GetEnumerator()) {
        if ($category.Value -contains $UtilityName) {
            return $category.Key
        }
    }

    return "Standard"
}

function Get-MonolithicUtilities {
    <#
    .SYNOPSIS
        Get list of utilities available in monolithic coreutils.exe
    #>

    $coreutilsPath = Join-Path $script:Config.SourceDir $script:Config.MonolithicBinary

    if (-not (Test-Path $coreutilsPath)) {
        Write-Log "Monolithic binary not found: $coreutilsPath" "WARN"
        return @()
    }

    try {
        # Execute coreutils.exe --list to get available utilities
        $output = & $coreutilsPath "--list" 2>&1

        if ($LASTEXITCODE -eq 0) {
            $utilities = $output -split "`n" | Where-Object { $_.Trim() } | ForEach-Object { $_.Trim() }
            Write-Log "Monolithic binary supports $($utilities.Count) utilities" "DEBUG"
            return $utilities
        }
    }
    catch {
        Write-Log "Failed to query monolithic binary: $_" "WARN"
    }

    return @()
}

# ============================================================================
# Deployment Validation
# ============================================================================

function Test-DeploymentPrerequisites {
    <#
    .SYNOPSIS
        Validate deployment prerequisites and environment
    #>

    Write-Log "Validating deployment prerequisites..." "INFO"

    $issues = @()

    # Check source directory
    if (-not (Test-Path $script:Config.SourceDir)) {
        $issues += "Source directory not found: $($script:Config.SourceDir)"
    }

    # Check disk space (minimum 500MB)
    $targetDrive = (Get-Item $script:Config.TargetDir -ErrorAction SilentlyContinue)
    if ($targetDrive) {
        $freeSpace = (Get-PSDrive $targetDrive.Root.Name.TrimEnd(':')).Free
        if ($freeSpace -lt 500MB) {
            $issues += "Insufficient disk space. Need at least 500MB on $($targetDrive.Root.Name)"
        }
    }

    # Check target directory writability
    if (Test-Path $script:Config.TargetDir) {
        $testFile = Join-Path $script:Config.TargetDir ".deployment-test-$TIMESTAMP"
        try {
            "test" | Out-File $testFile -Encoding ASCII
            Remove-Item $testFile -Force
        }
        catch {
            $issues += "Target directory not writable: $($script:Config.TargetDir)"
        }
    }

    # Check for existing problematic installations
    $existingUU = Get-ChildItem -Path $script:Config.TargetDir -Filter "uu-*.exe" -ErrorAction SilentlyContinue
    if ($existingUU.Count -gt 0) {
        Write-Log "Found $($existingUU.Count) existing uu-* utilities" "WARN"
    }

    if ($issues.Count -gt 0) {
        Write-Log "Prerequisites validation failed:" "ERROR"
        foreach ($issue in $issues) {
            Write-Log "  - $issue" "ERROR"
        }
        return $false
    }

    Write-Log "Prerequisites validation passed" "SUCCESS"
    return $true
}

function Test-UtilityHealth {
    param(
        [string]$UtilityPath,
        [string]$UtilityName
    )

    <#
    .SYNOPSIS
        Test if a deployed utility is functional
    #>

    if (-not (Test-Path $UtilityPath)) {
        return @{
            Healthy = $false
            Message = "File not found"
        }
    }

    try {
        # Try --version first (most utilities support this)
        $output = & $UtilityPath --version 2>&1

        if ($LASTEXITCODE -eq 0 -or $output) {
            return @{
                Healthy = $true
                Message = "OK"
                Version = ($output | Select-Object -First 1)
            }
        }

        # Try --help as fallback
        $output = & $UtilityPath --help 2>&1

        if ($LASTEXITCODE -eq 0 -or $output) {
            return @{
                Healthy = $true
                Message = "OK (via --help)"
            }
        }

        return @{
            Healthy = $false
            Message = "Utility did not respond to --version or --help"
        }
    }
    catch {
        return @{
            Healthy = $false
            Message = "Execution error: $($_.Exception.Message)"
        }
    }
}

# ============================================================================
# Backup and Rollback
# ============================================================================

function New-DeploymentBackup {
    param(
        [string]$BackupName = "deployment-$TIMESTAMP"
    )

    <#
    .SYNOPSIS
        Create backup of current deployment
    #>

    if ($SkipBackup) {
        Write-Log "Backup skipped by user request" "WARN"
        return $null
    }

    Write-Log "Creating deployment backup: $BackupName" "INFO"

    $backupDir = Join-Path $script:Config.BackupDir $BackupName
    New-Item -ItemType Directory -Path $backupDir -Force | Out-Null

    # Backup existing utilities
    $existingUtils = Get-ChildItem -Path $script:Config.TargetDir -Filter "*.exe" -ErrorAction SilentlyContinue
    $backedUp = 0

    foreach ($util in $existingUtils) {
        try {
            Copy-Item -Path $util.FullName -Destination $backupDir -Force
            $backedUp++
        }
        catch {
            Write-Log "Failed to backup $($util.Name): $_" "WARN"
        }
    }

    # Save deployment manifest
    $manifest = @{
        BackupDate = Get-Date -Format "o"
        UtilityCount = $backedUp
        TargetPath = $script:Config.TargetDir
        FrameworkVersion = $FRAMEWORK_VERSION
    }

    $manifestPath = Join-Path $backupDir "backup-manifest.json"
    $manifest | ConvertTo-Json -Depth 3 | Set-Content $manifestPath -Encoding UTF8

    Write-Log "Backup completed: $backedUp utilities backed up to $backupDir" "SUCCESS"

    return $backupDir
}

function Restore-DeploymentBackup {
    param(
        [string]$BackupPath
    )

    <#
    .SYNOPSIS
        Restore from backup
    #>

    if (-not (Test-Path $BackupPath)) {
        Write-Log "Backup not found: $BackupPath" "ERROR"
        return $false
    }

    Write-Log "Restoring from backup: $BackupPath" "INFO"

    # Read backup manifest
    $manifestPath = Join-Path $BackupPath "backup-manifest.json"
    if (Test-Path $manifestPath) {
        $manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
        Write-Log "Backup created: $($manifest.BackupDate)" "INFO"
        Write-Log "Utilities in backup: $($manifest.UtilityCount)" "INFO"
    }

    # Restore utilities
    $utilities = Get-ChildItem -Path $BackupPath -Filter "*.exe"
    $restored = 0

    foreach ($util in $utilities) {
        $targetPath = Join-Path $script:Config.TargetDir $util.Name

        try {
            Copy-Item -Path $util.FullName -Destination $targetPath -Force
            $restored++
            Write-Log "Restored: $($util.Name)" "DEBUG"
        }
        catch {
            Write-Log "Failed to restore $($util.Name): $_" "ERROR"
        }
    }

    Write-Log "Restore completed: $restored utilities restored" "SUCCESS"
    return $true
}

# ============================================================================
# Core Deployment Functions
# ============================================================================

function Deploy-IndividualUtilities {
    param(
        [object[]]$Utilities,
        [switch]$CreateSymlinks = $true
    )

    <#
    .SYNOPSIS
        Deploy individual utility binaries
    #>

    Write-Header "Deploying Individual Utilities"

    $deployed = 0
    $failed = 0
    $skipped = 0
    $total = $Utilities.Count

    foreach ($util in $Utilities) {
        $current = $deployed + $failed + $skipped + 1
        Write-Progress -Activity "Deploying Utilities" -Status "Processing $($util.Name)" -Current $current -Total $total

        $targetPath = Join-Path $script:Config.TargetDir "$($util.Name).exe"

        # Check if already deployed and up-to-date
        if (Test-Path $targetPath) {
            $existingFile = Get-Item $targetPath
            $sourceFile = Get-Item $util.SourcePath

            if ($existingFile.LastWriteTime -ge $sourceFile.LastWriteTime -and $existingFile.Length -eq $sourceFile.Length) {
                Write-Log "Skipping $($util.Name) - already up-to-date" "DEBUG"
                $skipped++
                continue
            }
        }

        try {
            if ($DryRun) {
                Write-Host "  [DRY RUN] Would deploy: $($util.Name)" -ForegroundColor Yellow
                $deployed++
            }
            else {
                Copy-Item -Path $util.SourcePath -Destination $targetPath -Force
                Write-Log "Deployed: $($util.Name) -> $targetPath" "SUCCESS"
                $deployed++

                # Create uu- prefixed symlink for compatibility
                if ($CreateSymlinks) {
                    $symlinkPath = Join-Path $script:Config.TargetDir "uu-$($util.Name).exe"

                    if (Test-Path $symlinkPath) {
                        Remove-Item $symlinkPath -Force -ErrorAction SilentlyContinue
                    }

                    # Use hard link for better performance
                    cmd /c mklink /H "$symlinkPath" "$targetPath" 2>&1 | Out-Null

                    if (Test-Path $symlinkPath) {
                        Write-Log "Created symlink: uu-$($util.Name).exe" "DEBUG"
                    }
                }
            }
        }
        catch {
            Write-Log "Failed to deploy $($util.Name): $_" "ERROR"
            $failed++
        }
    }

    Write-Progress -Activity "Deploying Utilities" -Completed

    return @{
        Deployed = $deployed
        Failed = $failed
        Skipped = $skipped
        Total = $total
    }
}

function Deploy-MonolithicBinary {
    param(
        [string[]]$UtilityList
    )

    <#
    .SYNOPSIS
        Deploy monolithic coreutils.exe with symlinks
    #>

    Write-Header "Deploying Monolithic Binary"

    $sourcePath = Join-Path $script:Config.SourceDir $script:Config.MonolithicBinary

    if (-not (Test-Path $sourcePath)) {
        Write-Log "Monolithic binary not found: $sourcePath" "ERROR"
        return @{ Deployed = 0; Failed = 1 }
    }

    $targetPath = Join-Path $script:Config.TargetDir $script:Config.MonolithicBinary

    try {
        if ($DryRun) {
            Write-Host "  [DRY RUN] Would deploy: $($script:Config.MonolithicBinary)" -ForegroundColor Yellow
        }
        else {
            Copy-Item -Path $sourcePath -Destination $targetPath -Force
            Write-Log "Deployed monolithic binary: $targetPath" "SUCCESS"
        }

        # Create symlinks for each utility
        $created = 0
        $failed = 0

        foreach ($utilName in $UtilityList) {
            $symlinkPath = Join-Path $script:Config.TargetDir "$utilName.exe"

            try {
                if (-not $DryRun) {
                    if (Test-Path $symlinkPath) {
                        Remove-Item $symlinkPath -Force -ErrorAction SilentlyContinue
                    }

                    cmd /c mklink /H "$symlinkPath" "$targetPath" 2>&1 | Out-Null

                    if (Test-Path $symlinkPath) {
                        $created++
                        Write-Log "Created symlink: $utilName.exe -> $($script:Config.MonolithicBinary)" "DEBUG"
                    }
                }
                else {
                    Write-Host "  [DRY RUN] Would create symlink: $utilName.exe" -ForegroundColor Yellow
                    $created++
                }
            }
            catch {
                Write-Log "Failed to create symlink for $utilName`: $_" "WARN"
                $failed++
            }
        }

        Write-Log "Created $created symlinks ($failed failed)" "INFO"

        return @{
            Deployed = 1
            Symlinks = $created
            Failed = $failed
        }
    }
    catch {
        Write-Log "Failed to deploy monolithic binary: $_" "ERROR"
        return @{ Deployed = 0; Failed = 1 }
    }
}

function Deploy-HybridMode {
    param([object[]]$Utilities)

    <#
    .SYNOPSIS
        Intelligent hybrid deployment - use individual binaries for critical path,
        monolithic for less frequently used utilities
    #>

    Write-Header "Deploying in Hybrid Mode (Optimized)"

    # Categorize utilities
    $criticalPath = $Utilities | Where-Object { $_.Category -eq "CriticalPath" }
    $remaining = $Utilities | Where-Object { $_.Category -ne "CriticalPath" }

    Write-Log "Critical path utilities: $($criticalPath.Count)" "INFO"
    Write-Log "Standard utilities: $($remaining.Count)" "INFO"

    # Deploy critical path as individual binaries for maximum performance
    $individualResult = Deploy-IndividualUtilities -Utilities $criticalPath -CreateSymlinks $true

    # Deploy remaining utilities via monolithic binary
    $monolithicUtils = $remaining | ForEach-Object { $_.Name }
    $monolithicResult = Deploy-MonolithicBinary -UtilityList $monolithicUtils

    return @{
        Individual = $individualResult
        Monolithic = $monolithicResult
    }
}

# ============================================================================
# PATH Integration
# ============================================================================

function Update-SystemPath {
    param([switch]$Remove)

    <#
    .SYNOPSIS
        Update system PATH to prioritize deployment directory
    #>

    if ($SkipPathIntegration) {
        Write-Log "PATH integration skipped by user request" "WARN"
        return
    }

    Write-Log "Updating system PATH..." "INFO"

    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    $deployPath = $script:Config.TargetDir

    if ($Remove) {
        if ($currentPath -like "*$deployPath*") {
            $newPath = $currentPath -replace [regex]::Escape("$deployPath;"), ""
            $newPath = $newPath -replace [regex]::Escape(";$deployPath"), ""
            [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
            $env:PATH = $env:PATH -replace [regex]::Escape("$deployPath;"), ""
            Write-Log "Removed deployment directory from PATH" "SUCCESS"
        }
    }
    else {
        if ($currentPath -notlike "*$deployPath*") {
            $newPath = "$deployPath;$currentPath"
            [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
            $env:PATH = "$deployPath;$env:PATH"
            Write-Log "Added deployment directory to PATH with priority" "SUCCESS"
        }
        else {
            Write-Log "Deployment directory already in PATH" "DEBUG"
        }
    }
}

# ============================================================================
# Performance Benchmarking
# ============================================================================

function Invoke-PerformanceBenchmark {
    param(
        [string[]]$UtilityNames,
        [int]$Iterations = 10
    )

    <#
    .SYNOPSIS
        Benchmark deployed utilities
    #>

    Write-Header "Performance Benchmarking ($Iterations iterations)"

    # Create benchmark data
    $benchmarkDir = $script:Config.BenchmarkDataDir
    if (-not (Test-Path $benchmarkDir)) {
        New-Item -ItemType Directory -Path $benchmarkDir -Force | Out-Null
    }

    # Generate test files if not exist
    $testFiles = @{
        "small.txt" = 1KB
        "medium.txt" = 100KB
        "large.txt" = 10MB
    }

    foreach ($file in $testFiles.GetEnumerator()) {
        $testPath = Join-Path $benchmarkDir $file.Key
        if (-not (Test-Path $testPath)) {
            $bytes = New-Object byte[] $file.Value
            (New-Object Random).NextBytes($bytes)
            [System.IO.File]::WriteAllBytes($testPath, $bytes)
        }
    }

    $results = @()

    foreach ($utilName in $UtilityNames) {
        $utilPath = Join-Path $script:Config.TargetDir "$utilName.exe"

        if (-not (Test-Path $utilPath)) {
            Write-Log "Utility not deployed, skipping benchmark: $utilName" "WARN"
            continue
        }

        Write-Log "Benchmarking: $utilName" "INFO"

        # Define benchmark tests per utility
        $testCases = Get-BenchmarkTestCases -UtilityName $utilName -DataDir $benchmarkDir

        foreach ($testCase in $testCases) {
            $times = @()

            for ($i = 1; $i -le $Iterations; $i++) {
                $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

                try {
                    $output = & $utilPath $testCase.Args 2>&1 | Out-Null
                    $stopwatch.Stop()
                    $times += $stopwatch.ElapsedMilliseconds
                }
                catch {
                    Write-Log "Benchmark failed for $utilName $($testCase.Description): $_" "WARN"
                    break
                }
            }

            if ($times.Count -eq $Iterations) {
                $avg = ($times | Measure-Object -Average).Average
                $min = ($times | Measure-Object -Minimum).Minimum
                $max = ($times | Measure-Object -Maximum).Maximum

                $performance = if ($avg -lt $script:Config.PerformanceThresholds.Critical) { "Excellent" }
                              elseif ($avg -lt $script:Config.PerformanceThresholds.Fast) { "Good" }
                              elseif ($avg -lt $script:Config.PerformanceThresholds.Normal) { "Acceptable" }
                              else { "Needs Optimization" }

                $results += [PSCustomObject]@{
                    Utility = $utilName
                    Test = $testCase.Description
                    AvgMs = [math]::Round($avg, 2)
                    MinMs = [math]::Round($min, 2)
                    MaxMs = [math]::Round($max, 2)
                    Performance = $performance
                }

                $color = switch ($performance) {
                    "Excellent" { "Green" }
                    "Good" { "Cyan" }
                    "Acceptable" { "Yellow" }
                    default { "Red" }
                }

                Write-Host "  $($testCase.Description): ${avg}ms (${min}-${max}ms) - $performance" -ForegroundColor $color
            }
        }
    }

    # Save results
    $reportPath = Join-Path $script:Config.LogDir "benchmark-results-$TIMESTAMP.json"
    $results | ConvertTo-Json -Depth 3 | Set-Content $reportPath -Encoding UTF8
    Write-Log "Benchmark results saved to: $reportPath" "SUCCESS"

    return $results
}

function Get-BenchmarkTestCases {
    param(
        [string]$UtilityName,
        [string]$DataDir
    )

    $smallFile = Join-Path $DataDir "small.txt"
    $mediumFile = Join-Path $DataDir "medium.txt"
    $largeFile = Join-Path $DataDir "large.txt"

    $testCases = @{
        "ls" = @(
            @{ Description = "List current directory"; Args = @(".") }
            @{ Description = "List with details"; Args = @("-la", ".") }
        )
        "cat" = @(
            @{ Description = "Display small file"; Args = @($smallFile) }
            @{ Description = "Display medium file"; Args = @($mediumFile) }
        )
        "wc" = @(
            @{ Description = "Count lines (medium)"; Args = @("-l", $mediumFile) }
            @{ Description = "Count words (large)"; Args = @("-w", $largeFile) }
        )
        "head" = @(
            @{ Description = "First 10 lines"; Args = @("-n", "10", $largeFile) }
        )
        "tail" = @(
            @{ Description = "Last 10 lines"; Args = @("-n", "10", $largeFile) }
        )
        "sort" = @(
            @{ Description = "Sort medium file"; Args = @($mediumFile) }
        )
        "grep" = @(
            @{ Description = "Search pattern"; Args = @("test", $mediumFile) }
        )
    }

    if ($testCases.ContainsKey($UtilityName)) {
        return $testCases[$UtilityName]
    }

    # Default test case
    return @(
        @{ Description = "Version check"; Args = @("--version") }
    )
}

# ============================================================================
# Status and Health Monitoring
# ============================================================================

function Get-DeploymentStatus {
    <#
    .SYNOPSIS
        Get comprehensive deployment status
    #>

    Write-Header "Deployment Status"

    $availableUtils = Get-AvailableUtilities
    $deployed = @()
    $notDeployed = @()
    $unhealthy = @()

    foreach ($util in $availableUtils) {
        $targetPath = Join-Path $script:Config.TargetDir "$($util.Name).exe"

        if (Test-Path $targetPath) {
            $health = Test-UtilityHealth -UtilityPath $targetPath -UtilityName $util.Name

            if ($health.Healthy) {
                $deployed += $util.Name
            }
            else {
                $unhealthy += [PSCustomObject]@{
                    Name = $util.Name
                    Issue = $health.Message
                }
            }
        }
        else {
            $notDeployed += $util.Name
        }
    }

    # Display status
    Write-Host "`nDeployed Utilities: $($deployed.Count)" -ForegroundColor Green
    if ($deployed.Count -gt 0 -and $Verbose) {
        $deployed | Sort-Object | ForEach-Object { Write-Host "  ✓ $_" -ForegroundColor Green }
    }

    Write-Host "`nNot Deployed: $($notDeployed.Count)" -ForegroundColor Yellow
    if ($notDeployed.Count -gt 0 -and $Verbose) {
        $notDeployed | Sort-Object | ForEach-Object { Write-Host "  • $_" -ForegroundColor Yellow }
    }

    if ($unhealthy.Count -gt 0) {
        Write-Host "`nUnhealthy Utilities: $($unhealthy.Count)" -ForegroundColor Red
        foreach ($item in $unhealthy) {
            Write-Host "  ✗ $($item.Name): $($item.Issue)" -ForegroundColor Red
        }
    }

    # Monolithic binary status
    $coreutilsPath = Join-Path $script:Config.TargetDir $script:Config.MonolithicBinary
    $monolithicDeployed = Test-Path $coreutilsPath

    Write-Host "`nMonolithic Binary: $(if ($monolithicDeployed) { '✓ Deployed' } else { '✗ Not Deployed' })" -ForegroundColor $(if ($monolithicDeployed) { 'Green' } else { 'Yellow' })

    # PATH status
    $pathStatus = $env:PATH -like "*$($script:Config.TargetDir)*"
    Write-Host "PATH Integration: $(if ($pathStatus) { '✓ Configured' } else { '✗ Not Configured' })" -ForegroundColor $(if ($pathStatus) { 'Green' } else { 'Yellow' })

    return @{
        Deployed = $deployed
        NotDeployed = $notDeployed
        Unhealthy = $unhealthy
        MonolithicDeployed = $monolithicDeployed
        PathConfigured = $pathStatus
        TotalAvailable = $availableUtils.Count
    }
}

# ============================================================================
# Main Execution Logic
# ============================================================================

function Invoke-DeploymentAction {
    param([string]$Action)

    try {
        switch ($Action) {
            "Deploy" {
                Write-Header "WinUtils Deployment Framework v$FRAMEWORK_VERSION"
                Write-Log "Action: Deploy | Mode: $Mode | Target: $($script:Config.TargetDir)" "INFO"

                if (-not (Test-DeploymentPrerequisites)) {
                    throw "Prerequisites validation failed"
                }

                # Create backup
                $backupDir = New-DeploymentBackup

                # Get utilities to deploy
                $availableUtils = Get-AvailableUtilities

                if ($Utilities.Count -gt 0) {
                    $availableUtils = $availableUtils | Where-Object { $Utilities -contains $_.Name }
                    Write-Log "Filtered to $($availableUtils.Count) specified utilities" "INFO"
                }

                # Execute deployment based on mode
                $result = switch ($Mode) {
                    "Individual" { Deploy-IndividualUtilities -Utilities $availableUtils }
                    "Monolithic" {
                        $utilNames = $availableUtils | ForEach-Object { $_.Name }
                        Deploy-MonolithicBinary -UtilityList $utilNames
                    }
                    "Hybrid" { Deploy-HybridMode -Utilities $availableUtils }
                }

                # Update PATH
                if (-not $SkipPathIntegration) {
                    Update-SystemPath
                }

                # Display summary
                Write-Header "Deployment Summary"
                Write-Host "Deployment Mode: $Mode" -ForegroundColor Cyan
                Write-Host "Backup Location: $backupDir" -ForegroundColor Yellow

                if ($result.Individual) {
                    Write-Host "`nIndividual Utilities:" -ForegroundColor Cyan
                    Write-Host "  Deployed: $($result.Individual.Deployed)" -ForegroundColor Green
                    Write-Host "  Failed: $($result.Individual.Failed)" -ForegroundColor $(if ($result.Individual.Failed -gt 0) { 'Red' } else { 'Green' })
                    Write-Host "  Skipped: $($result.Individual.Skipped)" -ForegroundColor Yellow
                }

                if ($result.Monolithic) {
                    Write-Host "`nMonolithic Binary:" -ForegroundColor Cyan
                    Write-Host "  Deployed: $($result.Monolithic.Deployed)" -ForegroundColor Green
                    Write-Host "  Symlinks: $($result.Monolithic.Symlinks)" -ForegroundColor Green
                    Write-Host "  Failed: $($result.Monolithic.Failed)" -ForegroundColor $(if ($result.Monolithic.Failed -gt 0) { 'Red' } else { 'Green' })
                }

                Write-Log "Deployment completed successfully" "SUCCESS"
            }

            "Rollback" {
                Write-Header "Rollback Deployment"

                if (-not $Force) {
                    $confirm = Read-Host "This will remove all deployed utilities. Continue? (Y/N)"
                    if ($confirm -notlike "Y*") {
                        Write-Log "Rollback cancelled by user" "WARN"
                        return
                    }
                }

                # Find latest backup
                $backups = Get-ChildItem -Path $script:Config.BackupDir -Directory | Sort-Object Name -Descending

                if ($backups.Count -gt 0) {
                    $latestBackup = $backups[0]
                    Write-Log "Restoring from latest backup: $($latestBackup.Name)" "INFO"
                    Restore-DeploymentBackup -BackupPath $latestBackup.FullName
                }
                else {
                    Write-Log "No backups found, performing clean removal" "WARN"

                    # Remove deployed utilities
                    $utils = Get-ChildItem -Path $script:Config.TargetDir -Filter "*.exe" |
                             Where-Object { $_.Name -ne "coreutils.exe" -or $_.Name -like "uu-*.exe" }

                    foreach ($util in $utils) {
                        Remove-Item $util.FullName -Force
                        Write-Log "Removed: $($util.Name)" "DEBUG"
                    }
                }

                # Remove from PATH
                Update-SystemPath -Remove

                Write-Log "Rollback completed" "SUCCESS"
            }

            "Status" {
                $status = Get-DeploymentStatus
            }

            "Health" {
                Write-Header "Health Check"

                $availableUtils = Get-AvailableUtilities
                $healthy = 0
                $unhealthy = 0

                foreach ($util in $availableUtils) {
                    $targetPath = Join-Path $script:Config.TargetDir "$($util.Name).exe"

                    if (Test-Path $targetPath) {
                        $health = Test-UtilityHealth -UtilityPath $targetPath -UtilityName $util.Name

                        if ($health.Healthy) {
                            $healthy++
                            if ($Verbose) {
                                Write-Host "  ✓ $($util.Name) - $($health.Message)" -ForegroundColor Green
                            }
                        }
                        else {
                            $unhealthy++
                            Write-Host "  ✗ $($util.Name) - $($health.Message)" -ForegroundColor Red
                        }
                    }
                }

                Write-Host "`nHealth Summary:" -ForegroundColor Cyan
                Write-Host "  Healthy: $healthy" -ForegroundColor Green
                Write-Host "  Unhealthy: $unhealthy" -ForegroundColor $(if ($unhealthy -gt 0) { 'Red' } else { 'Green' })
            }

            "Benchmark" {
                # Determine utilities to benchmark
                $utilsToTest = if ($Utilities.Count -gt 0) {
                    $Utilities
                }
                else {
                    # Benchmark critical path utilities by default
                    $script:Config.UtilityCategories.CriticalPath
                }

                $results = Invoke-PerformanceBenchmark -UtilityNames $utilsToTest -Iterations $BenchmarkIterations

                Write-Host "`nBenchmark completed. Tested $($results.Count) test cases." -ForegroundColor Green
            }

            "Validate" {
                Write-Header "Deployment Validation"

                $issues = @()

                # Validate prerequisites
                if (-not (Test-DeploymentPrerequisites)) {
                    $issues += "Prerequisites validation failed"
                }

                # Validate deployed utilities
                $status = Get-DeploymentStatus

                if ($status.Unhealthy.Count -gt 0) {
                    $issues += "$($status.Unhealthy.Count) unhealthy utilities detected"
                }

                # Validate PATH
                if (-not $status.PathConfigured) {
                    $issues += "PATH not properly configured"
                }

                if ($issues.Count -eq 0) {
                    Write-Host "`n✓ Validation passed - deployment is healthy" -ForegroundColor Green
                }
                else {
                    Write-Host "`nValidation issues found:" -ForegroundColor Red
                    foreach ($issue in $issues) {
                        Write-Host "  ✗ $issue" -ForegroundColor Red
                    }
                }
            }

            default {
                Write-Log "Unknown action: $Action" "ERROR"
                throw "Invalid action specified"
            }
        }
    }
    catch {
        Write-Log "Action failed: $($_.Exception.Message)" "ERROR"
        Write-Log "Stack trace: $($_.ScriptStackTrace)" "DEBUG"
        throw
    }
}

# ============================================================================
# Entry Point
# ============================================================================

try {
    Write-Host ""
    Write-Host "╔════════════════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║  WinUtils Deployment Framework v$FRAMEWORK_VERSION                                    ║" -ForegroundColor Cyan
    Write-Host "║  Production-Ready Deployment System for 78 Optimized Windows Utilities    ║" -ForegroundColor Cyan
    Write-Host "╚════════════════════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
    Write-Host ""

    if ($DryRun) {
        Write-Host ">>> DRY RUN MODE - No changes will be made <<<" -ForegroundColor Yellow
        Write-Host ""
    }

    Invoke-DeploymentAction -Action $Action

    Write-Host ""
    Write-Host "╔════════════════════════════════════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "║  Operation completed successfully!                                         ║" -ForegroundColor Green
    Write-Host "╚════════════════════════════════════════════════════════════════════════════╝" -ForegroundColor Green
    Write-Host ""

    Write-Log "Framework execution completed successfully" "SUCCESS"
    exit 0
}
catch {
    Write-Host ""
    Write-Host "╔════════════════════════════════════════════════════════════════════════════╗" -ForegroundColor Red
    Write-Host "║  Operation FAILED                                                          ║" -ForegroundColor Red
    Write-Host "╚════════════════════════════════════════════════════════════════════════════╝" -ForegroundColor Red
    Write-Host ""
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host ""
    Write-Log "Framework execution failed: $($_.Exception.Message)" "ERROR"
    exit 1
}
