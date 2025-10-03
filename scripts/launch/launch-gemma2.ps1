# Launch Gemma 2 2B Instruct - Balanced Model
# Optimized for general use, chat, and document processing
# VRAM: ~3 GB | Speed: 60-80 tokens/sec

param(
    [int]$Port = 8080,
    [switch]$EnableMCP
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Gemma 2 2B Model Server" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Configuration
$binaryPath = "C:\Users\david\.cargo\shared-target\release\mistralrs-server.exe"
$modelPath = "C:\codedev\llm\.models\gemma-2-2b-it-gguf\gemma-2-2b-it-Q4_K_M.gguf"

# Verify files exist
if (-not (Test-Path $binaryPath)) {
    Write-Host "[ERROR] Binary not found: $binaryPath" -ForegroundColor Red
    exit 1
}

if (-not (Test-Path $modelPath)) {
    Write-Host "[ERROR] Model not found: $modelPath" -ForegroundColor Red
    exit 1
}

Write-Host "[INFO] Model: Gemma 2 2B Instruct" -ForegroundColor Green
Write-Host "[INFO] Size: 1.59 GB" -ForegroundColor Green
Write-Host "[INFO] VRAM: ~3 GB" -ForegroundColor Green
Write-Host "[INFO] Speed: 60-80 tok/s (estimated)" -ForegroundColor Green
Write-Host "[INFO] Use Case: General purpose, balanced" -ForegroundColor Green
Write-Host "[INFO] Port: $Port" -ForegroundColor Green
Write-Host ""

# Environment settings
$env:CUDA_VISIBLE_DEVICES = "0"
$env:RUST_LOG = "info"

if ($EnableMCP) {
    $env:MCP_CONFIG_PATH = "T:\projects\rust-mistral\mistral.rs\MCP_CONFIG.json"
    Write-Host "[INFO] MCP Integration: ENABLED" -ForegroundColor Yellow
} else {
    Write-Host "[INFO] MCP Integration: DISABLED" -ForegroundColor Gray
}

Write-Host ""
Write-Host "Starting server..." -ForegroundColor Cyan
Write-Host "Press Ctrl+C to stop" -ForegroundColor Gray
Write-Host ""

# Launch server
& $binaryPath --port $Port gguf -m $modelPath -t 4
