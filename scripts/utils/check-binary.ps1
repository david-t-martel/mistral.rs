# Troubleshooting: Check binary dependencies
$ErrorActionPreference = 'Continue'
$exe = 'T:\projects\rust-mistral\mistral.rs\target\release\mistralrs-server.exe'

Write-Host "=== Binary Dependency Check ===" -ForegroundColor Cyan
Write-Host "Binary: $exe" -ForegroundColor Gray
Write-Host ""

# Check if binary exists
if (-not (Test-Path $exe)) {
    Write-Host "✗ Binary not found!" -ForegroundColor Red
    exit 1
}

$fileInfo = Get-Item $exe
Write-Host "✓ Binary exists" -ForegroundColor Green
Write-Host "  Size: $([math]::Round($fileInfo.Length/1MB, 1)) MB" -ForegroundColor Gray
Write-Host "  Modified: $($fileInfo.LastWriteTime)" -ForegroundColor Gray
Write-Host ""

# Check CUDA_PATH
Write-Host "Environment Variables:" -ForegroundColor Yellow
Write-Host "  CUDA_PATH: $env:CUDA_PATH" -ForegroundColor Gray
Write-Host "  CUDNN_PATH: $env:CUDNN_PATH" -ForegroundColor Gray
Write-Host ""

# Try to run with --help to see if it works
Write-Host "Testing with --help flag..." -ForegroundColor Yellow
$helpOutput = & $exe --help 2>&1
$helpExitCode = $LASTEXITCODE

if ($helpExitCode -eq 0) {
    Write-Host "✓ Binary runs successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "First 20 lines of help output:" -ForegroundColor Cyan
    ($helpOutput | Select-Object -First 20) -join "`n" | Write-Host
} else {
    Write-Host "✗ Binary failed with exit code: $helpExitCode (0x$($helpExitCode.ToString('X')))" -ForegroundColor Red

    # Decode common error codes
    switch ($helpExitCode) {
        3221225781 { Write-Host "  ERROR: 0xC0000135 - DLL dependency missing" -ForegroundColor Red }
        3221225501 { Write-Host "  ERROR: 0xC000007B - Architecture mismatch (32/64-bit)" -ForegroundColor Red }
        default { Write-Host "  Unknown error code" -ForegroundColor Red }
    }

    Write-Host ""
    Write-Host "Common fixes:" -ForegroundColor Yellow
    Write-Host "  1. Ensure CUDA 12.9 is installed and on PATH" -ForegroundColor Gray
    Write-Host "  2. Check cuDNN 9.8 DLLs are accessible" -ForegroundColor Gray
    Write-Host "  3. Verify Visual C++ Redistributable is installed" -ForegroundColor Gray
    Write-Host "  4. Run 'setup-dev-env.ps1' to configure environment" -ForegroundColor Gray

    Write-Host ""
    Write-Host "Error output:" -ForegroundColor Red
    $helpOutput | Write-Host
}

Write-Host ""
Write-Host "=== Check Complete ===" -ForegroundColor Cyan

# Save results
$result = @{
    binary_exists = (Test-Path $exe)
    binary_size_mb = [math]::Round($fileInfo.Length/1MB, 1)
    help_exit_code = $helpExitCode
    help_works = ($helpExitCode -eq 0)
    cuda_path = $env:CUDA_PATH
    cudnn_path = $env:CUDNN_PATH
    timestamp = (Get-Date -Format 'o')
}

$result | ConvertTo-Json -Depth 2 | Out-File -Encoding utf8 'T:\projects\rust-mistral\mistral.rs\BINARY_CHECK_RESULTS.json'
Write-Host "✓ Results saved to BINARY_CHECK_RESULTS.json" -ForegroundColor Green
