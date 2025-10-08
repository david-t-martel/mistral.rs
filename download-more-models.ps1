# Download Additional Models for mistral.rs
# Downloads various sized models for different use cases

param(
    [ValidateSet("all", "small", "coding", "large", "vision")]
    [string]$Type = "all",
    [switch]$SkipExisting
)

$ErrorActionPreference = "Continue"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Model Download Script" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

# Model definitions
$models = @{
    small = @{
        name = "Qwen2.5-1.5B-Instruct"
        url = "https://huggingface.co/bartowski/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf"
        path = "C:\codedev\llm\.models\qwen2.5-1.5b-it-gguf\Qwen2.5-1.5B-Instruct-Q4_K_M.gguf"
        size = "1.0 GB"
        use = "Fast responses, simple queries"
    }
    coding = @{
        name = "Qwen2.5-Coder-3B"
        url = "https://huggingface.co/bartowski/Qwen2.5-Coder-3B-Instruct-GGUF/resolve/main/Qwen2.5-Coder-3B-Instruct-Q4_K_M.gguf"
        path = "C:\codedev\llm\.models\qwen2.5-coder-3b-gguf\Qwen2.5-Coder-3B-Instruct-Q4_K_M.gguf"
        size = "2.0 GB"
        use = "Code analysis, refactoring"
    }
    large = @{
        name = "Qwen2.5-7B-Instruct"
        url = "https://huggingface.co/bartowski/Qwen2.5-7B-Instruct-GGUF/resolve/main/Qwen2.5-7B-Instruct-Q4_K_M.gguf"
        path = "C:\codedev\llm\.models\qwen2.5-7b-it-gguf\Qwen2.5-7B-Instruct-Q4_K_M.gguf"
        size = "4.7 GB"
        use = "Complex reasoning, architecture review"
    }
    vision = @{
        name = "Qwen2-VL-2B"
        url = "https://huggingface.co/Qwen/Qwen2-VL-2B-Instruct"
        path = "C:\codedev\llm\.models\qwen2-vl-2b-it"
        size = "~2-3 GB (safetensors)"
        use = "Image analysis, screenshot understanding"
        note = "Requires HF CLI or manual download"
    }
}

function Download-Model {
    param($modelInfo, $modelType)

    Write-Host "[$modelType] $($modelInfo.name)" -ForegroundColor Yellow
    Write-Host "  Size: $($modelInfo.size)" -ForegroundColor Gray
    Write-Host "  Use: $($modelInfo.use)" -ForegroundColor Gray

    # Create directory
    $dir = Split-Path $modelInfo.path -Parent
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
        Write-Host "  Created directory: $dir" -ForegroundColor Gray
    }

    # Check if exists
    if ((Test-Path $modelInfo.path) -and -not $SkipExisting) {
        $size = [math]::Round((Get-Item $modelInfo.path).Length / 1GB, 2)
        Write-Host "  [OK] Already exists ($size GB)" -ForegroundColor Green
        return $true
    }

    if (Test-Path $modelInfo.path) {
        Write-Host "  [SKIP] Skipping existing file" -ForegroundColor Yellow
        return $true
    }

    # Handle special cases
    if ($modelInfo.note) {
        Write-Host "  [WARN] $($modelInfo.note)" -ForegroundColor Yellow
        return $false
    }

    # Download
    Write-Host "  Downloading..." -ForegroundColor Cyan
    $startTime = Get-Date

    try {
        Invoke-WebRequest -Uri $modelInfo.url -OutFile $modelInfo.path -UseBasicParsing

        $duration = ((Get-Date) - $startTime).TotalMinutes
        $size = [math]::Round((Get-Item $modelInfo.path).Length / 1GB, 2)

        Write-Host "  [OK] Downloaded successfully ($size GB in $([math]::Round($duration, 1)) min)" -ForegroundColor Green
        return $true
    } catch {
        Write-Host "  [ERROR] Download failed: $_" -ForegroundColor Red

        # Clean up partial download
        if (Test-Path $modelInfo.path) {
            Remove-Item $modelInfo.path -Force
        }
        return $false
    }
}

# Determine which models to download
$toDownload = @()

switch ($Type) {
    "all" { $toDownload = @("small", "coding", "large") }
    "small" { $toDownload = @("small") }
    "coding" { $toDownload = @("coding") }
    "large" { $toDownload = @("large") }
    "vision" { $toDownload = @("vision") }
}

Write-Host "Models to download: $($toDownload -join ', ')" -ForegroundColor White
Write-Host ""

# Download each model
$results = @{}
foreach ($type in $toDownload) {
    $modelInfo = $models[$type]
    $results[$type] = Download-Model -modelInfo $modelInfo -modelType $type
    Write-Host ""
}

# Summary
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Download Summary" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

foreach ($type in $toDownload) {
    $status = if ($results[$type]) { "[OK]" } else { "[FAIL]" }
    $color = if ($results[$type]) { "Green" } else { "Red" }
    Write-Host "$status $type : $($models[$type].name)" -ForegroundColor $color
}

Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Create launch scripts for each model" -ForegroundColor Gray
Write-Host "2. Test each model with mistral.rs" -ForegroundColor Gray
Write-Host "3. Update documentation with model details" -ForegroundColor Gray
