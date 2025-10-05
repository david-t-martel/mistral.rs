# WinUtils Build Script
# Comprehensive build system for Windows-optimized coreutils

param(
    [Parameter(Position=0)]
    [ValidateSet('all', 'coreutils', 'derive-utils', 'release', 'debug', 'install', 'test', 'clean')]
    [string]$Action = 'all',

    [Parameter()]
    [string]$InstallPath = "$env:USERPROFILE\.local\bin"
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "Continue"

# Set Rust optimization flags for Windows
$env:RUSTFLAGS = "-C target-cpu=native -C opt-level=3 -C lto=fat"
$env:CARGO_PROFILE_RELEASE_LTO = "true"
$env:CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "1"

function Write-Status {
    param([string]$Message, [string]$Type = "INFO")
    $colors = @{
        "INFO" = "Cyan"
        "SUCCESS" = "Green"
        "WARNING" = "Yellow"
        "ERROR" = "Red"
    }
    Write-Host "[$Type] " -ForegroundColor $colors[$Type] -NoNewline
    Write-Host $Message
}

function Build-WinPath {
    Write-Status "Building winpath library..."
    Push-Location "shared/winpath"
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "Failed to build winpath" }
    Pop-Location
    Write-Status "winpath built successfully" "SUCCESS"
}

function Build-DeriveUtils {
    Write-Status "Building derive-utils..."

    $utils = @("where", "which", "tree")
    foreach ($util in $utils) {
        Write-Status "Building $util..."
        Push-Location "derive-utils/$util"
        cargo build --release
        if ($LASTEXITCODE -ne 0) {
            Write-Status "Failed to build $util" "WARNING"
        } else {
            Write-Status "$util built successfully" "SUCCESS"
        }
        Pop-Location
    }
}

function Build-CoreUtils {
    Write-Status "Building coreutils (this will take a while)..."

    # Build all utilities in workspace
    cargo build --release --workspace

    if ($LASTEXITCODE -ne 0) {
        Write-Status "Some utilities failed to build, continuing..." "WARNING"
    }

    Write-Status "Coreutils build complete" "SUCCESS"
}

function Install-Binaries {
    Write-Status "Installing binaries to $InstallPath..."

    # Create install directory if it doesn't exist
    if (!(Test-Path $InstallPath)) {
        New-Item -ItemType Directory -Force -Path $InstallPath | Out-Null
        Write-Status "Created install directory: $InstallPath"
    }

    # Copy derive-utils binaries
    $deriveUtils = @(
        "target\release\where.exe",
        "target\release\which.exe",
        "target\release\tree.exe"
    )

    foreach ($binary in $deriveUtils) {
        if (Test-Path $binary) {
            $name = Split-Path $binary -Leaf
            Copy-Item $binary "$InstallPath\wu-$name" -Force
            Write-Status "Installed wu-$name"
        }
    }

    # Copy existing rg and fd
    if (Test-Path "coreutils\rg.exe") {
        Copy-Item "coreutils\rg.exe" "$InstallPath\wu-rg.exe" -Force
        Write-Status "Installed wu-rg.exe"
    }

    if (Test-Path "coreutils\fd.exe") {
        Copy-Item "coreutils\fd.exe" "$InstallPath\wu-fd.exe" -Force
        Write-Status "Installed wu-fd.exe"
    }

    # Copy coreutils binaries with wu- prefix
    $coreutilsBinaries = Get-ChildItem "target\release\*.exe" -Exclude @("where.exe", "which.exe", "tree.exe")
    foreach ($binary in $coreutilsBinaries) {
        $name = "wu-" + $binary.Name
        Copy-Item $binary.FullName "$InstallPath\$name" -Force
        Write-Status "Installed $name"
    }

    Write-Status "Installation complete. Binaries installed to: $InstallPath" "SUCCESS"
    Write-Status "Add $InstallPath to your PATH to use the utilities globally" "INFO"
}

function Run-Tests {
    Write-Status "Running tests..."
    cargo test --workspace
    if ($LASTEXITCODE -ne 0) {
        Write-Status "Some tests failed" "WARNING"
    } else {
        Write-Status "All tests passed" "SUCCESS"
    }
}

function Clean-Build {
    Write-Status "Cleaning build artifacts..."
    cargo clean
    Write-Status "Clean complete" "SUCCESS"
}

# Main execution
Write-Host ""
Write-Host "========================================" -ForegroundColor Blue
Write-Host " WinUtils Build System" -ForegroundColor White
Write-Host " Windows-Optimized Coreutils & Utilities" -ForegroundColor Gray
Write-Host "========================================" -ForegroundColor Blue
Write-Host ""

try {
    switch ($Action) {
        'all' {
            Build-WinPath
            Build-DeriveUtils
            Build-CoreUtils
            Install-Binaries
        }
        'coreutils' {
            Build-WinPath
            Build-CoreUtils
        }
        'derive-utils' {
            Build-DeriveUtils
        }
        'release' {
            Build-WinPath
            Build-DeriveUtils
            Build-CoreUtils
        }
        'debug' {
            $env:RUSTFLAGS = ""
            cargo build --workspace
        }
        'install' {
            Install-Binaries
        }
        'test' {
            Run-Tests
        }
        'clean' {
            Clean-Build
        }
    }

    Write-Host ""
    Write-Status "Build completed successfully!" "SUCCESS"

} catch {
    Write-Host ""
    Write-Status $_.Exception.Message "ERROR"
    exit 1
}
