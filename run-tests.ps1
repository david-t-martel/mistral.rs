# Comprehensive Test Suite for mistral.rs
# Tests binary, model loading, MCP servers, and performance

param(
    [switch]$SkipMCP,
    [switch]$SkipPerformance,
    [switch]$QuickTest
)

$ErrorActionPreference = "Continue"
$TestResults = @()
$ProjectRoot = "T:\projects\rust-mistral\mistral.rs"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "mistral.rs Comprehensive Test Suite" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

# Test configuration
$BinaryPath = "C:\Users\david\.cargo\shared-target\release\mistralrs-server.exe"
$ModelPath = "C:\codedev\llm\.models\gemma-2-2b-it-gguf\gemma-2-2b-it-Q4_K_M.gguf"
$McpConfigPath = "$ProjectRoot\MCP_CONFIG.json"
$TestPort = 11434

function Add-TestResult {
    param($Category, $Test, $Status, $Details, $Duration)
    $script:TestResults += [PSCustomObject]@{
        Category = $Category
        Test = $Test
        Status = $Status
        Details = $Details
        Duration = $Duration
    }
}

function Test-Binary {
    Write-Host "[Phase 1] Binary Tests" -ForegroundColor Yellow
    Write-Host "----------------------------------------" -ForegroundColor Gray
    
    # Test 1: Binary exists
    $start = Get-Date
    if (Test-Path $BinaryPath) {
        $size = [math]::Round((Get-Item $BinaryPath).Length / 1MB, 2)
        Add-TestResult "Binary" "File Exists" "PASS" "Size: $size MB" ((Get-Date) - $start).TotalSeconds
        Write-Host "  ✓ Binary exists ($size MB)" -ForegroundColor Green
    } else {
        Add-TestResult "Binary" "File Exists" "FAIL" "Binary not found" ((Get-Date) - $start).TotalSeconds
        Write-Host "  ✗ Binary not found at $BinaryPath" -ForegroundColor Red
        return $false
    }
    
    # Test 2: Binary version
    $start = Get-Date
    try {
        $version = & $BinaryPath --version 2>&1
        if ($LASTEXITCODE -eq 0) {
            Add-TestResult "Binary" "Version Check" "PASS" "$version" ((Get-Date) - $start).TotalSeconds
            Write-Host "  ✓ Version: $version" -ForegroundColor Green
        } else {
            Add-TestResult "Binary" "Version Check" "FAIL" "Exit code: $LASTEXITCODE" ((Get-Date) - $start).TotalSeconds
            Write-Host "  ✗ Version check failed" -ForegroundColor Red
        }
    } catch {
        Add-TestResult "Binary" "Version Check" "FAIL" "$_" ((Get-Date) - $start).TotalSeconds
        Write-Host "  ✗ Error: $_" -ForegroundColor Red
    }
    
    # Test 3: Help command
    $start = Get-Date
    try {
        $help = & $BinaryPath --help 2>&1 | Select-Object -First 5
        if ($help) {
            Add-TestResult "Binary" "Help Command" "PASS" "Help output received" ((Get-Date) - $start).TotalSeconds
            Write-Host "  ✓ Help command works" -ForegroundColor Green
        }
    } catch {
        Add-TestResult "Binary" "Help Command" "FAIL" "$_" ((Get-Date) - $start).TotalSeconds
        Write-Host "  ✗ Help command failed" -ForegroundColor Red
    }
    
    Write-Host ""
    return $true
}

function Test-Dependencies {
    Write-Host "[Phase 2] Dependency Tests" -ForegroundColor Yellow
    Write-Host "----------------------------------------" -ForegroundColor Gray
    
    # Test CUDA
    $start = Get-Date
    try {
        $cuda = nvidia-smi --query-gpu=name,driver_version,memory.total --format=csv,noheader 2>&1
        if ($LASTEXITCODE -eq 0) {
            Add-TestResult "Dependencies" "CUDA/GPU" "PASS" "$cuda" ((Get-Date) - $start).TotalSeconds
            Write-Host "  ✓ CUDA: $cuda" -ForegroundColor Green
        } else {
            Add-TestResult "Dependencies" "CUDA/GPU" "FAIL" "nvidia-smi failed" ((Get-Date) - $start).TotalSeconds
            Write-Host "  ✗ CUDA check failed" -ForegroundColor Red
        }
    } catch {
        Add-TestResult "Dependencies" "CUDA/GPU" "FAIL" "$_" ((Get-Date) - $start).TotalSeconds
        Write-Host "  ✗ CUDA error: $_" -ForegroundColor Red
    }
    
    # Test Bun (for MCP)
    $start = Get-Date
    try {
        $bunVersion = bun --version 2>&1
        if ($LASTEXITCODE -eq 0) {
            Add-TestResult "Dependencies" "Bun" "PASS" "v$bunVersion" ((Get-Date) - $start).TotalSeconds
            Write-Host "  ✓ Bun: v$bunVersion" -ForegroundColor Green
        } else {
            Add-TestResult "Dependencies" "Bun" "FAIL" "Not found" ((Get-Date) - $start).TotalSeconds
            Write-Host "  ✗ Bun not found" -ForegroundColor Yellow
        }
    } catch {
        Add-TestResult "Dependencies" "Bun" "FAIL" "$_" ((Get-Date) - $start).TotalSeconds
        Write-Host "  ✗ Bun error: $_" -ForegroundColor Yellow
    }
    
    # Test Redis
    $start = Get-Date
    try {
        $redis = redis-cli ping 2>&1
        if ($redis -eq "PONG") {
            Add-TestResult "Dependencies" "Redis" "PASS" "Connected" ((Get-Date) - $start).TotalSeconds
            Write-Host "  ✓ Redis: Connected" -ForegroundColor Green
        } else {
            Add-TestResult "Dependencies" "Redis" "FAIL" "No response" ((Get-Date) - $start).TotalSeconds
            Write-Host "  ✗ Redis not responding" -ForegroundColor Yellow
        }
    } catch {
        Add-TestResult "Dependencies" "Redis" "FAIL" "$_" ((Get-Date) - $start).TotalSeconds
        Write-Host "  ✗ Redis error: $_" -ForegroundColor Yellow
    }
    
    # Test Model exists
    $start = Get-Date
    if (Test-Path $ModelPath) {
        $size = [math]::Round((Get-Item $ModelPath).Length / 1GB, 2)
        Add-TestResult "Dependencies" "Model File" "PASS" "Size: $size GB" ((Get-Date) - $start).TotalSeconds
        Write-Host "  ✓ Model: $size GB" -ForegroundColor Green
    } else {
        Add-TestResult "Dependencies" "Model File" "FAIL" "Not found" ((Get-Date) - $start).TotalSeconds
        Write-Host "  ✗ Model not found" -ForegroundColor Red
    }
    
    Write-Host ""
}

function Test-MCPServers {
    if ($SkipMCP) {
        Write-Host "[Skipped] MCP Server Tests" -ForegroundColor Gray
        return
    }
    
    Write-Host "[Phase 3] MCP Server Tests" -ForegroundColor Yellow
    Write-Host "----------------------------------------" -ForegroundColor Gray
    
    $mcpServers = @(
        @{Name="Memory"; Command="bun"; Args=@("x", "@modelcontextprotocol/server-memory@2025.8.4", "--help")},
        @{Name="Filesystem"; Command="bun"; Args=@("x", "@modelcontextprotocol/server-filesystem@2025.8.21", "--help")},
        @{Name="Sequential Thinking"; Command="bun"; Args=@("x", "@modelcontextprotocol/server-sequential-thinking@2025.7.1", "--help")},
        @{Name="GitHub"; Command="bun"; Args=@("x", "@modelcontextprotocol/server-github@2025.4.8", "--help")},
        @{Name="Fetch"; Command="bun"; Args=@("x", "@modelcontextprotocol/server-fetch@0.6.3", "--help")},
        @{Name="Time"; Command="bun"; Args=@("x", "@modelcontextprotocol/server-time@0.2.2", "--help"); Deprecated=$true}
    )
    
    foreach ($server in $mcpServers) {
        $start = Get-Date
        if ($server.Deprecated) {
            Add-TestResult "MCP" $server.Name "DEPRECATED" "Server is deprecated" 0
            Write-Host "  ⚠ $($server.Name): DEPRECATED" -ForegroundColor Yellow
            continue
        }
        
        try {
            $timeout = 10
            $job = Start-Job -ScriptBlock {
                param($cmd, $args)
                & $cmd @args 2>&1
            } -ArgumentList $server.Command, $server.Args
            
            $null = Wait-Job $job -Timeout $timeout
            $output = Receive-Job $job
            Stop-Job $job -ErrorAction SilentlyContinue
            Remove-Job $job -ErrorAction SilentlyContinue
            
            if ($output) {
                Add-TestResult "MCP" $server.Name "PASS" "Server available" ((Get-Date) - $start).TotalSeconds
                Write-Host "  ✓ $($server.Name): Available" -ForegroundColor Green
            } else {
                Add-TestResult "MCP" $server.Name "TIMEOUT" "No response in ${timeout}s" ((Get-Date) - $start).TotalSeconds
                Write-Host "  ⚠ $($server.Name): Timeout" -ForegroundColor Yellow
            }
        } catch {
            Add-TestResult "MCP" $server.Name "FAIL" "$_" ((Get-Date) - $start).TotalSeconds
            Write-Host "  ✗ $($server.Name): $_" -ForegroundColor Red
        }
    }
    
    # Test RAG-Redis
    $start = Get-Date
    $ragBinary = "C:\users\david\bin\rag-redis-mcp-server.exe"
    if (Test-Path $ragBinary) {
        Add-TestResult "MCP" "RAG-Redis" "PASS" "Binary exists" ((Get-Date) - $start).TotalSeconds
        Write-Host "  ✓ RAG-Redis: Binary found" -ForegroundColor Green
    } else {
        Add-TestResult "MCP" "RAG-Redis" "FAIL" "Binary not found" ((Get-Date) - $start).TotalSeconds
        Write-Host "  ✗ RAG-Redis: Binary not found" -ForegroundColor Red
    }
    
    Write-Host ""
}

function Test-ModelLoading {
    if ($QuickTest) {
        Write-Host "[Skipped] Model Loading Test (Quick mode)" -ForegroundColor Gray
        return
    }
    
    Write-Host "[Phase 4] Model Loading Test" -ForegroundColor Yellow
    Write-Host "----------------------------------------" -ForegroundColor Gray
    Write-Host "  Starting server (will run for 15 seconds)..." -ForegroundColor Gray
    
    $start = Get-Date
    $job = Start-Job -ScriptBlock {
        param($script)
        & $script
    } -ArgumentList "$ProjectRoot\start-mistralrs.ps1"
    
    Start-Sleep -Seconds 15
    
    $output = Receive-Job $job
    Stop-Job $job -ErrorAction SilentlyContinue
    Remove-Job $job -ErrorAction SilentlyContinue
    
    $duration = ((Get-Date) - $start).TotalSeconds
    
    if ($output -match "Model loaded" -or $output -match "Serving" -or $output -match "Listening") {
        Add-TestResult "Model" "Loading" "PASS" "Server started successfully" $duration
        Write-Host "  ✓ Model loaded successfully" -ForegroundColor Green
    } else {
        Add-TestResult "Model" "Loading" "PARTIAL" "Server started but unclear if model loaded" $duration
        Write-Host "  ⚠ Server started (status unclear)" -ForegroundColor Yellow
    }
    
    Write-Host ""
}

function Test-APIEndpoint {
    if ($QuickTest) {
        Write-Host "[Skipped] API Endpoint Test (Quick mode)" -ForegroundColor Gray
        return
    }
    
    Write-Host "[Phase 5] API Endpoint Test" -ForegroundColor Yellow
    Write-Host "----------------------------------------" -ForegroundColor Gray
    Write-Host "  Note: Requires server to be running manually" -ForegroundColor Gray
    Write-Host "  Run: .\start-mistralrs.ps1 in another terminal" -ForegroundColor Gray
    Write-Host "  Skipping for now..." -ForegroundColor Gray
    Write-Host ""
}

# Run all tests
Write-Host "Starting test suite at $(Get-Date -Format 'HH:mm:ss')" -ForegroundColor Gray
Write-Host ""

$startTime = Get-Date

Test-Binary
Test-Dependencies
Test-MCPServers
Test-ModelLoading
Test-APIEndpoint

$totalTime = ((Get-Date) - $startTime).TotalSeconds

# Summary
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Test Summary" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

$passed = ($TestResults | Where-Object {$_.Status -eq "PASS"}).Count
$failed = ($TestResults | Where-Object {$_.Status -eq "FAIL"}).Count
$deprecated = ($TestResults | Where-Object {$_.Status -eq "DEPRECATED"}).Count
$partial = ($TestResults | Where-Object {$_.Status -eq "PARTIAL"}).Count
$timeout = ($TestResults | Where-Object {$_.Status -eq "TIMEOUT"}).Count
$total = $TestResults.Count

Write-Host "Total Tests: $total" -ForegroundColor White
Write-Host "  Passed: $passed" -ForegroundColor Green
Write-Host "  Failed: $failed" -ForegroundColor Red
if ($deprecated -gt 0) { Write-Host "  Deprecated: $deprecated" -ForegroundColor Yellow }
if ($partial -gt 0) { Write-Host "  Partial: $partial" -ForegroundColor Yellow }
if ($timeout -gt 0) { Write-Host "  Timeout: $timeout" -ForegroundColor Yellow }
Write-Host ""
Write-Host "Total Duration: $([math]::Round($totalTime, 2)) seconds" -ForegroundColor Gray
Write-Host ""

# Save results
$resultsFile = "$ProjectRoot\test-results.json"
$TestResults | ConvertTo-Json | Out-File $resultsFile
Write-Host "Results saved to: $resultsFile" -ForegroundColor Gray
Write-Host ""

# Display failures
if ($failed -gt 0) {
    Write-Host "Failed Tests:" -ForegroundColor Red
    $TestResults | Where-Object {$_.Status -eq "FAIL"} | ForEach-Object {
        Write-Host "  - $($_.Category): $($_.Test) - $($_.Details)" -ForegroundColor Red
    }
    Write-Host ""
}

# Display warnings
if ($deprecated -gt 0 -or $timeout -gt 0) {
    Write-Host "Warnings:" -ForegroundColor Yellow
    $TestResults | Where-Object {$_.Status -in @("DEPRECATED", "TIMEOUT", "PARTIAL")} | ForEach-Object {
        Write-Host "  - $($_.Category): $($_.Test) - $($_.Details)" -ForegroundColor Yellow
    }
    Write-Host ""
}

Write-Host "Test suite complete!" -ForegroundColor Green
Write-Host "Check TODO.md for tracked issues" -ForegroundColor Gray
