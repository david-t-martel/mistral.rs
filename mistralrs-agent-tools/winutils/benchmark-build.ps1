# Build Performance Benchmarking Script for WinUtils
# Measures clean builds, incremental builds, and test execution times

param(
    [switch]$CleanBuild,
    [switch]$IncrementalBuild,
    [switch]$TestExecution,
    [switch]$All,
    [string]$Profile = "release",
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

# Ensure we're in the right directory
$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $ProjectRoot

# Color output functions
function Write-Header {
    param([string]$Text)
    Write-Host "`n╔════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║ $Text" -ForegroundColor Cyan
    Write-Host "╚════════════════════════════════════════════════╝" -ForegroundColor Cyan
}

function Write-Metric {
    param(
        [string]$Name,
        [string]$Value,
        [string]$Unit = ""
    )
    Write-Host "  ► $Name`: " -NoNewline -ForegroundColor Yellow
    Write-Host "$Value $Unit" -ForegroundColor Green
}

function Format-Duration {
    param([TimeSpan]$Duration)
    if ($Duration.TotalHours -ge 1) {
        return "{0:hh\:mm\:ss\.fff}" -f $Duration
    } elseif ($Duration.TotalMinutes -ge 1) {
        return "{0:mm\:ss\.fff}" -f $Duration
    } else {
        return "{0:ss\.fff} seconds" -f $Duration
    }
}

function Measure-BuildTime {
    param(
        [string]$BuildType,
        [string]$Profile
    )

    Write-Header "$BuildType Build Benchmark ($Profile profile)"

    # Prepare for build
    if ($BuildType -eq "Clean") {
        Write-Host "  Cleaning target directory..." -ForegroundColor Gray
        if (Test-Path "target") {
            Remove-Item -Recurse -Force "target" -ErrorAction SilentlyContinue
        }
        # Also clean cargo registry cache for true clean build
        if ($Verbose) {
            cargo clean
        }
    }

    # Start timing
    $StartTime = Get-Date
    Write-Host "  Starting build at: $($StartTime.ToString('HH:mm:ss.fff'))" -ForegroundColor Gray

    # Run the build
    $BuildArgs = @("build")
    if ($Profile -eq "release") {
        $BuildArgs += "--release"
    } elseif ($Profile -eq "release-fast") {
        $BuildArgs += "--profile", "release-fast"
    }

    if ($Verbose) {
        $BuildArgs += "-vv"
        Write-Host "  Command: cargo $($BuildArgs -join ' ')" -ForegroundColor DarkGray
    }

    # Execute build and capture output
    $BuildOutput = & cargo @BuildArgs 2>&1
    $BuildSuccess = $LASTEXITCODE -eq 0

    $EndTime = Get-Date
    $Duration = $EndTime - $StartTime

    # Display results
    if ($BuildSuccess) {
        Write-Metric "Status" "SUCCESS"
        Write-Metric "Duration" (Format-Duration $Duration)
        Write-Metric "Start Time" $StartTime.ToString('HH:mm:ss.fff')
        Write-Metric "End Time" $EndTime.ToString('HH:mm:ss.fff')

        # Count compiled crates
        $CompiledCrates = ($BuildOutput | Select-String "Compiling" | Measure-Object).Count
        if ($CompiledCrates -gt 0) {
            Write-Metric "Crates Compiled" $CompiledCrates
            $TimePerCrate = [math]::Round($Duration.TotalSeconds / $CompiledCrates, 2)
            Write-Metric "Avg Time/Crate" "$TimePerCrate" "seconds"
        }

        # Check target directory size
        if (Test-Path "target/$Profile") {
            $TargetSize = (Get-ChildItem -Recurse "target/$Profile" | Measure-Object -Property Length -Sum).Sum / 1MB
            Write-Metric "Target Size" ([math]::Round($TargetSize, 2)) "MB"
        }

    } else {
        Write-Host "  ✗ Build FAILED" -ForegroundColor Red
        Write-Host "  Duration: $(Format-Duration $Duration)" -ForegroundColor Yellow
        if ($Verbose) {
            Write-Host "`nBuild Output:" -ForegroundColor Red
            $BuildOutput | ForEach-Object { Write-Host $_ }
        }
    }

    return @{
        Type = $BuildType
        Profile = $Profile
        Success = $BuildSuccess
        Duration = $Duration
        StartTime = $StartTime
        EndTime = $EndTime
        CratesCompiled = $CompiledCrates
    }
}

function Measure-TestTime {
    param([string]$TestType = "all")

    Write-Header "Test Execution Benchmark"

    $StartTime = Get-Date
    Write-Host "  Starting tests at: $($StartTime.ToString('HH:mm:ss.fff'))" -ForegroundColor Gray

    # Run tests
    $TestArgs = @("test")
    if ($TestType -eq "nextest" -and (Get-Command "cargo-nextest" -ErrorAction SilentlyContinue)) {
        $TestArgs = @("nextest", "run")
        Write-Host "  Using cargo-nextest for faster execution" -ForegroundColor Green
    }

    if ($Profile -eq "release") {
        $TestArgs += "--release"
    }

    if ($Verbose) {
        Write-Host "  Command: cargo $($TestArgs -join ' ')" -ForegroundColor DarkGray
    }

    $TestOutput = & cargo @TestArgs 2>&1
    $TestSuccess = $LASTEXITCODE -eq 0

    $EndTime = Get-Date
    $Duration = $EndTime - $StartTime

    # Parse test results
    $TestsPassed = 0
    $TestsFailed = 0
    $TestsIgnored = 0

    foreach ($line in $TestOutput) {
        if ($line -match "(\d+) passed.*(\d+) failed.*(\d+) ignored") {
            $TestsPassed = [int]$Matches[1]
            $TestsFailed = [int]$Matches[2]
            $TestsIgnored = [int]$Matches[3]
            break
        }
    }

    # Display results
    if ($TestSuccess) {
        Write-Metric "Status" "SUCCESS"
        Write-Metric "Duration" (Format-Duration $Duration)
        Write-Metric "Tests Passed" $TestsPassed
        Write-Metric "Tests Failed" $TestsFailed
        Write-Metric "Tests Ignored" $TestsIgnored

        if ($TestsPassed -gt 0) {
            $TimePerTest = [math]::Round($Duration.TotalMilliseconds / $TestsPassed, 2)
            Write-Metric "Avg Time/Test" "$TimePerTest" "ms"
        }
    } else {
        Write-Host "  ✗ Tests FAILED" -ForegroundColor Red
        Write-Host "  Duration: $(Format-Duration $Duration)" -ForegroundColor Yellow
    }

    return @{
        Type = "Test"
        Success = $TestSuccess
        Duration = $Duration
        TestsPassed = $TestsPassed
        TestsFailed = $TestsFailed
        TestsIgnored = $TestsIgnored
    }
}

function Measure-IncrementalBuild {
    param([string]$Profile)

    Write-Header "Incremental Build Benchmark"

    # First ensure we have a baseline build
    Write-Host "  Ensuring baseline build exists..." -ForegroundColor Gray
    $null = & cargo build $(if ($Profile -eq "release") { "--release" }) 2>&1

    # Make a small change to trigger incremental build
    $TestFile = "shared/winpath/src/lib.rs"
    if (Test-Path $TestFile) {
        Write-Host "  Making small change to trigger incremental compilation..." -ForegroundColor Gray
        $Content = Get-Content $TestFile -Raw
        $ModifiedContent = $Content + "`n// Benchmark timestamp: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss.fff')"
        Set-Content -Path $TestFile -Value $ModifiedContent

        # Measure incremental build
        $Result = Measure-BuildTime -BuildType "Incremental" -Profile $Profile

        # Restore original content
        Set-Content -Path $TestFile -Value $Content

        return $Result
    } else {
        Write-Host "  Warning: Could not find test file for incremental build test" -ForegroundColor Yellow
        return $null
    }
}

# Main execution
$Results = @()

if ($All -or $CleanBuild) {
    $Results += Measure-BuildTime -BuildType "Clean" -Profile $Profile
}

if ($All -or $IncrementalBuild) {
    $Results += Measure-IncrementalBuild -Profile $Profile
}

if ($All -or $TestExecution) {
    $Results += Measure-TestTime
}

# Summary
if ($Results.Count -gt 1) {
    Write-Header "Performance Summary"

    $TotalDuration = [TimeSpan]::Zero
    foreach ($result in $Results) {
        if ($result.Success) {
            $TotalDuration = $TotalDuration.Add($result.Duration)
            Write-Metric "$($result.Type) Build" (Format-Duration $result.Duration)
        }
    }

    Write-Metric "Total Time" (Format-Duration $TotalDuration)

    # Save results to JSON for tracking
    $ResultsFile = "benchmark-results-$(Get-Date -Format 'yyyyMMdd-HHmmss').json"
    $Results | ConvertTo-Json -Depth 10 | Set-Content $ResultsFile
    Write-Host "`n  Results saved to: $ResultsFile" -ForegroundColor Cyan
}

# Recommendations based on results
Write-Header "Optimization Opportunities"

$CleanBuildResult = $Results | Where-Object { $_.Type -eq "Clean" } | Select-Object -First 1
if ($CleanBuildResult -and $CleanBuildResult.Duration.TotalMinutes -gt 5) {
    Write-Host "  ⚠ Clean build time exceeds 5 minutes" -ForegroundColor Yellow
    Write-Host "    -> Consider using sccache for compilation caching" -ForegroundColor Gray
    Write-Host "    -> Consider using cargo-nextest for parallel test execution" -ForegroundColor Gray
}

$IncrementalResult = $Results | Where-Object { $_.Type -eq "Incremental" } | Select-Object -First 1
if ($IncrementalResult -and $IncrementalResult.Duration.TotalSeconds -gt 30) {
    Write-Host "  ⚠ Incremental build time exceeds 30 seconds" -ForegroundColor Yellow
    Write-Host "    -> Review dependency graph for unnecessary recompilations" -ForegroundColor Gray
    Write-Host "    -> Consider splitting large crates into smaller modules" -ForegroundColor Gray
}

Write-Host ""
