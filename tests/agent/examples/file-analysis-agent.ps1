#!/usr/bin/env pwsh
# file-analysis-agent.ps1
# Example: Using agent mode for automated file analysis

[CmdletBinding()]
param(
    [Parameter()]
    [string]$TargetDirectory = ".",

    [Parameter()]
    [string]$FilePattern = "*.rs",

    [Parameter()]
    [string]$Model = "Qwen/Qwen2.5-1.5B-Instruct"
)

. "$PSScriptRoot\..\..\..\scripts\utils\Get-ProjectPaths.ps1"

Write-Host "`n=== File Analysis Agent Example ===" -ForegroundColor Cyan
Write-Host "Analyzes files using autonomous ReAct agent`n" -ForegroundColor Gray

# Create MCP config for filesystem access
$mcpConfig = @{
    servers = @(
        @{
            id = "filesystem"
            name = "Filesystem Tools"
            source = @{
                type = "Process"
                command = "npx"
                args = @("-y", "@modelcontextprotocol/server-filesystem", $TargetDirectory)
            }
        }
    )
    auto_register_tools = $true
    tool_timeout_secs = 60
    max_concurrent_calls = 3
} | ConvertTo-Json -Depth 10

$mcpConfigPath = Join-Path $env:TEMP "file-analysis-mcp-config.json"
$mcpConfig | Set-Content $mcpConfigPath

Write-Host "MCP Config: $mcpConfigPath" -ForegroundColor Yellow
Write-Host "Target Directory: $TargetDirectory" -ForegroundColor Yellow
Write-Host "File Pattern: $FilePattern`n" -ForegroundColor Yellow

# Get binary
$binaryPath = Get-MistralRSBinary

# Prepare agent prompt
$prompt = @"
Analyze all $FilePattern files in the current directory:
1. List all matching files
2. For each file, count the lines of code
3. Identify the main purpose of each file
4. Create a summary report

Present findings in a structured format.
"@

Write-Host "Agent Prompt:" -ForegroundColor Cyan
Write-Host $prompt -ForegroundColor Gray
Write-Host "`nLaunching agent...`n" -ForegroundColor Green

# Launch agent (would be automated in production)
$cmdArgs = @(
    '--agent-mode',
    '--mcp-config', $mcpConfigPath,
    'plain', '-m', $Model
)

Write-Host "Running: $binaryPath $($cmdArgs -join ' ')" -ForegroundColor Gray
Write-Host "`nNote: Paste the prompt above when agent starts`n" -ForegroundColor Yellow

& $binaryPath @cmdArgs

# Cleanup
Remove-Item $mcpConfigPath -ErrorAction SilentlyContinue

Write-Host "`n=== Analysis Complete ===" -ForegroundColor Cyan
