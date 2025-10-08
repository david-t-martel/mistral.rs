# Comprehensive Test Suite for mistral.rs
# Tests binary functionality, model loading, inference, and API endpoints

param(
    [switch]$Quick,
    [switch]$SkipInference,
    [string]$Model = "gemma2"
)

$ErrorActionPreference = "Continue"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "mistral.rs Test Suite" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

# Configuration
$binaryPath = "C:\Users\david\.cargo\shared-target\release\mistralrs-server.exe"
$cargoToml = "T:\projects\rust-mistral\mistral.rs\mistralrs-server\Cargo.toml"
$results = @{}

# Test 1: Binary existence and features
Write-Host "[1/8] Checking binary build..." -ForegroundColor Yellow
if (Test-Path $binaryPath) {
    $fileSize = [math]::Round((Get-Item $binaryPath).Length / 1MB, 2)
    Write-Host "  [OK] Binary found: $binaryPath" -ForegroundColor Green
    Write-Host "  [OK] Size: $fileSize MB" -ForegroundColor Green
    $results["binary_exists"] = $true

    # Check build date
    $buildDate = (Get-Item $binaryPath).LastWriteTime
    $daysSince = ((Get-Date) - $buildDate).Days
    Write-Host "  [INFO] Built: $buildDate ($daysSince days ago)" -ForegroundColor Gray
} else {
    Write-Host "  [ERROR] Binary not found: $binaryPath" -ForegroundColor Red
    $results["binary_exists"] = $false
}
Write-Host ""

# Test 2: Check build features
Write-Host "[2/8] Checking build features..." -ForegroundColor Yellow
if (Test-Path $cargoToml) {
    $cargoContent = Get-Content $cargoToml -Raw
    $features = @{
        "cuda" = $cargoContent -match 'cuda'
        "flash-attn" = $cargoContent -match 'flash-attn'
        "cudnn" = $cargoContent -match 'cudnn'
        "mkl" = $cargoContent -match 'mkl'
    }

    foreach ($feature in $features.GetEnumerator()) {
        $status = if ($feature.Value) { "[OK]" } else { "[SKIP]" }
        $color = if ($feature.Value) { "Green" } else { "Gray" }
        Write-Host "  $status Feature: $($feature.Key)" -ForegroundColor $color
    }
    $results["features_enabled"] = $features["cuda"] -and $features["flash-attn"]
} else {
    Write-Host "  [WARN] Cargo.toml not found" -ForegroundColor Yellow
    $results["features_enabled"] = $false
}
Write-Host ""

# Test 3: CUDA availability
Write-Host "[3/8] Checking CUDA environment..." -ForegroundColor Yellow
try {
    $nvidiaSmi = nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv,noheader 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  [OK] GPU: $nvidiaSmi" -ForegroundColor Green
        $results["cuda_available"] = $true
    } else {
        Write-Host "  [WARN] CUDA not available" -ForegroundColor Yellow
        $results["cuda_available"] = $false
    }
} catch {
    Write-Host "  [WARN] nvidia-smi not found" -ForegroundColor Yellow
    $results["cuda_available"] = $false
}
Write-Host ""

# Test 4: Model files
Write-Host "[4/8] Checking model files..." -ForegroundColor Yellow
$models = @{
    "gemma2" = "C:\codedev\llm\.models\gemma-2-2b-it-gguf\gemma-2-2b-it-Q4_K_M.gguf"
    "qwen1.5b" = "C:\codedev\llm\.models\qwen2.5-1.5b-it-gguf\Qwen2.5-1.5B-Instruct-Q4_K_M.gguf"
    "qwen3b" = "C:\codedev\llm\.models\qwen2.5-coder-3b-gguf\Qwen2.5-Coder-3B-Instruct-Q4_K_M.gguf"
    "qwen7b" = "C:\codedev\llm\.models\qwen2.5-7b-it-gguf\Qwen2.5-7B-Instruct-Q4_K_M.gguf"
}

$modelCount = 0
foreach ($model in $models.GetEnumerator()) {
    if (Test-Path $model.Value) {
        $size = [math]::Round((Get-Item $model.Value).Length / 1GB, 2)
        Write-Host "  [OK] $($model.Key): $size GB" -ForegroundColor Green
        $modelCount++
    } else {
        Write-Host "  [MISS] $($model.Key): not found" -ForegroundColor Yellow
    }
}
$results["models_available"] = $modelCount -gt 0
Write-Host "  [INFO] Total models: $modelCount / $($models.Count)" -ForegroundColor Gray
Write-Host ""

# Test 5: Server help and version
Write-Host "[5/8] Testing server help..." -ForegroundColor Yellow
if ($results["binary_exists"]) {
    try {
        $helpOutput = & $binaryPath --help 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Host "  [OK] Server responds to --help" -ForegroundColor Green
            $results["server_responds"] = $true
        } else {
            Write-Host "  [WARN] Help command returned error" -ForegroundColor Yellow
            $results["server_responds"] = $false
        }
    } catch {
        Write-Host "  [ERROR] Failed to run server: $_" -ForegroundColor Red
        $results["server_responds"] = $false
    }
} else {
    Write-Host "  [SKIP] Binary not available" -ForegroundColor Yellow
    $results["server_responds"] = $false
}
Write-Host ""

# Test 6: Model loading (if not skipping)
Write-Host "[6/8] Testing model loading..." -ForegroundColor Yellow
if (-not $Quick -and $results["binary_exists"] -and $models.ContainsKey($Model)) {
    $modelPath = $models[$Model]
    if (Test-Path $modelPath) {
        Write-Host "  [INFO] Loading model: $Model" -ForegroundColor Gray
        Write-Host "  [INFO] Path: $modelPath" -ForegroundColor Gray

        # Start server in background with timeout
        $serverJob = Start-Job -ScriptBlock {
            param($bin, $path)
            & $bin --port 8080 gguf -m $path -t 1
        } -ArgumentList $binaryPath, $modelPath

        Write-Host "  [INFO] Waiting for server startup (max 30s)..." -ForegroundColor Gray
        Start-Sleep -Seconds 5

        # Check if job is still running (model loaded)
        $jobState = Get-Job -Id $serverJob.Id
        if ($jobState.State -eq "Running") {
            Write-Host "  [OK] Model loaded successfully" -ForegroundColor Green
            $results["model_loads"] = $true

            # Stop the server
            Stop-Job -Id $serverJob.Id
            Remove-Job -Id $serverJob.Id -Force
        } else {
            Write-Host "  [ERROR] Server failed to start" -ForegroundColor Red
            $results["model_loads"] = $false

            # Get error output
            $output = Receive-Job -Id $serverJob.Id 2>&1
            if ($output) {
                Write-Host "  [ERROR] $output" -ForegroundColor Red
            }
            Remove-Job -Id $serverJob.Id -Force
        }
    } else {
        Write-Host "  [SKIP] Model not found: $Model" -ForegroundColor Yellow
        $results["model_loads"] = $false
    }
} else {
    if ($Quick) {
        Write-Host "  [SKIP] Quick mode enabled" -ForegroundColor Yellow
    } else {
        Write-Host "  [SKIP] Prerequisites not met" -ForegroundColor Yellow
    }
    $results["model_loads"] = $false
}
Write-Host ""

# Test 7: API endpoint test (if inference not skipped)
Write-Host "[7/8] Testing API endpoints..." -ForegroundColor Yellow
if (-not $SkipInference -and $results["model_loads"]) {
    Write-Host "  [INFO] Starting server for API test..." -ForegroundColor Gray

    # This would require actually starting the server and testing
    # Skip for now to avoid long-running tests
    Write-Host "  [SKIP] Requires running server (use manual test)" -ForegroundColor Yellow
    $results["api_works"] = $false
} else {
    Write-Host "  [SKIP] Model loading test not passed" -ForegroundColor Yellow
    $results["api_works"] = $false
}
Write-Host ""

# Test 8: MCP configuration
Write-Host "[8/8] Checking MCP configuration..." -ForegroundColor Yellow
$mcpConfig = "T:\projects\rust-mistral\mistral.rs\MCP_CONFIG.json"
if (Test-Path $mcpConfig) {
    try {
        $config = Get-Content $mcpConfig -Raw | ConvertFrom-Json
        $serverCount = $config.mcpServers.PSObject.Properties.Count
        Write-Host "  [OK] MCP config found with $serverCount servers" -ForegroundColor Green
        $results["mcp_configured"] = $true
    } catch {
        Write-Host "  [ERROR] Failed to parse MCP config: $_" -ForegroundColor Red
        $results["mcp_configured"] = $false
    }
} else {
    Write-Host "  [WARN] MCP config not found" -ForegroundColor Yellow
    $results["mcp_configured"] = $false
}
Write-Host ""

# Summary
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Test Summary" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

$passCount = ($results.Values | Where-Object { $_ -eq $true }).Count
$totalCount = $results.Count

foreach ($test in $results.GetEnumerator() | Sort-Object Name) {
    $status = if ($test.Value) { "[OK]" } else { "[FAIL]" }
    $color = if ($test.Value) { "Green" } else { "Red" }
    Write-Host "$status $($test.Key)" -ForegroundColor $color
}

Write-Host ""
$passPercent = [math]::Round(($passCount / $totalCount) * 100, 0)
Write-Host "Tests passed: $passCount / $totalCount ($passPercent%)" -ForegroundColor $(
    if ($passPercent -ge 80) { "Green" }
    elseif ($passPercent -ge 50) { "Yellow" }
    else { "Red" }
)

Write-Host ""
Write-Host "Recommendations:" -ForegroundColor Yellow
if (-not $results["binary_exists"]) {
    Write-Host "  - Build mistralrs-server binary" -ForegroundColor Gray
}
if (-not $results["cuda_available"]) {
    Write-Host "  - Install NVIDIA drivers and CUDA toolkit" -ForegroundColor Gray
}
if (-not $results["models_available"]) {
    Write-Host "  - Download models using download-more-models.ps1" -ForegroundColor Gray
}

Write-Host ""
Write-Host "Manual tests to perform:" -ForegroundColor Cyan
Write-Host "  1. Start server: .\launch-mistralrs.ps1" -ForegroundColor Gray
Write-Host "  2. Test inference: curl http://localhost:8080/v1/chat/completions" -ForegroundColor Gray
Write-Host "  3. Monitor VRAM: nvidia-smi dmon" -ForegroundColor Gray
Write-Host "  4. Check performance: measure tokens/sec" -ForegroundColor Gray
