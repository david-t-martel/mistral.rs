# run-coverage.ps1
# Runs cargo-llvm-cov with proper environment configuration
# Ensures local target directory and disables sccache for coverage builds

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet('html', 'open', 'lcov', 'json', 'text', 'fast')]
    [string]$Mode = 'open',

    [Parameter(Mandatory=$false)]
    [string[]]$Packages = @(),

    [Parameter(Mandatory=$false)]
    [switch]$AllFeatures = $true,

    [Parameter(Mandatory=$false)]
    [switch]$Clean = $false
)

# Save original environment
$OriginalCargoTargetDir = $env:CARGO_TARGET_DIR
$OriginalRustcWrapper = $env:RUSTC_WRAPPER
$OriginalCargoIncremental = $env:CARGO_INCREMENTAL

try {
    Write-Host "[Coverage] Configuring environment..." -ForegroundColor Cyan

    # Force local target directory (override shared target)
    # Must remove the variable completely, not set to empty string
    Remove-Item Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue

    # Disable sccache for coverage builds (incompatible)
    # Set to empty string to override .cargo/config.toml
    $env:RUSTC_WRAPPER = ""

    # Enable incremental for coverage (cargo-llvm-cov handles this)
    $env:CARGO_INCREMENTAL = "1"

    Write-Host "  - Using local target directory: target/" -ForegroundColor Green
    Write-Host "  - sccache disabled" -ForegroundColor Green
    Write-Host "  - Incremental compilation enabled" -ForegroundColor Green

    if ($Clean) {
        Write-Host "`n[Coverage] Cleaning previous coverage data..." -ForegroundColor Cyan
        cargo llvm-cov clean
    }

    # Build package arguments
    $PackageArgs = @()
    if ($Packages.Count -gt 0) {
        foreach ($pkg in $Packages) {
            $PackageArgs += "-p"
            $PackageArgs += $pkg
        }
    } else {
        $PackageArgs += "--workspace"
    }

    # Build feature arguments
    $FeatureArgs = @()
    if ($AllFeatures) {
        $FeatureArgs += "--all-features"
    }

    # Build command based on mode
    $Args = @('llvm-cov') + $PackageArgs + $FeatureArgs

    switch ($Mode) {
        'html' {
            Write-Host "`n[Coverage] Generating HTML report..." -ForegroundColor Cyan
            $Args += "--html"
            $OutputMsg = "Coverage report: target/llvm-cov/html/index.html"
        }
        'open' {
            Write-Host "`n[Coverage] Generating and opening HTML report..." -ForegroundColor Cyan
            $Args += "--open"
            $OutputMsg = "Opening coverage report in browser..."
        }
        'lcov' {
            Write-Host "`n[Coverage] Generating LCOV report..." -ForegroundColor Cyan
            $Args += "--lcov"
            $Args += "--output-path"
            $Args += "lcov.info"
            $OutputMsg = "LCOV report: lcov.info"
        }
        'json' {
            Write-Host "`n[Coverage] Generating JSON report..." -ForegroundColor Cyan
            $Args += "--json"
            $Args += "--output-path"
            $Args += "coverage.json"
            $OutputMsg = "JSON report: coverage.json"
        }
        'text' {
            Write-Host "`n[Coverage] Generating text summary..." -ForegroundColor Cyan
            $Args += "--summary-only"
            $OutputMsg = "Coverage summary displayed above"
        }
        'fast' {
            Write-Host "`n[Coverage] Fast coverage (no pyo3 crates)..." -ForegroundColor Cyan
            # Override packages for fast mode
            $Args = @(
                "llvm-cov",
                "-p", "mistralrs-core",
                "-p", "mistralrs-agent-tools",
                "-p", "mistralrs-quant",
                "-p", "mistralrs-vision",
                "-p", "mistralrs-mcp",
                "--all-features",
                "--html",
                "--open"
            )
            $OutputMsg = "Fast coverage report (5 crates)"
        }
    }

    # Run cargo-llvm-cov
    Write-Host "  Command: cargo $($Args -join ' ')" -ForegroundColor Gray
    Write-Host ""

    & cargo @Args

    if ($LASTEXITCODE -eq 0) {
        Write-Host "`n[Coverage] Success!" -ForegroundColor Green
        Write-Host "  $OutputMsg" -ForegroundColor Green
    } else {
        Write-Host "`n[Coverage] Failed with exit code $LASTEXITCODE" -ForegroundColor Red
        exit $LASTEXITCODE
    }

} finally {
    # Restore original environment
    Write-Host "`n[Coverage] Restoring environment..." -ForegroundColor Cyan

    if ($null -eq $OriginalCargoTargetDir) {
        Remove-Item Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    } else {
        $env:CARGO_TARGET_DIR = $OriginalCargoTargetDir
    }

    if ($null -eq $OriginalRustcWrapper) {
        Remove-Item Env:\RUSTC_WRAPPER -ErrorAction SilentlyContinue
    } else {
        $env:RUSTC_WRAPPER = $OriginalRustcWrapper
    }

    if ($null -eq $OriginalCargoIncremental) {
        Remove-Item Env:\CARGO_INCREMENTAL -ErrorAction SilentlyContinue
    } else {
        $env:CARGO_INCREMENTAL = $OriginalCargoIncremental
    }

    Write-Host "  - Environment restored" -ForegroundColor Green
}

Write-Host ""
