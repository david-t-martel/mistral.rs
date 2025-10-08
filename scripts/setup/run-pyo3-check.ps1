# Phase 4: PyO3 Bindings Check
# Check if Python bindings are built and functional

$ErrorActionPreference = 'Continue'
$projectRoot = 'T:\projects\rust-mistral\mistral.rs'
Set-Location $projectRoot

Write-Host "=== Phase 4: PyO3 Bindings Check ===" -ForegroundColor Cyan
Write-Host ""

# Check for built artifacts
$pyo3Cargo = Join-Path $projectRoot 'mistralrs-pyo3\Cargo.toml'
$wheelDir = Join-Path $projectRoot 'target\wheels'

if (Test-Path $pyo3Cargo) {
    Write-Host "✓ PyO3 crate exists" -ForegroundColor Green

    # Extract features from Cargo.toml
    $cargoContent = Get-Content $pyo3Cargo -Raw
    $featureLines = $cargoContent -split "`n" | Where-Object { $_ -match 'features|cuda|cudnn|mkl|flash-attn' }

    Write-Host ""
    Write-Host "Features found in Cargo.toml:" -ForegroundColor Yellow
    $featureLines | ForEach-Object { Write-Host "  $_" }

    # Check if wheel exists
    Write-Host ""
    if (Test-Path $wheelDir) {
        $wheels = Get-ChildItem $wheelDir -Filter "*.whl" -ErrorAction SilentlyContinue
        if ($wheels) {
            Write-Host "✓ Wheel(s) found:" -ForegroundColor Green
            $wheels | ForEach-Object { Write-Host "  - $($_.Name) ($([math]::Round($_.Length/1MB, 1)) MB)" }
        } else {
            Write-Host "⚠ No wheel found in $wheelDir" -ForegroundColor Yellow
            Write-Host "  Wheel needs building with: maturin build --release" -ForegroundColor Gray
        }
    } else {
        Write-Host "⚠ Wheel directory doesn't exist: $wheelDir" -ForegroundColor Yellow
    }

    # Check for .pyd in release
    Write-Host ""
    $pydFiles = Get-ChildItem (Join-Path $projectRoot 'target\release') -Filter "mistralrs*.pyd" -ErrorAction SilentlyContinue
    if ($pydFiles) {
        Write-Host "✓ Found .pyd file(s):" -ForegroundColor Green
        $pydFiles | ForEach-Object { Write-Host "  - $($_.Name)" }
    } else {
        Write-Host "⚠ No .pyd files found in target\release" -ForegroundColor Yellow
    }

    # Test Python import
    Write-Host ""
    Write-Host "Testing Python import..." -ForegroundColor Yellow
    $importTest = python -c "import sys; sys.path.insert(0, r'$projectRoot\target\release'); import mistralrs; print('✓ Import successful')" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host $importTest -ForegroundColor Green

        # Get version info
        $versionInfo = python -c "import sys; sys.path.insert(0, r'$projectRoot\target\release'); import mistralrs; print(f'Version: {getattr(mistralrs, '__version__', 'unknown')}')" 2>&1
        Write-Host $versionInfo -ForegroundColor Gray
    } else {
        Write-Host "✗ Import failed:" -ForegroundColor Red
        Write-Host $importTest -ForegroundColor Red
    }

} else {
    Write-Host "✗ PyO3 crate not found at $pyo3Cargo" -ForegroundColor Red
}

# Document status
Write-Host ""
Write-Host "Creating status report..." -ForegroundColor Gray
$status = @{
    pyo3_crate_present = (Test-Path $pyo3Cargo)
    wheel_directory_exists = (Test-Path $wheelDir)
    wheels_built = if (Test-Path $wheelDir) {
        (Get-ChildItem $wheelDir -Filter "*.whl" -ErrorAction SilentlyContinue).Count
    } else { 0 }
    pyd_files = if (Test-Path (Join-Path $projectRoot 'target\release')) {
        (Get-ChildItem (Join-Path $projectRoot 'target\release') -Filter "mistralrs*.pyd" -ErrorAction SilentlyContinue).Count
    } else { 0 }
    import_test_passed = ($LASTEXITCODE -eq 0)
    features_checked = @("cuda", "cudnn", "flash-attn", "mkl")
    timestamp = (Get-Date -Format 'o')
    status = if ((Test-Path $pyo3Cargo) -and ($LASTEXITCODE -eq 0)) { 'AVAILABLE' } elseif (Test-Path $pyo3Cargo) { 'NEEDS_BUILD' } else { 'NOT_FOUND' }
}

$statusFile = Join-Path $projectRoot 'PYO3_STATUS_REPORT.json'
$status | ConvertTo-Json -Depth 3 | Out-File -Encoding utf8 $statusFile
Write-Host "✓ Status saved to PYO3_STATUS_REPORT.json" -ForegroundColor Green

Write-Host ""
Write-Host "=== Phase 4 Complete ===" -ForegroundColor Cyan
Write-Host "Status: $($status.status)" -ForegroundColor $(if ($status.status -eq 'AVAILABLE') { 'Green' } else { 'Yellow' })
