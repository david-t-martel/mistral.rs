# Launch Qwen 2.5 7B Instruct - Large Reasoning Model
# Optimized for complex reasoning, research, and architecture
# VRAM: ~8 GB | Speed: 25-40 tokens/sec

param(
    [int]$Port = 8080,
    [switch]$EnableMCP
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Qwen 2.5 7B Large Model Server" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Configuration
$binaryPath = "C:\Users\david\.cargo\shared-target\release\mistralrs-server.exe"
$modelPath = "C:\codedev\llm\.models\qwen2.5-7b-it-gguf\Qwen2.5-7B-Instruct-Q4_K_M.gguf"

# Verify files exist
if (-not (Test-Path $binaryPath)) {
    Write-Host "[ERROR] Binary not found: $binaryPath" -ForegroundColor Red
    exit 1
}

if (-not (Test-Path $modelPath)) {
    Write-Host "[ERROR] Model not found: $modelPath" -ForegroundColor Red
    Write-Host "[HINT] Run: .\download-more-models.ps1 -Type large" -ForegroundColor Yellow
    exit 1
}

Write-Host "[INFO] Model: Qwen 2.5 7B Instruct" -ForegroundColor Green
Write-Host "[INFO] Size: 4.36 GB" -ForegroundColor Green
Write-Host "[INFO] VRAM: ~8 GB" -ForegroundColor Green
Write-Host "[INFO] Speed: 25-40 tok/s (estimated)" -ForegroundColor Green
Write-Host "[INFO] Use Case: Complex reasoning, research" -ForegroundColor Green
Write-Host "[INFO] Port: $Port" -ForegroundColor Green
Write-Host ""

# Environment settings
$env:CUDA_VISIBLE_DEVICES = "0"
$env:RUST_LOG = "info"

if ($EnableMCP) {
    $env:MCP_CONFIG_PATH = "T:\projects\rust-mistral\mistral.rs\MCP_CONFIG.json"
    Write-Host "[INFO] MCP Integration: ENABLED" -ForegroundColor Yellow
    Write-Host "[INFO] Full toolset enabled for complex tasks" -ForegroundColor Yellow
} else {
    Write-Host "[INFO] MCP Integration: DISABLED" -ForegroundColor Gray
}

Write-Host ""
Write-Host "[WARN] This model requires ~8GB VRAM" -ForegroundColor Yellow
Write-Host "[INFO] Ensure no other GPU-intensive apps are running" -ForegroundColor Yellow
Write-Host ""
Write-Host "Starting server..." -ForegroundColor Cyan
Write-Host "Press Ctrl+C to stop" -ForegroundColor Gray
Write-Host ""

# Launch server
& $binaryPath --port $Port gguf -m $modelPath -t 4
