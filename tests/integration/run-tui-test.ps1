# Phase 3A: TUI Interactive Test
# Tests the mistralrs-server in interactive TUI mode

$ErrorActionPreference = 'Continue'
$projectRoot = 'T:\projects\rust-mistral\mistral.rs'
Set-Location $projectRoot
$logDir = Join-Path $projectRoot '.testlogs'
$exe = Join-Path $projectRoot 'target\release\mistralrs-server.exe'
$modelDir = 'C:\codedev\llm\.models\qwen2.5-1.5b-it-gguf'
$modelFile = 'Qwen2.5-1.5B-Instruct-Q4_K_M.gguf'

Write-Host "=== Phase 3A: TUI Interactive Test ===" -ForegroundColor Cyan
Write-Host "Model: Qwen2.5-1.5B-Instruct (0.94 GB)" -ForegroundColor Yellow
Write-Host "Executable: $exe" -ForegroundColor Gray
Write-Host "Starting mistralrs-server in interactive mode..." -ForegroundColor Yellow
Write-Host ""

# Create test input commands
$testInput = @"
Hello! Please respond with exactly the word 'test' and nothing else.
\help
\exit
"@

# Run with timeout (2 minutes max for small model)
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$job = Start-Job -ScriptBlock {
    param($exe, $modelDir, $modelFile, $input, $projectRoot)
    Set-Location $projectRoot
    $input | & $exe -i gguf -m $modelDir -f $modelFile 2>&1
} -ArgumentList $exe, $modelDir, $modelFile, $testInput, $projectRoot

Write-Host "Waiting for model to load and respond (max 120s)..." -ForegroundColor Gray

if (Wait-Job $job -Timeout 120) {
    $sw.Stop()
    $output = Receive-Job $job
    $outputText = $output | Out-String
    
    # Save full log
    $logFile = Join-Path $logDir 'tui-test.log'
    $outputText | Out-File -Encoding utf8 $logFile
    
    Write-Host "✓ TUI test completed in $([math]::Round($sw.Elapsed.TotalSeconds, 1))s" -ForegroundColor Green
    Write-Host ""
    
    # Display first 15 lines
    $lines = $outputText -split "`n"
    Write-Host "--- First 15 lines of output ---" -ForegroundColor Cyan
    $lines | Select-Object -First 15 | ForEach-Object { Write-Host $_ }
    
    Write-Host ""
    Write-Host "--- Last 15 lines of output ---" -ForegroundColor Cyan
    $lines | Select-Object -Last 15 | ForEach-Object { Write-Host $_ }
    
    # Validate results
    Write-Host ""
    Write-Host "=== Validation ===" -ForegroundColor Yellow
    $checks = @{
        'Model loaded' = ($outputText -match 'Loading|Loaded|Model')
        'Interactive mode' = ($outputText -match 'interactive|>>|>')
        'Response generated' = ($outputText -match 'test|response|assistant')
        'Help command' = ($outputText -match 'help|commands|usage')
        'Clean exit' = ($outputText -notmatch 'error|panic|failed' -or $outputText -match 'exit|quit')
    }
    
    $passed = 0
    $total = $checks.Count
    foreach ($check in $checks.GetEnumerator()) {
        $status = if ($check.Value) { '✓'; $passed++ } else { '✗' }
        $color = if ($check.Value) { 'Green' } else { 'Red' }
        Write-Host "  $status $($check.Key)" -ForegroundColor $color
    }
    
    Write-Host ""
    Write-Host "Results: $passed/$total checks passed" -ForegroundColor $(if ($passed -eq $total) { 'Green' } else { 'Yellow' })
    
    # Create result JSON
    $result = @{
        phase = '3A-TUI'
        model = 'Qwen2.5-1.5B-Instruct-Q4_K_M'
        duration_seconds = [math]::Round($sw.Elapsed.TotalSeconds, 1)
        checks_passed = $passed
        checks_total = $total
        log_file = $logFile
        log_lines = $lines.Count
        status = if ($passed -ge 3) { 'PASS' } else { 'FAIL' }
        timestamp = (Get-Date -Format 'o')
    }
    
    $resultFile = Join-Path $projectRoot 'TUI_TEST_RESULTS.json'
    $result | ConvertTo-Json -Depth 3 | Out-File -Encoding utf8 $resultFile
    Write-Host "✓ Results saved to TUI_TEST_RESULTS.json" -ForegroundColor Green
    
} else {
    $sw.Stop()
    Write-Host "✗ TUI test timeout after $([math]::Round($sw.Elapsed.TotalSeconds, 1))s" -ForegroundColor Red
    Stop-Job $job -ErrorAction SilentlyContinue
    
    # Try to get partial output
    $partialOutput = Receive-Job $job -ErrorAction SilentlyContinue
    if ($partialOutput) {
        $timeoutLog = Join-Path $logDir 'tui-test-timeout.log'
        $partialOutput | Out-String | Out-File -Encoding utf8 $timeoutLog
        Write-Host "Partial output saved to $timeoutLog" -ForegroundColor Yellow
    }
    
    $result = @{
        phase = '3A-TUI'
        status = 'TIMEOUT'
        duration_seconds = [math]::Round($sw.Elapsed.TotalSeconds, 1)
        timestamp = (Get-Date -Format 'o')
    }
    $resultFile = Join-Path $projectRoot 'TUI_TEST_RESULTS.json'
    $result | ConvertTo-Json | Out-File -Encoding utf8 $resultFile
}

Remove-Job $job -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "=== Phase 3A Complete ===" -ForegroundColor Cyan