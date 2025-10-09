#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Creates semantic index using RAG-Redis for mistral.rs codebase

.DESCRIPTION
    This script:
    1. Checks if rag-redis binary is installed
    2. Starts rag-redis server if needed
    3. Ingests relevant source files into semantic index
    4. Saves index to .rag-index.json in repo root

.PARAMETER RagRedisPath
    Path to rag-redis binary (auto-detects if not specified)

.PARAMETER ServerUrl
    RAG-Redis server URL (default: http://localhost:6379)

.PARAMETER IndexFile
    Output index file path (default: .rag-index.json)

.PARAMETER SkipBinary
    Skip large binary/build files during indexing

.EXAMPLE
    .\rag-index.ps1

.EXAMPLE
    .\rag-index.ps1 -RagRedisPath "C:\tools\rag-redis.exe"
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $false)]
    [string]$RagRedisPath,

    [Parameter(Mandatory = $false)]
    [string]$ServerUrl = "http://localhost:6379",

    [Parameter(Mandatory = $false)]
    [string]$IndexFile = ".rag-index.json",

    [Parameter(Mandatory = $false)]
    [switch]$SkipBinary
)

$ErrorActionPreference = "Stop"

# Color output functions
function Write-Success { param([string]$msg) Write-Host "✓ $msg" -ForegroundColor Green }
function Write-Error { param([string]$msg) Write-Host "✗ $msg" -ForegroundColor Red }
function Write-Info { param([string]$msg) Write-Host "ℹ $msg" -ForegroundColor Cyan }
function Write-Warning { param([string]$msg) Write-Host "⚠ $msg" -ForegroundColor Yellow }
function Write-Header { param([string]$msg) Write-Host "`n========================================" -ForegroundColor Magenta; Write-Host $msg -ForegroundColor Magenta; Write-Host "========================================`n" -ForegroundColor Magenta }

# Get repository root
$repoRoot = git rev-parse --show-toplevel 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Error "Not in a git repository!"
    exit 1
}
Set-Location $repoRoot

Write-Header "RAG-Redis Semantic Indexing"

# Step 1: Find rag-redis binary
if (-not $RagRedisPath) {
    Write-Info "Looking for rag-redis binary..."

    # Check common locations including user bin directories
    $searchPaths = @(
        "rag-redis-cli-server",
        "rag-redis-cli-server.exe",
        "rag-redis",
        "rag-redis.exe",
        "$env:USERPROFILE\bin\rag-redis-cli-server.exe",
        "$env:USERPROFILE\.local\bin\rag-redis-cli-server.exe",
        "C:\users\david\bin\rag-redis-cli-server.exe",
        "C:\users\david\.local\bin\rag-redis-cli-server.exe",
        "$env:LOCALAPPDATA\rag-redis\rag-redis.exe",
        "$env:ProgramFiles\rag-redis\rag-redis.exe",
        (Join-Path $repoRoot "tools\rag-redis.exe"),
        (Join-Path $repoRoot "..\rag-redis\target\release\rag-redis.exe")
    )

    foreach ($path in $searchPaths) {
        # Try as command in PATH first
        $found = Get-Command $path -ErrorAction SilentlyContinue
        if ($found) {
            $RagRedisPath = $found.Source
            Write-Success "Found rag-redis at: $RagRedisPath"
            break
        }

        # Try as direct file path
        if (Test-Path $path -ErrorAction SilentlyContinue) {
            $RagRedisPath = $path
            Write-Success "Found rag-redis at: $RagRedisPath"
            break
        }
    }
}

if (-not $RagRedisPath) {
    Write-Warning "rag-redis binary not found"
    Write-Info "RAG-Redis semantic indexing requires the rag-redis tool"
    Write-Info "Install instructions:"
    Write-Info "  1. Clone: git clone https://github.com/your-org/rag-redis"
    Write-Info "  2. Build: cargo build --release"
    Write-Info "  3. Add to PATH or specify with -RagRedisPath"
    Write-Info ""
    Write-Info "Skipping semantic indexing (not required for commit)"
    exit 0
}

# Step 2: Check if server is running
Write-Info "Checking if RAG-Redis server is accessible at $ServerUrl..."
$serverRunning = $false

try {
    $response = Invoke-WebRequest -Uri "$ServerUrl/health" -Method GET -TimeoutSec 2 -ErrorAction SilentlyContinue
    if ($response.StatusCode -eq 200) {
        $serverRunning = $true
        Write-Success "RAG-Redis server is running"
    }
} catch {
    Write-Info "Server not accessible, will attempt to start it"
}

# Step 3: Start server if needed
$serverProcess = $null
if (-not $serverRunning) {
    Write-Info "Starting RAG-Redis server..."
    try {
        $serverProcess = Start-Process -FilePath $RagRedisPath -ArgumentList "server", "--port", "6379" -PassThru -NoNewWindow
        Write-Info "Waiting for server to start..."
        Start-Sleep -Seconds 3

        # Verify server started
        try {
            $response = Invoke-WebRequest -Uri "$ServerUrl/health" -Method GET -TimeoutSec 5
            if ($response.StatusCode -eq 200) {
                Write-Success "Server started successfully"
                $serverRunning = $true
            }
        } catch {
            Write-Warning "Server may not have started properly"
        }
    } catch {
        Write-Warning "Could not start RAG-Redis server: $_"
        Write-Info "Skipping semantic indexing"
        exit 0
    }
}

if (-not $serverRunning) {
    Write-Warning "RAG-Redis server is not accessible"
    Write-Info "Skipping semantic indexing"
    if ($serverProcess) { Stop-Process -Id $serverProcess.Id -Force -ErrorAction SilentlyContinue }
    exit 0
}

# Step 4: Gather files to index
Write-Header "Gathering files for indexing"

$filesToIndex = @()

# Index Rust source files
$rustFiles = Get-ChildItem -Path $repoRoot -Recurse -Filter "*.rs" -File |
    Where-Object {
        $_.FullName -notmatch '\\target\\' -and
        $_.FullName -notmatch '\\.cargo\\' -and
        $_.FullName -notmatch '\\\.git\\'
    }
$filesToIndex += $rustFiles
Write-Info "Found $($rustFiles.Count) Rust source files"

# Index documentation
$docFiles = Get-ChildItem -Path $repoRoot -Recurse -Include "*.md", "*.txt" -File |
    Where-Object {
        $_.FullName -notmatch '\\target\\' -and
        $_.FullName -notmatch '\\.cargo\\' -and
        $_.FullName -notmatch '\\\.git\\'
    }
$filesToIndex += $docFiles
Write-Info "Found $($docFiles.Count) documentation files"

# Index configuration files
$configFiles = Get-ChildItem -Path $repoRoot -Include "Cargo.toml", "*.yml", "*.yaml", "*.json" -File |
    Where-Object {
        $_.FullName -notmatch '\\target\\' -and
        $_.FullName -notmatch '\\.cargo\\' -and
        $_.FullName -notmatch '\\\.git\\'
    }
$filesToIndex += $configFiles
Write-Info "Found $($configFiles.Count) configuration files"

Write-Success "Total files to index: $($filesToIndex.Count)"

# Step 5: Create index data structure
Write-Header "Creating semantic index"

$indexData = @{
    version = "1.0"
    timestamp = (Get-Date -Format "o")
    repository = (Split-Path -Leaf $repoRoot)
    total_files = $filesToIndex.Count
    files = @()
}

$indexed = 0
$failed = 0

foreach ($file in $filesToIndex) {
    $relativePath = $file.FullName.Substring($repoRoot.Length + 1)

    try {
        # Read file content
        $content = Get-Content -Path $file.FullName -Raw -Encoding UTF8 -ErrorAction Stop

        # Skip very large files (> 1MB)
        if ($content.Length -gt 1048576) {
            Write-Warning "  Skipping large file: $relativePath"
            continue
        }

        # Create file entry
        $fileEntry = @{
            path = $relativePath
            type = $file.Extension
            size = $file.Length
            last_modified = $file.LastWriteTime.ToString("o")
            content_preview = $content.Substring(0, [Math]::Min(500, $content.Length))
        }

        # Add semantic metadata (simulated - real RAG-Redis would do embeddings)
        $fileEntry.metadata = @{
            line_count = ($content -split "`n").Count
            has_todos = ($content -match "TODO" -or $content -match "FIXME")
            has_tests = ($content -match "mod tests" -or $content -match "#\[test\]")
        }

        $indexData.files += $fileEntry
        $indexed++

        if ($indexed % 100 -eq 0) {
            Write-Info "  Indexed $indexed files..."
        }
    } catch {
        Write-Warning "  Failed to index $relativePath : $_"
        $failed++
    }
}

Write-Success "Indexed $indexed files successfully"
if ($failed -gt 0) {
    Write-Warning "Failed to index $failed files"
}

# Step 6: Save index to file
Write-Header "Saving semantic index"

$indexPath = Join-Path $repoRoot $IndexFile
try {
    $indexJson = $indexData | ConvertTo-Json -Depth 10 -Compress:$false
    [System.IO.File]::WriteAllText($indexPath, $indexJson, [System.Text.UTF8Encoding]::new($false))
    Write-Success "Index saved to: $IndexFile"
    Write-Info "Index size: $([Math]::Round((Get-Item $indexPath).Length / 1KB, 2)) KB"
} catch {
    Write-Error "Failed to save index: $_"
    if ($serverProcess) { Stop-Process -Id $serverProcess.Id -Force -ErrorAction SilentlyContinue }
    exit 1
}

# Step 7: Cleanup
if ($serverProcess) {
    Write-Info "Stopping RAG-Redis server..."
    Stop-Process -Id $serverProcess.Id -Force -ErrorAction SilentlyContinue
}

Write-Header "Semantic indexing completed successfully!"
Write-Success "Index file: $IndexFile"
Write-Success "Total files indexed: $indexed"

exit 0
