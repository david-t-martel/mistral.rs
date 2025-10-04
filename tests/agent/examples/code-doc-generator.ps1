#!/usr/bin/env pwsh
# code-doc-generator.ps1
# Example: Generate documentation for code files using agent mode

[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$SourceFile,

    [Parameter()]
    [string]$OutputFile,

    [Parameter()]
    [string]$Model = "Qwen/Qwen2.5-3B-Instruct"
)

. "$PSScriptRoot\..\..\..\scripts\utils\Get-ProjectPaths.ps1"

Write-Host "`n=== Code Documentation Generator Agent ===" -ForegroundColor Cyan

# Validate source file
if (-not (Test-Path $SourceFile)) {
    Write-Error "Source file not found: $SourceFile"
    exit 1
}

# Determine output file
if (-not $OutputFile) {
    $baseName = [System.IO.Path]::GetFileNameWithoutExtension($SourceFile)
    $OutputFile = "$baseName-docs.md"
}

Write-Host "Source: $SourceFile" -ForegroundColor Yellow
Write-Host "Output: $OutputFile" -ForegroundColor Yellow
Write-Host "Model: $Model`n" -ForegroundColor Yellow

# Create MCP config with filesystem access
$sourceDir = Split-Path $SourceFile -Parent
if (-not $sourceDir) { $sourceDir = "." }

$mcpConfig = @{
    servers = @(
        @{
            id = "filesystem"
            name = "Filesystem Tools"
            source = @{
                type = "Process"
                command = "npx"
                args = @("-y", "@modelcontextprotocol/server-filesystem", $sourceDir)
            }
        }
    )
    auto_register_tools = $true
    tool_timeout_secs = 120
} | ConvertTo-Json -Depth 10

$mcpConfigPath = Join-Path $env:TEMP "code-doc-mcp-config.json"
$mcpConfig | Set-Content $mcpConfigPath

# Prepare documentation prompt
$fileName = Split-Path $SourceFile -Leaf
$prompt = @"
Generate comprehensive documentation for the file '$fileName':

1. Read the file contents
2. Analyze the code structure:
   - Main purpose and functionality
   - Key functions/classes/modules
   - Important algorithms or patterns
   - Dependencies and imports
   - Public API (if applicable)
3. Create detailed markdown documentation
4. Write the documentation to '$OutputFile'

Format the documentation with:
- Title and overview
- Architecture explanation
- Function/class descriptions
- Usage examples (if applicable)
- Notes and caveats

Be thorough and technical.
"@

Write-Host "Documentation Task:" -ForegroundColor Cyan
Write-Host $prompt -ForegroundColor Gray

# Get binary
$binaryPath = Get-MistralRSBinary

Write-Host "`nLaunching documentation agent...`n" -ForegroundColor Green

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
    Write-Host "`n✓ Documentation generated: $OutputFile" -ForegroundColor Green
    $lines = (Get-Content $OutputFile).Count
    Write-Host "  Lines: $lines" -ForegroundColor Gray
} else {
    Write-Host "`n✗ Documentation file not created" -ForegroundColor Red
}

Write-Host "`n=== Documentation Generation Complete ===" -ForegroundColor Cyan
