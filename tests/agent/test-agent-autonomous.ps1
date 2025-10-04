#!/usr/bin/env pwsh
# test-agent-autonomous.ps1
# Automated test suite for ReAct agent mode
# Tests autonomous tool execution with MCP servers

[CmdletBinding()]
param(
    [Parameter()]
    [string]$Model = "Qwen/Qwen2.5-1.5B-Instruct",

    [Parameter()]
    [string]$OutputFormat = "console",  # console, json, markdown

    [Parameter()]
    [switch]$CI
)

# Source project paths utility
. "$PSScriptRoot\..\..\scripts\utils\Get-ProjectPaths.ps1"

# Initialize test results
$testResults = @{
    TestSuite = "Agent Autonomous Execution"
    Model = $Model
    Timestamp = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
    Tests = @()
    Summary = @{
        Total = 0
        Passed = 0
        Failed = 0
        Skipped = 0
    }
}

function Write-TestStatus {
    param(
        [string]$TestName,
        [string]$Status,  # PASS, FAIL, SKIP
        [string]$Message = "",
        [hashtable]$Details = @{}
    )

    $color = switch ($Status) {
        "PASS" { "Green" }
        "FAIL" { "Red" }
        "SKIP" { "Yellow" }
        default { "Gray" }
    }

    $testResult = @{
        Name = $TestName
        Status = $Status
        Message = $Message
        Details = $Details
        Timestamp = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
    }

    $testResults.Tests += $testResult
    $testResults.Summary.Total++

    switch ($Status) {
        "PASS" { $testResults.Summary.Passed++ }
        "FAIL" { $testResults.Summary.Failed++ }
        "SKIP" { $testResults.Summary.Skipped++ }
    }

    if ($OutputFormat -eq "console") {
        Write-Host "[$Status] " -NoNewline -ForegroundColor $color
        Write-Host $TestName -NoNewline -ForegroundColor White
        if ($Message) {
            Write-Host " - $Message" -ForegroundColor Gray
        } else {
            Write-Host ""
        }
    }
}

Write-Host "`n=== Agent Autonomous Execution Test Suite ===" -ForegroundColor Cyan
Write-Host "Model: $Model" -ForegroundColor Yellow
Write-Host "Output: $OutputFormat`n" -ForegroundColor Yellow

# Test 1: Binary availability
Write-Host "`n--- Pre-flight Checks ---" -ForegroundColor Cyan
try {
    $binaryPath = Get-MistralRSBinary
    Write-TestStatus -TestName "Binary Availability" -Status "PASS" -Message "Found at $binaryPath"
} catch {
    Write-TestStatus -TestName "Binary Availability" -Status "FAIL" -Message $_.Exception.Message
    Write-Host "`nCannot proceed without binary. Build with: make build-cuda-full" -ForegroundColor Red
    exit 1
}

# Test 2: MCP configuration validation
$mcpConfigPath = Join-Path $PSScriptRoot "mcp-agent-demo-config.json"
if (Test-Path $mcpConfigPath) {
    try {
        $mcpConfig = Get-Content $mcpConfigPath -Raw | ConvertFrom-Json
        $serverCount = $mcpConfig.servers.Count
        Write-TestStatus -TestName "MCP Configuration Valid" -Status "PASS" -Message "$serverCount servers configured"
    } catch {
        Write-TestStatus -TestName "MCP Configuration Valid" -Status "FAIL" -Message $_.Exception.Message
    }
} else {
    Write-TestStatus -TestName "MCP Configuration Valid" -Status "FAIL" -Message "Config not found at $mcpConfigPath"
}

# Test 3: Check npm/npx availability (required for MCP servers)
try {
    $npxVersion = npx --version 2>&1
    Write-TestStatus -TestName "NPX Available" -Status "PASS" -Message "Version: $npxVersion"
} catch {
    Write-TestStatus -TestName "NPX Available" -Status "FAIL" -Message "npx not found (required for MCP servers)"
}

Write-Host "`n--- Agent Mode Tests ---" -ForegroundColor Cyan
Write-Host "Note: These tests require interactive verification or API mode" -ForegroundColor Yellow
Write-Host "      Automated testing requires HTTP API endpoint`n" -ForegroundColor Yellow

# Test 4: Agent mode launches
Write-TestStatus -TestName "Agent Mode Launch" -Status "SKIP" -Message "Requires interactive mode"

# Test 5: Tool auto-discovery
Write-TestStatus -TestName "MCP Tool Auto-Discovery" -Status "SKIP" -Message "Requires runtime verification"

# Test 6: Multi-step reasoning
Write-TestStatus -TestName "Multi-Step Reasoning" -Status "SKIP" -Message "Requires interactive mode"

# Test 7: Tool execution
Write-TestStatus -TestName "Tool Auto-Execution" -Status "SKIP" -Message "Requires interactive mode"

# Test 8: Error handling
Write-TestStatus -TestName "Error Handling" -Status "SKIP" -Message "Requires interactive mode"

Write-Host "`n--- Integration Test Scenarios ---" -ForegroundColor Cyan
Write-Host "The following scenarios should be tested manually:`n" -ForegroundColor Gray

$scenarios = @(
    @{
        Name = "Filesystem Operations"
        Prompt = "List all .md files in the docs directory"
        ExpectedBehavior = "Should use list_directory or read_directory MCP tool"
    },
    @{
        Name = "Time Query"
        Prompt = "What time is it in New York?"
        ExpectedBehavior = "Should use get_current_time MCP tool with timezone"
    },
    @{
        Name = "File Reading"
        Prompt = "Read the README.md and tell me what this project does"
        ExpectedBehavior = "Should use read_file MCP tool then summarize"
    },
    @{
        Name = "Multi-Tool Workflow"
        Prompt = "Check current time and write it to a file named timestamp.txt"
        ExpectedBehavior = "Should use get_current_time then write_file"
    }
)

foreach ($scenario in $scenarios) {
    Write-Host "  Scenario: " -NoNewline -ForegroundColor Yellow
    Write-Host $scenario.Name -ForegroundColor White
    Write-Host "    Prompt: " -NoNewline -ForegroundColor Gray
    Write-Host $scenario.Prompt -ForegroundColor Cyan
    Write-Host "    Expected: " -NoNewline -ForegroundColor Gray
    Write-Host $scenario.ExpectedBehavior -ForegroundColor Green
    Write-Host ""

    Write-TestStatus -TestName "Scenario: $($scenario.Name)" -Status "SKIP" -Message "Manual verification required" -Details $scenario
}

# Generate summary
Write-Host "`n=== Test Summary ===" -ForegroundColor Cyan
Write-Host "Total:   " -NoNewline -ForegroundColor Yellow
Write-Host $testResults.Summary.Total -ForegroundColor White
Write-Host "Passed:  " -NoNewline -ForegroundColor Green
Write-Host $testResults.Summary.Passed -ForegroundColor White
Write-Host "Failed:  " -NoNewline -ForegroundColor Red
Write-Host $testResults.Summary.Failed -ForegroundColor White
Write-Host "Skipped: " -NoNewline -ForegroundColor Yellow
Write-Host $testResults.Summary.Skipped -ForegroundColor White

# Output results in requested format
$outputDir = Join-Path $PSScriptRoot ".." "results"
if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

switch ($OutputFormat) {
    "json" {
        $outputPath = Join-Path $outputDir "agent-test-results.json"
        $testResults | ConvertTo-Json -Depth 10 | Set-Content $outputPath
        Write-Host "`nJSON results saved to: " -NoNewline -ForegroundColor Yellow
        Write-Host $outputPath -ForegroundColor Green
    }
    "markdown" {
        $outputPath = Join-Path $outputDir "agent-test-results.md"
        $markdown = @"
# Agent Autonomous Execution Test Results

**Model:** $($testResults.Model)
**Timestamp:** $($testResults.Timestamp)

## Summary
- **Total Tests:** $($testResults.Summary.Total)
- **Passed:** $($testResults.Summary.Passed)
- **Failed:** $($testResults.Summary.Failed)
- **Skipped:** $($testResults.Summary.Skipped)

## Test Results

| Test Name | Status | Message |
|-----------|--------|---------|
"@
        foreach ($test in $testResults.Tests) {
            $statusIcon = switch ($test.Status) {
                "PASS" { "✅" }
                "FAIL" { "❌" }
                "SKIP" { "⏭️" }
                default { "❓" }
            }
            $markdown += "`n| $($test.Name) | $statusIcon $($test.Status) | $($test.Message) |"
        }

        $markdown += @"


## Manual Test Scenarios

The following scenarios require manual verification:

"@
        foreach ($scenario in $scenarios) {
            $markdown += @"

### $($scenario.Name)
- **Prompt:** $($scenario.Prompt)
- **Expected Behavior:** $($scenario.ExpectedBehavior)

"@
        }

        $markdown | Set-Content $outputPath
        Write-Host "`nMarkdown results saved to: " -NoNewline -ForegroundColor Yellow
        Write-Host $outputPath -ForegroundColor Green
    }
}

# Exit with appropriate code
if ($testResults.Summary.Failed -gt 0) {
    Write-Host "`nTests FAILED" -ForegroundColor Red
    exit 1
} elseif ($testResults.Summary.Passed -eq 0 -and $testResults.Summary.Skipped -gt 0) {
    Write-Host "`nAll tests skipped (requires interactive mode)" -ForegroundColor Yellow
    exit 0
} else {
    Write-Host "`nTests PASSED" -ForegroundColor Green
    exit 0
}
