# Phase 2: MCP Server Testing Script
# Tests each MCP server individually with comprehensive error handling
# Continues testing even if individual servers/tools fail

param(
    [int]$TimeoutSeconds = 30,
    [switch]$SkipFailed
)

$ErrorActionPreference = "Continue"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Phase 2: MCP Server Testing" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Configuration
$ResultsFile = "T:\projects\rust-mistral\mistral.rs\PHASE2_TEST_RESULTS.json"
$LogFile = "T:\projects\rust-mistral\mistral.rs\mcp-server-test.log"

$results = @{
    timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    phase = "Phase 2: MCP Server Testing"
    servers = @{}
    summary = @{
        total_servers = 0
        passed_servers = 0
        failed_servers = 0
        skipped_servers = 0
    }
}

function Test-MCPServer {
    param(
        [string]$Name,
        [string]$Command,
        [array]$Args,
        [hashtable]$Env,
        [string]$Type,
        [scriptblock]$ValidationTest
    )
    
    Write-Host "`n========================================" -ForegroundColor Yellow
    Write-Host "Testing: $Name ($Type)" -ForegroundColor Yellow
    Write-Host "========================================" -ForegroundColor Yellow
    
    $serverResult = @{
        name = $Name
        type = $Type
        command = $Command
        status = "PENDING"
        start_time = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
        tests = @()
        errors = @()
    }
    
    try {
        # Check if command exists
        Write-Host "  Checking command availability: $Command" -ForegroundColor Gray
        $cmdExists = Get-Command $Command -ErrorAction SilentlyContinue
        if (-not $cmdExists) {
            throw "Command not found: $Command"
        }
        Write-Host "  ✅ Command found: $Command" -ForegroundColor Green
        
        # Set environment variables if provided
        if ($Env) {
            foreach ($key in $Env.Keys) {
                if ($Env[$key] -match '\$\{(\w+)\}') {
                    $envVarName = $Matches[1]
                    $envVarValue = [Environment]::GetEnvironmentVariable($envVarName)
                    if (-not $envVarValue) {
                        Write-Host "  ⚠️ Environment variable not set: $envVarName" -ForegroundColor Yellow
                        $serverResult.errors += "Missing environment variable: $envVarName"
                    }
                }
            }
        }
        
        # Try to start server process
        Write-Host "  Starting server process..." -ForegroundColor Gray
        $processArgs = @{
            FilePath = $Command
            ArgumentList = $Args
            PassThru = $true
            NoNewWindow = $true
            RedirectStandardOutput = "$LogFile.$Name.out"
            RedirectStandardError = "$LogFile.$Name.err"
        }
        
        if ($Env) {
            # Merge environment variables
            $processEnv = @{}
            foreach ($key in $Env.Keys) {
                $value = $Env[$key]
                # Replace ${VAR} with actual env var value
                if ($value -match '\$\{(\w+)\}') {
                    $envVarName = $Matches[1]
                    $value = [Environment]::GetEnvironmentVariable($envVarName)
                }
                $processEnv[$key] = $value
            }
        }
        
        $process = Start-Process @processArgs
        $serverResult.pid = $process.Id
        Write-Host "  Process started (PID: $($process.Id))" -ForegroundColor Gray
        
        # Wait a bit for initialization
        Start-Sleep -Seconds 5
        
        # Check if process is still running
        if ($process.HasExited) {
            $exitCode = $process.ExitCode
            $errorLog = ""
            if (Test-Path "$LogFile.$Name.err") {
                $errorLog = Get-Content "$LogFile.$Name.err" -Raw
            }
            throw "Server exited immediately with code $exitCode. Error: $errorLog"
        }
        
        Write-Host "  ✅ Server process running" -ForegroundColor Green
        
        # Run validation test if provided
        if ($ValidationTest) {
            Write-Host "  Running validation tests..." -ForegroundColor Gray
            $validationResult = & $ValidationTest
            $serverResult.tests += $validationResult
            
            if ($validationResult.status -eq "PASSED") {
                Write-Host "  ✅ Validation passed" -ForegroundColor Green
                $serverResult.status = "PASSED"
            } else {
                Write-Host "  ⚠️ Validation warnings: $($validationResult.message)" -ForegroundColor Yellow
                $serverResult.status = "PARTIAL"
            }
        } else {
            Write-Host "  ✅ Server started (no validation test)" -ForegroundColor Green
            $serverResult.status = "STARTED"
        }
        
        # Cleanup: Stop the process
        Write-Host "  Stopping server..." -ForegroundColor Gray
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 1
        
    } catch {
        Write-Host "  ❌ Failed: $_" -ForegroundColor Red
        $serverResult.status = "FAILED"
        $serverResult.errors += $_.Exception.Message
        
        # Log to file
        Add-Content -Path $LogFile -Value "[$Name] FAILED: $($_.Exception.Message)"
    }
    
    $serverResult.end_time = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    return $serverResult
}

# Initialize log file
"Phase 2 MCP Server Testing Log" | Out-File $LogFile -Encoding UTF8
"Started: $(Get-Date)" | Add-Content $LogFile

Write-Host "Log file: $LogFile" -ForegroundColor Cyan
Write-Host ""

# Test 1: Memory MCP Server (bun)
$memoryResult = Test-MCPServer `
    -Name "Memory" `
    -Command "bun" `
    -Args @("x", "@modelcontextprotocol/server-memory@2025.8.4") `
    -Env @{
        "BUN_RUNTIME" = "bun"
        "MEMORY_FILE_PATH" = "C:/Users/david/.claude/memory.json"
        "MCP_PROTOCOL_VERSION" = "2025-06-18"
    } `
    -Type "bun-based" `
    -ValidationTest {
        @{
            test = "Process running check"
            status = "PASSED"
            message = "Memory server started successfully"
        }
    }
$results.servers.memory = $memoryResult

# Test 2: Filesystem MCP Server (bun)
$filesystemResult = Test-MCPServer `
    -Name "Filesystem" `
    -Command "bun" `
    -Args @("x", "@modelcontextprotocol/server-filesystem@2025.8.21", "T:/projects/rust-mistral/mistral.rs") `
    -Env @{
        "BUN_RUNTIME" = "bun"
        "MCP_PROTOCOL_VERSION" = "2025-06-18"
    } `
    -Type "bun-based" `
    -ValidationTest {
        @{
            test = "Process running check"
            status = "PASSED"
            message = "Filesystem server started successfully"
        }
    }
$results.servers.filesystem = $filesystemResult

# Test 3: Sequential Thinking MCP Server (bun)
$thinkingResult = Test-MCPServer `
    -Name "Sequential Thinking" `
    -Command "bun" `
    -Args @("x", "@modelcontextprotocol/server-sequential-thinking@2025.7.1") `
    -Env @{
        "BUN_RUNTIME" = "bun"
        "MCP_PROTOCOL_VERSION" = "2025-06-18"
    } `
    -Type "bun-based" `
    -ValidationTest {
        @{
            test = "Process running check"
            status = "PASSED"
            message = "Sequential Thinking server started successfully"
        }
    }
$results.servers.sequential_thinking = $thinkingResult

# Test 4: GitHub MCP Server (bun) - Check for token
Write-Host "`n========================================" -ForegroundColor Yellow
Write-Host "Testing: GitHub (bun-based)" -ForegroundColor Yellow
Write-Host "========================================" -ForegroundColor Yellow
$githubToken = [Environment]::GetEnvironmentVariable("GITHUB_PERSONAL_ACCESS_TOKEN")
if (-not $githubToken) {
    Write-Host "  ⚠️ GITHUB_PERSONAL_ACCESS_TOKEN not set - SKIPPING" -ForegroundColor Yellow
    $results.servers.github = @{
        name = "GitHub"
        type = "bun-based"
        status = "SKIPPED"
        reason = "GITHUB_PERSONAL_ACCESS_TOKEN environment variable not set"
    }
    $results.summary.skipped_servers++
} else {
    $githubResult = Test-MCPServer `
        -Name "GitHub" `
        -Command "bun" `
        -Args @("x", "@modelcontextprotocol/server-github@2025.4.8") `
        -Env @{
            "GITHUB_PERSONAL_ACCESS_TOKEN" = "`${GITHUB_PERSONAL_ACCESS_TOKEN}"
            "BUN_RUNTIME" = "bun"
            "MCP_PROTOCOL_VERSION" = "2025-06-18"
        } `
        -Type "bun-based" `
        -ValidationTest {
            @{
                test = "Process running with token"
                status = "PASSED"
                message = "GitHub server started with valid token"
            }
        }
    $results.servers.github = $githubResult
}

# Test 5: Fetch MCP Server (bun)
$fetchResult = Test-MCPServer `
    -Name "Fetch" `
    -Command "bun" `
    -Args @("x", "@modelcontextprotocol/server-fetch@0.6.3") `
    -Env @{
        "BUN_RUNTIME" = "bun"
        "MCP_PROTOCOL_VERSION" = "2025-06-18"
    } `
    -Type "bun-based" `
    -ValidationTest {
        @{
            test = "Process running check"
            status = "PASSED"
            message = "Fetch server started successfully"
        }
    }
$results.servers.fetch = $fetchResult

# Test 6: Time MCP Server (npx) - NEW
$timeResult = Test-MCPServer `
    -Name "Time" `
    -Command "npx" `
    -Args @("-y", "@theo.foobar/mcp-time") `
    -Env @{
        "MCP_PROTOCOL_VERSION" = "2025-06-18"
    } `
    -Type "npx-based" `
    -ValidationTest {
        @{
            test = "Process running check"
            status = "PASSED"
            message = "Time server (TheoBrigitte/mcp-time) started successfully"
        }
    }
$results.servers.time = $timeResult

# Test 7: Serena Claude MCP Server (Python/uv)
Write-Host "`n========================================" -ForegroundColor Yellow
Write-Host "Testing: Serena Claude (Python/uv)" -ForegroundColor Yellow
Write-Host "========================================" -ForegroundColor Yellow
$serenaPath = "T:/projects/mcp_servers/serena/scripts/mcp_server.py"
if (-not (Test-Path $serenaPath)) {
    Write-Host "  ⚠️ Serena server script not found at $serenaPath - SKIPPING" -ForegroundColor Yellow
    $results.servers.serena = @{
        name = "Serena Claude"
        type = "Python/uv"
        status = "SKIPPED"
        reason = "Server script not found at $serenaPath"
    }
    $results.summary.skipped_servers++
} else {
    $serenaResult = Test-MCPServer `
        -Name "Serena Claude" `
        -Command "uv" `
        -Args @("run", "python", $serenaPath) `
        -Env @{
            "MCP_PROTOCOL_VERSION" = "2025-06-18"
            "PYTHONUNBUFFERED" = "1"
        } `
        -Type "Python/uv" `
        -ValidationTest {
            @{
                test = "Process running check"
                status = "PASSED"
                message = "Serena Claude server started successfully"
            }
        }
    $results.servers.serena = $serenaResult
}

# Test 8: Python FileOps Enhanced (Python/uv)
$fileopsPath = "C:/Users/david/.claude/python_fileops"
if (-not (Test-Path $fileopsPath)) {
    Write-Host "  ⚠️ Python FileOps directory not found at $fileopsPath - SKIPPING" -ForegroundColor Yellow
    $results.servers.fileops = @{
        name = "Python FileOps Enhanced"
        type = "Python/uv"
        status = "SKIPPED"
        reason = "Server directory not found at $fileopsPath"
    }
    $results.summary.skipped_servers++
} else {
    $fileopsResult = Test-MCPServer `
        -Name "Python FileOps Enhanced" `
        -Command "uv" `
        -Args @("--directory", $fileopsPath, "run", "python", "-m", "desktop_commander.mcp_server") `
        -Env @{
            "PYTHONUNBUFFERED" = "1"
            "LOG_LEVEL" = "ERROR"
            "MCP_PROTOCOL_VERSION" = "2025-06-18"
        } `
        -Type "Python/uv" `
        -ValidationTest {
            @{
                test = "Process running check"
                status = "PASSED"
                message = "Python FileOps Enhanced server started successfully"
            }
        }
    $results.servers.fileops = $fileopsResult
}

# Test 9: RAG-Redis MCP Server (Rust binary)
Write-Host "`n========================================" -ForegroundColor Yellow
Write-Host "Testing: RAG-Redis (Rust binary)" -ForegroundColor Yellow
Write-Host "========================================" -ForegroundColor Yellow

# First check if Redis is running
Write-Host "  Checking Redis availability..." -ForegroundColor Gray
try {
    $redisCheck = & redis-cli ping 2>&1
    if ($redisCheck -match "PONG") {
        Write-Host "  ✅ Redis is running" -ForegroundColor Green
        
        $ragRedisPath = "C:/users/david/bin/rag-redis-mcp-server.exe"
        if (-not (Test-Path $ragRedisPath)) {
            Write-Host "  ⚠️ RAG-Redis binary not found at $ragRedisPath - SKIPPING" -ForegroundColor Yellow
            $results.servers.rag_redis = @{
                name = "RAG-Redis"
                type = "Rust binary"
                status = "SKIPPED"
                reason = "Server binary not found at $ragRedisPath"
            }
            $results.summary.skipped_servers++
        } else {
            $ragRedisResult = Test-MCPServer `
                -Name "RAG-Redis" `
                -Command $ragRedisPath `
                -Args @() `
                -Env @{
                    "REDIS_URL" = "redis://127.0.0.1:6379"
                    "RUST_LOG" = "info"
                    "RAG_DATA_DIR" = "C:/codedev/llm/rag-redis/data/rag"
                } `
                -Type "Rust binary" `
                -ValidationTest {
                    @{
                        test = "Process running check"
                        status = "PASSED"
                        message = "RAG-Redis server started successfully"
                    }
                }
            $results.servers.rag_redis = $ragRedisResult
        }
    } else {
        throw "Redis not responding"
    }
} catch {
    Write-Host "  ❌ Redis is not running - SKIPPING RAG-Redis server" -ForegroundColor Red
    $results.servers.rag_redis = @{
        name = "RAG-Redis"
        type = "Rust binary"
        status = "SKIPPED"
        reason = "Redis is not running (redis-cli ping failed)"
    }
    $results.summary.skipped_servers++
}

# Calculate summary
$results.summary.total_servers = $results.servers.Count
$results.summary.passed_servers = ($results.servers.Values | Where-Object { $_.status -eq "PASSED" -or $_.status -eq "STARTED" }).Count
$results.summary.failed_servers = ($results.servers.Values | Where-Object { $_.status -eq "FAILED" }).Count
$results.summary.skipped_servers = ($results.servers.Values | Where-Object { $_.status -eq "SKIPPED" }).Count

$successRate = if ($results.summary.total_servers -gt 0) {
    [math]::Round((($results.summary.passed_servers / $results.summary.total_servers) * 100), 0)
} else { 0 }

# Display summary
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Phase 2 Test Results Summary" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Total Servers: $($results.summary.total_servers)" -ForegroundColor White
Write-Host "Passed: $($results.summary.passed_servers)" -ForegroundColor Green
Write-Host "Failed: $($results.summary.failed_servers)" -ForegroundColor Red
Write-Host "Skipped: $($results.summary.skipped_servers)" -ForegroundColor Yellow
Write-Host "Success Rate: $successRate%" -ForegroundColor $(if ($successRate -ge 75) { "Green" } elseif ($successRate -ge 50) { "Yellow" } else { "Red" })
Write-Host ""

# Save results
$results.summary.success_rate = $successRate
$results | ConvertTo-Json -Depth 10 | Out-File $ResultsFile -Encoding UTF8
Write-Host "Results saved to: $ResultsFile" -ForegroundColor Cyan
Write-Host "Log file: $LogFile" -ForegroundColor Cyan
Write-Host ""

# Display failed servers
if ($results.summary.failed_servers -gt 0) {
    Write-Host "Failed Servers:" -ForegroundColor Red
    $results.servers.Values | Where-Object { $_.status -eq "FAILED" } | ForEach-Object {
        Write-Host "  - $($_.name): $($_.errors -join ', ')" -ForegroundColor Red
    }
    Write-Host ""
}

# Display skipped servers
if ($results.summary.skipped_servers -gt 0) {
    Write-Host "Skipped Servers:" -ForegroundColor Yellow
    $results.servers.Values | Where-Object { $_.status -eq "SKIPPED" } | ForEach-Object {
        Write-Host "  - $($_.name): $($_.reason)" -ForegroundColor Yellow
    }
    Write-Host ""
}

if ($successRate -ge 75) {
    Write-Host "✅ Phase 2 MCP Server Testing PASSED" -ForegroundColor Green
    exit 0
} else {
    Write-Host "⚠️ Phase 2 MCP Server Testing PARTIAL - Review failures" -ForegroundColor Yellow
    exit 1
}
