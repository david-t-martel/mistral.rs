# Launch Qwen 1.5B Instruct - Fast Response Model
# Optimized for quick queries and simple tasks
# VRAM: ~2 GB | Speed: 80-100 tokens/sec

param(
    [int]$Port = 8080,
    [switch]$EnableMCP
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Qwen 1.5B Fast Model Server" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Configuration
$binaryPath = "C:\Users\david\.cargo\shared-target\release\mistralrs-server.exe"
$modelPath = "C:\codedev\llm\.models\qwen2.5-1.5b-it-gguf\Qwen2.5-1.5B-Instruct-Q4_K_M.gguf"

# Verify files exist
if (-not (Test-Path $binaryPath)) {
    Write-Host "[ERROR] Binary not found: $binaryPath" -ForegroundColor Red
    exit 1
}

if (-not (Test-Path $modelPath)) {
    Write-Host "[ERROR] Model not found: $modelPath" -ForegroundColor Red
    exit 1
}

Write-Host "[INFO] Model: Qwen 2.5 1.5B Instruct" -ForegroundColor Green
Write-Host "[INFO] Size: 0.92 GB" -ForegroundColor Green
Write-Host "[INFO] VRAM: ~2 GB" -ForegroundColor Green
Write-Host "[INFO] Speed: 80-100 tok/s (estimated)" -ForegroundColor Green
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
