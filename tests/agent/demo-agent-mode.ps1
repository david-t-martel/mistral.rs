#!/usr/bin/env pwsh
# demo-agent-mode.ps1
# Demonstrates ReAct agent mode with MCP tools (filesystem, time)

[CmdletBinding()]
param(
    [Parameter()]
    [string]$Model = "Qwen/Qwen2.5-1.5B-Instruct",  # Default to smallest model

    [Parameter()]
    [ValidateSet('gguf', 'plain')]
    [string]$ModelType = 'plain',

    [Parameter()]
    [string]$ModelPath = "",

    [Parameter()]
    [string]$ModelFile = "",

    [Parameter()]
    [switch]$Interactive,

    [Parameter()]
    [switch]$Verbose
)

# Source project paths utility
. "$PSScriptRoot\..\..\scripts\utils\Get-ProjectPaths.ps1"

Write-Host "`n=== Mistral.rs Agent Mode Demo ===" -ForegroundColor Cyan
Write-Host "Demonstrates autonomous ReAct-style reasoning with MCP tools`n" -ForegroundColor Gray

# Get binary path
$binaryPath = Get-MistralRSBinary
if (-not $binaryPath) {
    Write-Error "mistralrs-server binary not found. Build it with: make build-cuda-full"
    exit 1
}

Write-Host "Using binary: " -NoNewline -ForegroundColor Yellow
Write-Host $binaryPath -ForegroundColor Green

# Get MCP config path
$mcpConfigPath = Join-Path $PSScriptRoot "mcp-agent-demo-config.json"
if (-not (Test-Path $mcpConfigPath)) {
    Write-Error "MCP configuration not found at: $mcpConfigPath"
    exit 1
}

Write-Host "MCP config: " -NoNewline -ForegroundColor Yellow
Write-Host $mcpConfigPath -ForegroundColor Green

# Prepare model arguments
$modelArgs = @()

if ($ModelType -eq 'gguf') {
    if (-not $ModelPath -or -not $ModelFile) {
        Write-Error "For GGUF models, specify both -ModelPath and -ModelFile"
        exit 1
    }
    $modelArgs = @('gguf', '-m', $ModelPath, '-f', $ModelFile)
    Write-Host "Model: " -NoNewline -ForegroundColor Yellow
    Write-Host "$ModelPath/$ModelFile (GGUF)" -ForegroundColor Green
} else {
    $modelArgs = @('plain', '-m', $Model)
    Write-Host "Model: " -NoNewline -ForegroundColor Yellow
    Write-Host "$Model (HuggingFace)" -ForegroundColor Green
}

# Build command
$cmdArgs = @(
    '--agent-mode',
    '--mcp-config', $mcpConfigPath
) + $modelArgs

Write-Host "`nLaunching agent mode..." -ForegroundColor Cyan
Write-Host "Command: " -NoNewline -ForegroundColor Yellow
Write-Host "$binaryPath $($cmdArgs -join ' ')" -ForegroundColor Gray

if ($Verbose) {
    Write-Host "`n--- MCP Configuration ---" -ForegroundColor Cyan
    Get-Content $mcpConfigPath | Write-Host -ForegroundColor Gray
    Write-Host "-------------------------`n" -ForegroundColor Cyan
}

Write-Host "`nAgent mode starting..." -ForegroundColor Green
Write-Host "Available MCP Tools:" -ForegroundColor Cyan
Write-Host "  - Filesystem operations (read_file, write_file, list_directory, etc.)" -ForegroundColor Gray
Write-Host "  - Time operations (get_current_time, convert_timezone, etc.)" -ForegroundColor Gray

Write-Host "`nExample prompts to try:" -ForegroundColor Cyan
Write-Host "  1. 'List all .ps1 files in the scripts directory'" -ForegroundColor Gray
Write-Host "  2. 'What time is it in Tokyo right now?'" -ForegroundColor Gray
Write-Host "  3. 'Read the README.md file and summarize it'" -ForegroundColor Gray
Write-Host "  4. 'Create a test.txt file with hello world'" -ForegroundColor Gray
Write-Host "`nPress Ctrl+C to exit`n" -ForegroundColor Yellow
Write-Host "=" * 60 -ForegroundColor Cyan
Write-Host ""

if ($Interactive) {
    # Interactive mode - wait for user input before launching
    Write-Host "Press Enter to launch agent mode, or Ctrl+C to cancel..." -ForegroundColor Yellow
    Read-Host
}

# Launch agent mode
try {
    & $binaryPath @cmdArgs
} catch {
    Write-Error "Failed to launch agent mode: $_"
    exit 1
}

Write-Host "`n=== Agent Mode Demo Complete ===" -ForegroundColor Cyan
