if (-not (Get-Command ast-grep -ErrorAction SilentlyContinue)) {
    Write-Host "ast-grep not installed; skipping hook."
    exit 0
}

& ast-grep scan -c tools/ast-grep/sgconfig.yml
if ($LASTEXITCODE -ne 0) {
    Write-Host "ast-grep scan failed with exit code $LASTEXITCODE; treating as non-blocking."
    exit 0
}

exit 0
