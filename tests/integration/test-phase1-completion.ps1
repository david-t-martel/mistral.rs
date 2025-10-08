# Phase 1 Completion Test Script
# Tests CUDA initialization, API endpoints, and validates DLL dependencies
# Uses smallest model (Qwen 1.5B - 940 MB) for fast testing

param(
    [int]$Port = 8080,
    [int]$TimeoutSeconds = 120
)

$ErrorActionPreference = "Continue"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Phase 1 Completion Tests" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Configuration
$BinaryPath = "T:\projects\rust-mistral\mistral.rs\target\release\mistralrs-server.exe"
$ModelPath = "C:\codedev\llm\.models\qwen2.5-1.5b-it-gguf\Qwen2.5-1.5B-Instruct-Q4_K_M.gguf"
$ResultsFile = "T:\projects\rust-mistral\mistral.rs\PHASE1_COMPLETION_RESULTS.json"

# Set up environment with CUDA/cuDNN paths
$env:PATH = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9\bin;C:\Program Files\NVIDIA\CUDNN\v9.8\bin\12.8;C:\Program Files (x86)\Intel\oneAPI\2025.0\bin;C:\Users\david\.local\bin;$env:PATH"
$env:CUDA_PATH = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9"
$env:CUDNN_PATH = "C:\Program Files\NVIDIA\CUDNN\v9.8"

$results = @{
    timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    tests = @{}
}

# Test 1: Verify binary exists
Write-Host "Test 1: Binary Verification" -ForegroundColor Yellow
$results.tests.binaryVerification = @{}
if (Test-Path $BinaryPath) {
    $binary = Get-Item $BinaryPath
    Write-Host "  ✅ Binary found: $($binary.FullName)" -ForegroundColor Green
    Write-Host "  Size: $([math]::Round($binary.Length/1MB, 2)) MB" -ForegroundColor Gray
    $results.tests.binaryVerification.status = "PASSED"
    $results.tests.binaryVerification.size_mb = [math]::Round($binary.Length/1MB, 2)
} else {
    Write-Host "  ❌ Binary not found at $BinaryPath" -ForegroundColor Red
    $results.tests.binaryVerification.status = "FAILED"
    exit 1
}

# Test 2: Verify model exists
Write-Host "`nTest 2: Model Verification" -ForegroundColor Yellow
$results.tests.modelVerification = @{}
if (Test-Path $ModelPath) {
    $model = Get-Item $ModelPath
    Write-Host "  ✅ Model found: $($model.Name)" -ForegroundColor Green
    Write-Host "  Size: $([math]::Round($model.Length/1MB, 2)) MB" -ForegroundColor Gray
    $results.tests.modelVerification.status = "PASSED"
    $results.tests.modelVerification.model = $model.Name
    $results.tests.modelVerification.size_mb = [math]::Round($model.Length/1MB, 2)
} else {
    Write-Host "  ❌ Model not found at $ModelPath" -ForegroundColor Red
    $results.tests.modelVerification.status = "FAILED"
    exit 1
}

# Test 3: Check GPU availability
Write-Host "`nTest 3: GPU Detection" -ForegroundColor Yellow
$results.tests.gpuDetection = @{}
try {
    $nvidiaSmi = & nvidia-smi --query-gpu=name,driver_version,memory.total --format=csv,noheader 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✅ NVIDIA GPU detected" -ForegroundColor Green
        Write-Host "  $nvidiaSmi" -ForegroundColor Gray
        $results.tests.gpuDetection.status = "PASSED"
        $results.tests.gpuDetection.output = $nvidiaSmi
    } else {
        Write-Host "  ⚠️ nvidia-smi failed" -ForegroundColor Yellow
        $results.tests.gpuDetection.status = "WARNING"
    }
} catch {
    Write-Host "  ⚠️ Could not run nvidia-smi: $_" -ForegroundColor Yellow
    $results.tests.gpuDetection.status = "WARNING"
}

# Test 4: DLL Dependencies Check
Write-Host "`nTest 4: DLL Dependencies Validation" -ForegroundColor Yellow
$results.tests.dllDependencies = @{}
$requiredDlls = @(
    "cudart64_12.dll",
    "cublas64_12.dll",
    "cudnn64_9.dll"
)

$foundDlls = @()
$missingDlls = @()

foreach ($dll in $requiredDlls) {
    $dllPath = Get-ChildItem -Path $env:PATH.Split(';') -Filter $dll -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($dllPath) {
        Write-Host "  ✅ Found: $dll at $($dllPath.DirectoryName)" -ForegroundColor Green
        $foundDlls += $dll
    } else {
        Write-Host "  ⚠️ Not found in PATH: $dll" -ForegroundColor Yellow
        $missingDlls += $dll
    }
}

$results.tests.dllDependencies.status = if ($foundDlls.Count -eq $requiredDlls.Count) { "PASSED" } else { "PARTIAL" }
$results.tests.dllDependencies.found = $foundDlls
$results.tests.dllDependencies.missing = $missingDlls

# Test 5: Start Server
Write-Host "`nTest 5: CUDA Initialization & Server Startup" -ForegroundColor Yellow
Write-Host "  Starting server on port $Port..." -ForegroundColor Gray
$results.tests.serverStartup = @{}

$logFile = "T:\projects\rust-mistral\mistral.rs\server-test.log"
$startTime = Get-Date

$processArgs = @(
    "gguf",
    "-m", $ModelPath,
    "-t", "qwen2.5",
    "-p", $Port,
    "--serve-ip", "127.0.0.1"
)

try {
    # Start server process in background
    $process = Start-Process -FilePath $BinaryPath -ArgumentList $processArgs -PassThru -RedirectStandardOutput $logFile -RedirectStandardError "${logFile}.err" -NoNewWindow

    Write-Host "  Process started (PID: $($process.Id))" -ForegroundColor Gray
    $results.tests.serverStartup.pid = $process.Id

    # Wait for server to initialize
    Write-Host "  Waiting for server initialization..." -ForegroundColor Gray
    $initialized = $false
    $attempts = 0
    $maxAttempts = 60

    while (-not $initialized -and $attempts -lt $maxAttempts) {
        Start-Sleep -Seconds 2
        $attempts++

        # Check if process is still running
        if ($process.HasExited) {
            Write-Host "  ❌ Server process exited unexpectedly (Exit code: $($process.ExitCode))" -ForegroundColor Red
            $results.tests.serverStartup.status = "FAILED"
            $results.tests.serverStartup.error = "Process exited with code $($process.ExitCode)"

            # Show error log
            if (Test-Path "${logFile}.err") {
                $errorContent = Get-Content "${logFile}.err" -Raw
                Write-Host "`n  Error log:" -ForegroundColor Red
                Write-Host "  $errorContent" -ForegroundColor Red
            }
            break
        }

        # Try health check
        try {
            $response = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/health" -TimeoutSec 2 -ErrorAction SilentlyContinue
            if ($response.StatusCode -eq 200) {
                $initialized = $true
                Write-Host "  ✅ Server initialized successfully!" -ForegroundColor Green
                $loadTime = (Get-Date) - $startTime
                Write-Host "  Load time: $($loadTime.TotalSeconds) seconds" -ForegroundColor Gray
                $results.tests.serverStartup.status = "PASSED"
                $results.tests.serverStartup.load_time_seconds = [math]::Round($loadTime.TotalSeconds, 2)
            }
        } catch {
            # Still initializing, continue waiting
        }

        Write-Host "." -NoNewline -ForegroundColor Gray
    }

    Write-Host ""

    if (-not $initialized -and -not $process.HasExited) {
        Write-Host "  ⚠️ Server timeout after $($maxAttempts * 2) seconds" -ForegroundColor Yellow
        $results.tests.serverStartup.status = "TIMEOUT"
    }

    # Test 6: API Endpoint Tests (only if server initialized)
    if ($initialized) {
        Write-Host "`nTest 6: API Endpoint Testing" -ForegroundColor Yellow
        $results.tests.apiEndpoints = @{}

        # Test 6a: Health endpoint
        try {
            $healthResponse = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/health" -Method Get
            Write-Host "  ✅ GET /health successful" -ForegroundColor Green
            Write-Host "  Response: $($healthResponse | ConvertTo-Json -Compress)" -ForegroundColor Gray
            $results.tests.apiEndpoints.health = @{
                status = "PASSED"
                response = $healthResponse
            }
        } catch {
            Write-Host "  ❌ GET /health failed: $_" -ForegroundColor Red
            $results.tests.apiEndpoints.health = @{ status = "FAILED"; error = $_.Exception.Message }
        }

        # Test 6b: Models endpoint
        try {
            $modelsResponse = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/v1/models" -Method Get
            Write-Host "  ✅ GET /v1/models successful" -ForegroundColor Green
            Write-Host "  Models: $($modelsResponse.data.Count)" -ForegroundColor Gray
            $results.tests.apiEndpoints.models = @{
                status = "PASSED"
                model_count = $modelsResponse.data.Count
            }
        } catch {
            Write-Host "  ⚠️ GET /v1/models failed: $_" -ForegroundColor Yellow
            $results.tests.apiEndpoints.models = @{ status = "FAILED"; error = $_.Exception.Message }
        }

        # Test 6c: Chat completions endpoint
        Write-Host "  Testing inference..." -ForegroundColor Gray
        try {
            $chatBody = @{
                model = "qwen2.5"
                messages = @(
                    @{
                        role = "user"
                        content = "What is 2+2? Answer in one word."
                    }
                )
                max_tokens = 10
                temperature = 0.1
            } | ConvertTo-Json

            $inferenceStart = Get-Date
            $chatResponse = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/v1/chat/completions" -Method Post -Body $chatBody -ContentType "application/json" -TimeoutSec 30
            $inferenceTime = (Get-Date) - $inferenceStart

            Write-Host "  ✅ POST /v1/chat/completions successful" -ForegroundColor Green
            Write-Host "  Response: $($chatResponse.choices[0].message.content)" -ForegroundColor Gray
            Write-Host "  Inference time: $($inferenceTime.TotalSeconds) seconds" -ForegroundColor Gray
            Write-Host "  Tokens: $($chatResponse.usage.total_tokens)" -ForegroundColor Gray

            $results.tests.apiEndpoints.chat_completions = @{
                status = "PASSED"
                inference_time_seconds = [math]::Round($inferenceTime.TotalSeconds, 2)
                tokens = $chatResponse.usage.total_tokens
                response = $chatResponse.choices[0].message.content
            }
        } catch {
            Write-Host "  ❌ POST /v1/chat/completions failed: $_" -ForegroundColor Red
            $results.tests.apiEndpoints.chat_completions = @{ status = "FAILED"; error = $_.Exception.Message }
        }

        # Test 6d: Check VRAM usage
        Write-Host "`nTest 7: VRAM Usage Monitoring" -ForegroundColor Yellow
        $results.tests.vramUsage = @{}
        try {
            $vramInfo = & nvidia-smi --query-gpu=memory.used,memory.total --format=csv,noheader,nounits 2>&1
            if ($LASTEXITCODE -eq 0) {
                $vramParts = $vramInfo -split ','
                $used = [int]$vramParts[0].Trim()
                $total = [int]$vramParts[1].Trim()
                $percent = [math]::Round(($used / $total) * 100, 1)

                Write-Host "  ✅ VRAM Usage: $used MB / $total MB ($percent%)" -ForegroundColor Green
                $results.tests.vramUsage.status = "PASSED"
                $results.tests.vramUsage.used_mb = $used
                $results.tests.vramUsage.total_mb = $total
                $results.tests.vramUsage.percent = $percent
            }
        } catch {
            Write-Host "  ⚠️ Could not query VRAM usage" -ForegroundColor Yellow
            $results.tests.vramUsage.status = "WARNING"
        }
    }

    # Cleanup: Stop server
    Write-Host "`nCleaning up..." -ForegroundColor Yellow
    if (-not $process.HasExited) {
        Write-Host "  Stopping server (PID: $($process.Id))..." -ForegroundColor Gray
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 2
    }

    # Check if logs exist
    if (Test-Path $logFile) {
        $logSize = (Get-Item $logFile).Length
        Write-Host "  Log file created: $logFile ($logSize bytes)" -ForegroundColor Gray

        # Check for CUDA initialization in logs
        $logContent = Get-Content $logFile -Raw
        if ($logContent -match "CUDA") {
            Write-Host "  ✅ CUDA mentioned in logs" -ForegroundColor Green
            $results.tests.cudaInitialization = @{
                status = "DETECTED"
                log_file = $logFile
            }
        }
    }

} catch {
    Write-Host "  ❌ Server startup failed: $_" -ForegroundColor Red
    $results.tests.serverStartup.status = "FAILED"
    $results.tests.serverStartup.error = $_.Exception.Message
}

# Calculate overall success rate
$totalTests = $results.tests.Count
$passedTests = ($results.tests.Values | Where-Object { $_.status -eq "PASSED" }).Count
$successRate = [math]::Round(($passedTests / $totalTests) * 100, 0)

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Test Results Summary" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Total Tests: $totalTests" -ForegroundColor White
Write-Host "Passed: $passedTests" -ForegroundColor Green
Write-Host "Success Rate: $successRate%" -ForegroundColor $(if ($successRate -ge 75) { "Green" } else { "Yellow" })
Write-Host ""

# Save results to JSON
$results.summary = @{
    total_tests = $totalTests
    passed_tests = $passedTests
    success_rate = $successRate
}

$results | ConvertTo-Json -Depth 10 | Out-File $ResultsFile -Encoding UTF8
Write-Host "Results saved to: $ResultsFile" -ForegroundColor Cyan
Write-Host ""

if ($successRate -ge 75) {
    Write-Host "✅ Phase 1 COMPLETE - Ready for Phase 2!" -ForegroundColor Green
    exit 0
} else {
    Write-Host "⚠️ Phase 1 PARTIAL - Review results before Phase 2" -ForegroundColor Yellow
    exit 1
}
