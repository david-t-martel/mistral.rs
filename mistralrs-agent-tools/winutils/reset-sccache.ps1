# Reset sccache cache
param([switch]$Confirm)

if (-not $Confirm) {
    Write-Host "This will clear the sccache cache. Use -Confirm to proceed." -ForegroundColor Yellow
    return
}

Write-Host "Stopping sccache server..." -ForegroundColor Cyan
sccache --stop-server

Write-Host "Clearing cache..." -ForegroundColor Cyan
$CacheDir = $env:SCCACHE_DIR
if ($CacheDir -and (Test-Path $CacheDir)) {
    Remove-Item -Recurse -Force "$CacheDir\*"
    Write-Host "Cache cleared: $CacheDir" -ForegroundColor Green
} else {
    Write-Host "Cache directory not found" -ForegroundColor Yellow
}

Write-Host "Starting sccache server..." -ForegroundColor Cyan
sccache --start-server
sccache --show-stats
