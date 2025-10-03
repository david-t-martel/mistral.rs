# Start mistralrs-server with Gemma 2 2B IT model
# Automatically configures all required paths and DLLs

param(
    [int]$Port = 11434,
    [string]$ModelPath = "C:\codedev\llm\.models\gemma-2-2b-it-gguf\gemma-2-2b-it-Q4_K_M.gguf",
    [string]$ServeIp = "0.0.0.0",
    [switch]$EnableMCP,
    [string]$McpConfigPath = "C:\codedev\mcp\config.json"
)

$ErrorActionPreference = "Stop"

Write-Host "======================================" -ForegroundColor Cyan
Write-Host "Starting mistralrs-server" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

# Binary path
$BinaryPath = "C:\Users\david\.cargo\shared-target\release\mistralrs-server.exe"

if (-not (Test-Path $BinaryPath)) {
    Write-Host "Error: Binary not found at $BinaryPath" -ForegroundColor Red
    exit 1
}

if (-not (Test-Path $ModelPath)) {
    Write-Host "Error: Model not found at $ModelPath" -ForegroundColor Red
    exit 1
}

# Configuration
Write-Host "Configuration:" -ForegroundColor Yellow
Write-Host "  Binary: $BinaryPath" -ForegroundColor Gray
Write-Host "  Model: $ModelPath" -ForegroundColor Gray
Write-Host "  Port: $Port" -ForegroundColor Gray
Write-Host "  IP: $ServeIp" -ForegroundColor Gray
Write-Host ""

# Set PATH to include all required DLLs
Write-Host "Setting up environment..." -ForegroundColor Yellow

# CUDA and cuDNN
$env:CUDA_PATH = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9"
$env:CUDNN_PATH = "C:\Program Files\NVIDIA\CUDNN\v9.8"

# Build PATH with all required DLL locations
$dllPaths = @(
    "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9\bin",
    "C:\Program Files\NVIDIA\CUDNN\v9.8\bin\12.8",
    "C:\Program Files (x86)\Intel\oneAPI\2025.0\bin",
    "C:\Users\david\.local\bin"
)

$env:PATH = ($dllPaths -join ";") + ";$env:PATH"

Write-Host "  CUDA_PATH: $env:CUDA_PATH" -ForegroundColor Green
Write-Host "  CUDNN_PATH: $env:CUDNN_PATH" -ForegroundColor Green
Write-Host "  PATH configured with required DLLs" -ForegroundColor Green
Write-Host ""

# Launch server
Write-Host "Launching server..." -ForegroundColor Yellow
Write-Host ""
Write-Host "Command:" -ForegroundColor Gray
Write-Host "  $BinaryPath gguf -m `"$ModelPath`" -a gemma2 -p $Port --serve-ip $ServeIp" -ForegroundColor Cyan
Write-Host ""
Write-Host "--------------------------------------" -ForegroundColor Cyan
Write-Host ""

try {
    $args = @(
        "gguf",
        "-m", $ModelPath,
        "-a", "gemma2",
        "-p", $Port,
        "--serve-ip", $ServeIp
    )
    
    if ($EnableMCP) {
        if (Test-Path $McpConfigPath) {
            Write-Host "MCP enabled with config: $McpConfigPath" -ForegroundColor Green
            $args += "--mcp-config"
            $args += $McpConfigPath
        } else {
            Write-Host "Warning: MCP config not found at $McpConfigPath" -ForegroundColor Yellow
        }
    }
    
    & $BinaryPath @args
} catch {
    Write-Host ""
    Write-Host "Error starting server: $_" -ForegroundColor Red
    exit 1
}
