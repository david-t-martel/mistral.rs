Param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Args
)

if ($env:AUTO_CLAUDE_ENABLED -ne "1") {
    Write-Host "auto-claude hook disabled; set AUTO_CLAUDE_ENABLED=1 to enable."
    exit 0
}

$autoClaudePath = Join-Path $env:USERPROFILE "bin/auto-claude.exe"

if (-not (Test-Path -LiteralPath $autoClaudePath)) {
    Write-Host "auto-claude.exe not found at $autoClaudePath; skipping hook."
    exit 0
}

& $autoClaudePath analyze --fix --fail-on-errors --target-files @Args
if ($LASTEXITCODE -ne 0) {
    Write-Host "auto-claude exited with $LASTEXITCODE; treating as non-blocking."
    exit 0
}

exit 0
