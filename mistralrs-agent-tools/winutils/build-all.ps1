<#
.SYNOPSIS
    Unified build script for all winutils utilities
.DESCRIPTION
    Builds all 78+ utilities with optimized settings for Windows
.PARAMETER Profile
    Build profile: Debug, Release, or ReleaseFast
.PARAMETER Parallel
    Number of parallel jobs (default: number of CPU cores)
.PARAMETER Utilities
    Specific utilities to build (default: all)
.PARAMETER Features
    Additional features to enable
.PARAMETER Clean
    Clean build artifacts before building
.PARAMETER Test
    Run tests after building
#>

param(
    [ValidateSet("Debug", "Release", "ReleaseFast")]
    [string]$Profile = "Release",

    [int]$Parallel = 0,

    [string[]]$Utilities = @(),

    [string[]]$Features = @(),

    [switch]$Clean,

    [switch]$Test,

    [switch]$Verbose
)

$ErrorActionPreference = "Stop"
$ProjectRoot = "T:\projects\coreutils\winutils"

# Color output functions
function Write-Header {
    param([string]$Message)
    Write-Host "`n╔═══════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║  $($Message.PadRight(61))║" -ForegroundColor Cyan
    Write-Host "╚═══════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
}

function Write-Status {
    param([string]$Message)
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Message)
    Write-Host "[✓] $Message" -ForegroundColor Green
}

function Write-ErrorMessage {
    param([string]$Message)
    Write-Host "[✗] $Message" -ForegroundColor Red
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[!] $Message" -ForegroundColor Yellow
}

# Get system info
function Get-SystemInfo {
    $cpuCount = (Get-CimInstance -ClassName Win32_Processor).NumberOfLogicalProcessors
    $memory = [math]::Round((Get-CimInstance -ClassName Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 2)

    return @{
        CPUCores = $cpuCount
        MemoryGB = $memory
        Platform = "x86_64-pc-windows-msvc"
    }
}

# Clean build artifacts
function Invoke-Clean {
    Write-Status "Cleaning build artifacts..."

    Push-Location $ProjectRoot
    try {
        if (Test-Path "target") {
            Remove-Item -Path "target" -Recurse -Force
            Write-Success "Cleaned target directory"
        }

        if (Test-Path "Cargo.lock") {
            Remove-Item -Path "Cargo.lock" -Force
            Write-Success "Removed Cargo.lock"
        }
    } finally {
        Pop-Location
    }
}

# Build utilities
function Invoke-Build {
    param(
        [string]$ProfileName,
        [int]$Jobs,
        [string[]]$UtilList,
        [string[]]$FeatureList
    )

    Write-Status "Building utilities with profile: $ProfileName"

    $sysInfo = Get-SystemInfo
    Write-Host "  Platform: $($sysInfo.Platform)"
    Write-Host "  CPU Cores: $($sysInfo.CPUCores)"
    Write-Host "  Memory: $($sysInfo.MemoryGB) GB"

    # Determine parallel jobs
    if ($Jobs -eq 0) {
        $Jobs = $sysInfo.CPUCores
    }
    Write-Host "  Parallel Jobs: $Jobs"

    # Build profile mapping
    $profileFlag = switch ($ProfileName) {
        "Debug" { "" }
        "Release" { "--release" }
        "ReleaseFast" { "--profile release-fast" }
    }

    # Feature flags
    $featureFlags = ""
    if ($FeatureList.Count -gt 0) {
        $featureFlags = "--features " + ($FeatureList -join ",")
    } else {
        $featureFlags = "--all-features"
    }

    Push-Location $ProjectRoot
    try {
        $startTime = Get-Date

        # Build shared libraries first
        Write-Status "Building shared libraries..."

        $sharedLibs = @("winpath", "winutils-core")
        foreach ($lib in $sharedLibs) {
            Write-Host "  Building $lib..." -ForegroundColor Yellow
            $buildCmd = "cargo build --package $lib $profileFlag $featureFlags -j $Jobs"

            if ($Verbose) {
                Write-Host "    Command: $buildCmd" -ForegroundColor Gray
            }

            $result = Invoke-Expression $buildCmd 2>&1
            if ($LASTEXITCODE -eq 0) {
                Write-Success "  Built $lib"
            } else {
                Write-ErrorMessage "  Failed to build $lib"
                Write-Host $result -ForegroundColor Red
                return $false
            }
        }

        # Build utilities
        if ($UtilList.Count -eq 0) {
            Write-Status "Building all utilities..."
            $buildCmd = "cargo build --workspace $profileFlag $featureFlags -j $Jobs"
        } else {
            Write-Status "Building specific utilities: $($UtilList -join ', ')"
            $packageFlags = ($UtilList | ForEach-Object { "--package $_" }) -join " "
            $buildCmd = "cargo build $packageFlags $profileFlag $featureFlags -j $Jobs"
        }

        if ($Verbose) {
            Write-Host "  Command: $buildCmd" -ForegroundColor Gray
        }

        $result = Invoke-Expression $buildCmd 2>&1
        if ($LASTEXITCODE -eq 0) {
            $elapsed = (Get-Date) - $startTime
            Write-Success "Build completed in $($elapsed.TotalSeconds.ToString('F2')) seconds"
            return $true
        } else {
            Write-ErrorMessage "Build failed"
            Write-Host $result -ForegroundColor Red
            return $false
        }
    } finally {
        Pop-Location
    }
}

# Run tests
function Invoke-Tests {
    Write-Status "Running tests..."

    Push-Location $ProjectRoot
    try {
        $testCmd = "cargo test --workspace --all-features"
        if ($Verbose) {
            $testCmd += " -- --nocapture"
        }

        $result = Invoke-Expression $testCmd 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Success "All tests passed"
            return $true
        } else {
            Write-ErrorMessage "Some tests failed"
            Write-Host $result -ForegroundColor Red
            return $false
        }
    } finally {
        Pop-Location
    }
}

# List built binaries
function Show-BuildResults {
    param([string]$ProfileName)

    Write-Status "Listing built binaries..."

    $targetDir = switch ($ProfileName) {
        "Debug" { "target\debug" }
        "Release" { "target\release" }
        "ReleaseFast" { "target\release-fast" }
    }

    $binaryPath = Join-Path $ProjectRoot $targetDir
    if (Test-Path $binaryPath) {
        $binaries = Get-ChildItem -Path $binaryPath -Filter "*.exe" | Where-Object { $_.Name -notmatch "(build|deps)" }
        $totalSize = ($binaries | Measure-Object -Property Length -Sum).Sum / 1MB

        Write-Host "`nBuilt $($binaries.Count) utilities (Total size: $($totalSize.ToString('F2')) MB):" -ForegroundColor Green
        Write-Host ""

        $binaries | Sort-Object Name | ForEach-Object {
            $size = $_.Length / 1KB
            Write-Host "  $($_.Name.PadRight(30)) $($size.ToString('F1').PadLeft(10)) KB"
        }

        Write-Host ""
        Write-Host "Output directory: $binaryPath" -ForegroundColor Cyan
    } else {
        Write-Warning "Build directory not found: $binaryPath"
    }
}

# Main execution
function Main {
    Write-Header "WinUtils Unified Build System"

    Write-Host @"

Build Configuration:
  Profile:    $Profile
  Parallel:   $Parallel (0 = auto)
  Clean:      $Clean
  Test:       $Test
  Utilities:  $(if ($Utilities.Count -eq 0) { "All" } else { $Utilities -join ", " })
  Features:   $(if ($Features.Count -eq 0) { "All" } else { $Features -join ", " })

"@

    # Change to project root
    Push-Location $ProjectRoot
    try {
        # Clean if requested
        if ($Clean) {
            Invoke-Clean
        }

        # Apply fixes first
        if (Test-Path "fix-compilation-errors.ps1") {
            Write-Status "Applying compilation fixes..."
            & ".\fix-compilation-errors.ps1"
            Write-Host ""
        }

        # Build
        $buildSuccess = Invoke-Build -ProfileName $Profile -Jobs $Parallel -UtilList $Utilities -FeatureList $Features

        if (-not $buildSuccess) {
            Write-ErrorMessage "Build failed!"
            exit 1
        }

        # Show results
        Show-BuildResults -ProfileName $Profile

        # Run tests if requested
        if ($Test) {
            $testSuccess = Invoke-Tests
            if (-not $testSuccess) {
                Write-ErrorMessage "Tests failed!"
                exit 1
            }
        }

        Write-Header "Build Complete"
        Write-Host @"

╔═══════════════════════════════════════════════════════════════╗
║                    ✓ BUILD SUCCESSFUL                         ║
║                                                               ║
║  All utilities compiled successfully.                         ║
║                                                               ║
║  Next steps:                                                  ║
║    • Install binaries: cargo install --path .                 ║
║    • Run tests: cargo test --workspace                        ║
║    • Generate docs: cargo doc --workspace --all-features      ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝

"@ -ForegroundColor Green

    } catch {
        Write-ErrorMessage "Build process failed with error: $_"
        exit 1
    } finally {
        Pop-Location
    }
}

# Run main
Main
