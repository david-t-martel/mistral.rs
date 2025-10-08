$cliPath = Join-Path $env:USERPROFILE ".claude\semantic_index\semantic_index_cli.py"
$modulePath = Join-Path $env:USERPROFILE ".claude\semantic_index\semantic_index_real.py"

if (-not (Test-Path -LiteralPath $cliPath)) {
    Write-Host "semantic_index CLI not found at $cliPath; skipping hook."
    exit 0
}

if (-not (Test-Path -LiteralPath $modulePath)) {
    Write-Host "semantic_index dependency not found at $modulePath; skipping hook."
    exit 0
}

if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
    Write-Host "uv command not available; skipping semantic_index hook."
    exit 0
}

& uv run --python 3.12 python $cliPath --staged
if ($LASTEXITCODE -ne 0) {
    Write-Host "semantic_index hook failed with exit code $LASTEXITCODE; treating as non-blocking."
    exit 0
}

exit 0
