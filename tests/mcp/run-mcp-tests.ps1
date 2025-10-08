# Phase 2: MCP Server Testing
# Test each configured MCP server

$ErrorActionPreference = 'Continue'
$projectRoot = 'T:\projects\rust-mistral\mistral.rs'
Set-Location $projectRoot

Write-Host "=== Phase 2: MCP Server Testing ===" -ForegroundColor Cyan
Write-Host ""

# Helper function to test MCP servers
function Test-McpServer {
    param(
        [string]$Name,
        [string]$Command,
        [string[]]$Args,
        [int]$TimeoutSec = 15
    )

    Write-Host "Testing: $Name" -ForegroundColor Yellow

    $job = Start-Job -ScriptBlock {
        param($cmd, $arglist)
        try {
            & $cmd @arglist 2>&1
        } catch {
            "ERROR: $($_.Exception.Message)"
        }
    } -ArgumentList $Command, $Args

    $completed = Wait-Job $job -Timeout $TimeoutSec

    if ($completed) {
        $output = Receive-Job $job -ErrorAction SilentlyContinue
        $hasError = ($job.State -eq 'Failed') -or ($output -match 'error|ERROR|exception')

        if (-not $hasError) {
            Write-Host "  ✓ $Name responded" -ForegroundColor Green
            $status = 'OK'
        } else {
            Write-Host "  ⚠ $Name error" -ForegroundColor Yellow
            $status = 'ERROR'
        }
    } else {
        Write-Host "  ⏱ $Name timeout" -ForegroundColor Yellow
        $status = 'TIMEOUT'
    }

    Stop-Job $job -ErrorAction SilentlyContinue
    Remove-Job $job -Force -ErrorAction SilentlyContinue

    return @{
        name = $Name
        status = $status
        tested_at = (Get-Date -Format 'o')
    }
}

Write-Host "Starting MCP server tests (15s timeout each)..." -ForegroundColor Gray
Write-Host ""

# Test each server
$mcpResults = @()

# 1. Memory
$mcpResults += Test-McpServer -Name 'Memory' -Command 'npx' -Args @('-y', '@modelcontextprotocol/server-memory@2025.8.4')

# 2. Filesystem
$mcpResults += Test-McpServer -Name 'Filesystem' -Command 'npx' -Args @('-y', '@modelcontextprotocol/server-filesystem@2025.8.21', 'T:/projects/rust-mistral/mistral.rs')

# 3. Sequential Thinking
$mcpResults += Test-McpServer -Name 'Sequential-Thinking' -Command 'npx' -Args @('-y', '@modelcontextprotocol/server-sequential-thinking@2025.7.1')

# 4. Fetch
$mcpResults += Test-McpServer -Name 'Fetch' -Command 'npx' -Args @('-y', '@modelcontextprotocol/server-fetch@0.6.3')

# 5. Time
$mcpResults += Test-McpServer -Name 'Time' -Command 'npx' -Args @('-y', '@theo.foobar/mcp-time')

Write-Host ""
Write-Host "=== Test Results ===" -ForegroundColor Cyan
$mcpResults | Format-Table -AutoSize name, status

# Count results
$total = $mcpResults.Count
$passed = ($mcpResults | Where-Object { $_.status -eq 'OK' }).Count
$timeout = ($mcpResults | Where-Object { $_.status -eq 'TIMEOUT' }).Count
$errors = ($mcpResults | Where-Object { $_.status -eq 'ERROR' }).Count

Write-Host ""
Write-Host "Summary:" -ForegroundColor Yellow
Write-Host "  Total: $total"Write-Host "  Passed: $passed" -ForegroundColor Green
Write-Host "  Timeouts: $timeout" -ForegroundColor Yellow
Write-Host "  Errors: $errors" -ForegroundColor Red

# Save results
$reportFile = Join-Path $projectRoot 'MCP_TEST_RESULTS.json'
$report = @{
    total = $total
    passed = $passed
    timeout = $timeout
    errors = $errors
    servers = $mcpResults
    timestamp = (Get-Date -Format 'o')
    overall_status = if ($passed -eq $total) { 'ALL_PASS' } elseif ($passed -ge ($total * 0.6)) { 'PARTIAL' } else { 'FAIL' }
}

$report | ConvertTo-Json -Depth 4 | Out-File -Encoding utf8 $reportFile
Write-Host ""
Write-Host "✓ Results saved to MCP_TEST_RESULTS.json" -ForegroundColor Green

# Cleanup any remaining node processes
Write-Host ""
Write-Host "Cleaning up processes..." -ForegroundColor Gray
Get-Process node, npx -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "=== Phase 2 Complete ===" -ForegroundColor Cyan
Write-Host "Status: $($report.overall_status)" -ForegroundColor $(if ($report.overall_status -eq 'ALL_PASS') { 'Green' } else { 'Yellow' })
