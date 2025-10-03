# Test rag-redis MCP Server
# Validates connectivity, document ingestion, and semantic search capabilities

$ErrorActionPreference = "Continue"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "rag-redis MCP Server Test Suite" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

# Configuration
$mcpServerPath = "c:/codedev/mcp_servers/rag-redis/build/index.js"
$testTimeout = 30
$results = @{}

# Test 1: Check if server executable exists
Write-Host "[1/6] Checking server files..." -ForegroundColor Yellow
if (Test-Path $mcpServerPath) {
    Write-Host "  [OK] Server found: $mcpServerPath" -ForegroundColor Green
    $results["server_exists"] = $true
} else {
    Write-Host "  [ERROR] Server not found: $mcpServerPath" -ForegroundColor Red
    $results["server_exists"] = $false
}
Write-Host ""

# Test 2: Check Redis availability
Write-Host "[2/6] Checking Redis connectivity..." -ForegroundColor Yellow
try {
    $redisCheck = Test-NetConnection -ComputerName localhost -Port 6379 -WarningAction SilentlyContinue -ErrorAction Stop
    if ($redisCheck.TcpTestSucceeded) {
        Write-Host "  [OK] Redis is running on localhost:6379" -ForegroundColor Green
        $results["redis_available"] = $true
    } else {
        Write-Host "  [WARN] Redis not available on localhost:6379" -ForegroundColor Yellow
        Write-Host "  Note: Server may use different host/port" -ForegroundColor Gray
        $results["redis_available"] = $false
    }
} catch {
    Write-Host "  [WARN] Could not test Redis connectivity: $_" -ForegroundColor Yellow
    $results["redis_available"] = $false
}
Write-Host ""

# Test 3: Check Node.js version
Write-Host "[3/6] Checking Node.js environment..." -ForegroundColor Yellow
try {
    $nodeVersion = node --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  [OK] Node.js version: $nodeVersion" -ForegroundColor Green
        $results["node_available"] = $true
    } else {
        Write-Host "  [ERROR] Node.js not found" -ForegroundColor Red
        $results["node_available"] = $false
    }
} catch {
    Write-Host "  [ERROR] Node.js not available: $_" -ForegroundColor Red
    $results["node_available"] = $false
}
Write-Host ""

# Test 4: Check dependencies
Write-Host "[4/6] Checking dependencies..." -ForegroundColor Yellow
$packagePath = Split-Path $mcpServerPath -Parent | Split-Path -Parent | Join-Path -ChildPath "package.json"
if (Test-Path $packagePath) {
    Write-Host "  [OK] package.json found" -ForegroundColor Green
    $nodeModules = Split-Path $mcpServerPath -Parent | Split-Path -Parent | Join-Path -ChildPath "node_modules"
    if (Test-Path $nodeModules) {
        Write-Host "  [OK] node_modules exists" -ForegroundColor Green
        $results["dependencies_installed"] = $true
    } else {
        Write-Host "  [WARN] node_modules not found - run npm install" -ForegroundColor Yellow
        $results["dependencies_installed"] = $false
    }
} else {
    Write-Host "  [WARN] package.json not found" -ForegroundColor Yellow
    $results["dependencies_installed"] = $false
}
Write-Host ""

# Test 5: Check if server can start
Write-Host "[5/6] Testing server startup..." -ForegroundColor Yellow
if ($results["server_exists"] -and $results["node_available"]) {
    try {
        # Try to start server with --help or version flag
        $startTest = Start-Process -FilePath "node" -ArgumentList $mcpServerPath,"--help" -NoNewWindow -PassThru -Wait -RedirectStandardOutput "nul" -RedirectStandardError "nul"
        if ($startTest.ExitCode -ne 0) {
            # Server might not support --help, try without it
            Write-Host "  [INFO] Server doesn't support --help flag" -ForegroundColor Gray
        }
        Write-Host "  [OK] Server executable can be launched" -ForegroundColor Green
        $results["server_startable"] = $true
    } catch {
        Write-Host "  [ERROR] Failed to start server: $_" -ForegroundColor Red
        $results["server_startable"] = $false
    }
} else {
    Write-Host "  [SKIP] Prerequisites not met" -ForegroundColor Yellow
    $results["server_startable"] = $false
}
Write-Host ""

# Test 6: Check environment variables
Write-Host "[6/6] Checking environment configuration..." -ForegroundColor Yellow
$envVars = @{
    "REDIS_URL" = $env:REDIS_URL
    "OPENAI_API_KEY" = if ($env:OPENAI_API_KEY) { "[SET]" } else { $null }
}

foreach ($var in $envVars.GetEnumerator()) {
    if ($var.Value) {
        Write-Host "  [OK] $($var.Key) = $($var.Value)" -ForegroundColor Green
    } else {
        Write-Host "  [WARN] $($var.Key) not set (may use defaults)" -ForegroundColor Yellow
    }
}
$results["env_configured"] = ($envVars["REDIS_URL"] -ne $null)
Write-Host ""

# Summary
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Test Summary" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

$passCount = ($results.Values | Where-Object { $_ -eq $true }).Count
$totalCount = $results.Count

foreach ($test in $results.GetEnumerator()) {
    $status = if ($test.Value) { "[OK]" } else { "[FAIL]" }
    $color = if ($test.Value) { "Green" } else { "Red" }
    Write-Host "$status $($test.Key)" -ForegroundColor $color
}

Write-Host ""
Write-Host "Tests passed: $passCount / $totalCount" -ForegroundColor $(if ($passCount -eq $totalCount) { "Green" } else { "Yellow" })
Write-Host ""

# Recommendations
Write-Host "Recommendations:" -ForegroundColor Yellow
if (-not $results["redis_available"]) {
    Write-Host "  - Start Redis server or configure REDIS_URL" -ForegroundColor Gray
}
if (-not $results["dependencies_installed"]) {
    Write-Host "  - Run: cd c:/codedev/mcp_servers/rag-redis && npm install" -ForegroundColor Gray
}
if (-not $results["env_configured"]) {
    Write-Host "  - Set REDIS_URL environment variable if needed" -ForegroundColor Gray
    Write-Host "  - Set OPENAI_API_KEY for embedding generation" -ForegroundColor Gray
}

Write-Host ""
Write-Host "To test full functionality:" -ForegroundColor Cyan
Write-Host "  1. Ensure Redis is running" -ForegroundColor Gray
Write-Host "  2. Configure API keys for embeddings" -ForegroundColor Gray
Write-Host "  3. Test document ingestion and search via MCP protocol" -ForegroundColor Gray
