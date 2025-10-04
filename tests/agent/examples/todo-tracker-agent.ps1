#!/usr/bin/env pwsh
# todo-tracker-agent.ps1
# Example: Extract and track TODO comments using agent mode

[CmdletBinding()]
param(
    [Parameter()]
    [string]$ProjectDirectory = ".",

    [Parameter()]
    [string]$FilePattern = "*.rs",

    [Parameter()]
    [string]$OutputFile = "TODO-TRACKER.md",

    [Parameter()]
    [string]$Model = "Qwen/Qwen2.5-3B-Instruct"
)

. "$PSScriptRoot\..\..\..\scripts\utils\Get-ProjectPaths.ps1"

Write-Host "`n=== TODO Tracker Agent ===" -ForegroundColor Cyan
Write-Host "Extracts and organizes TODO comments from codebase`n" -ForegroundColor Gray

# Validate directory
if (-not (Test-Path $ProjectDirectory)) {
    Write-Error "Project directory not found: $ProjectDirectory"
    exit 1
}

Write-Host "Project: $ProjectDirectory" -ForegroundColor Yellow
Write-Host "Pattern: $FilePattern" -ForegroundColor Yellow
Write-Host "Output: $OutputFile" -ForegroundColor Yellow
Write-Host "Model: $Model`n" -ForegroundColor Yellow

# Create MCP config
$mcpConfig = @{
    servers = @(
        @{
            id = "filesystem"
            name = "Filesystem Tools"
            source = @{
                type = "Process"
                command = "npx"
                args = @("-y", "@modelcontextprotocol/server-filesystem", $ProjectDirectory)
            }
        },
        @{
            id = "time"
            name = "Time Tools"
            source = @{
                type = "Process"
                command = "npx"
                args = @("-y", "@modelcontextprotocol/server-time")
            }
        }
    )
    auto_register_tools = $true
    tool_timeout_secs = 90
    max_concurrent_calls = 5
} | ConvertTo-Json -Depth 10

$mcpConfigPath = Join-Path $env:TEMP "todo-tracker-mcp-config.json"
$mcpConfig | Set-Content $mcpConfigPath

# Prepare tracking prompt
$prompt = @"
Track all TODO comments in the codebase:

1. Search for all $FilePattern files in the current directory
2. For each file, read the contents and extract:
   - All TODO comments
   - Line numbers where they appear
   - Context around each TODO (what function/class it's in)
3. Categorize TODOs by:
   - Priority (if indicated: HIGH, MEDIUM, LOW)
   - Component (based on file/module)
4. Get current timestamp
5. Create a TODO tracking report in markdown format and write to '$OutputFile'

Report format:
# TODO Tracker Report
**Generated:** [timestamp]

## Summary
- Total TODOs found: X
- By Priority: HIGH (X), MEDIUM (X), LOW (X)

## TODOs by Component

### [Component Name]
- [ ] **Priority:** [priority] **File:** [file:line]
  - Description: [TODO text]
  - Context: [surrounding code context]

Be comprehensive and well-organized.
"@

Write-Host "Tracking Task:" -ForegroundColor Cyan
Write-Host $prompt -ForegroundColor Gray

# Get binary
$binaryPath = Get-MistralRSBinary

Write-Host "`nLaunching TODO tracker agent...`n" -ForegroundColor Green

$cmdArgs = @(
    '--agent-mode',
    '--mcp-config', $mcpConfigPath,
    'plain', '-m', $Model
)

Write-Host "Command: $binaryPath $($cmdArgs -join ' ')" -ForegroundColor Gray
Write-Host "`nNote: Paste the prompt above when agent starts`n" -ForegroundColor Yellow

& $binaryPath @cmdArgs

# Cleanup
Remove-Item $mcpConfigPath -ErrorAction SilentlyContinue

# Verify output
if (Test-Path $OutputFile) {
    Write-Host "`n✓ TODO tracker report generated: $OutputFile" -ForegroundColor Green

    # Show preview
    Write-Host "`nReport Preview:" -ForegroundColor Cyan
    Get-Content $OutputFile -TotalCount 20 | ForEach-Object {
        Write-Host "  $_" -ForegroundColor Gray
    }

    $totalLines = (Get-Content $OutputFile).Count
    if ($totalLines -gt 20) {
        Write-Host "  ... ($($totalLines - 20) more lines)" -ForegroundColor DarkGray
    }
} else {
    Write-Host "`n✗ TODO tracker report not created" -ForegroundColor Red
}

Write-Host "`n=== TODO Tracking Complete ===" -ForegroundColor Cyan
