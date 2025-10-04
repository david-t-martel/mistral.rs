# PowerShell health check script for mistral.rs server
# Tests: HTTP endpoint, MCP connectivity, model loading, resource usage

param(
    [Parameter()][string]$Host = "localhost",
    [Parameter()][int]$Port = 8080,
    [Parameter()][int]$Timeout = 10,
    [Parameter()][switch]$Verbose
)

# Exit codes
$EXIT_SUCCESS = 0
$EXIT_HTTP_FAILED = 1
$EXIT_HEALTH_FAILED = 2
$EXIT_MCP_FAILED = 3
$EXIT_RESOURCE_FAILED = 4

# ============================================================================
# Helper Functions
# ============================================================================

function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Blue
}

function Write-Success {
    param([string]$Message)
    Write-Host "[✓] $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[!] $Message" -ForegroundColor Yellow
}

function Write-Failure {
    param([string]$Message)
    Write-Host "[✗] $Message" -ForegroundColor Red
}

# ============================================================================
# Health Checks
# ============================================================================

function Test-HttpConnectivity {
    Write-Info "Checking HTTP connectivity..."

    try {
        $response = Invoke-WebRequest -Uri "http://${Host}:${Port}/health" `
            -TimeoutSec $Timeout `
            -UseBasicParsing `
            -ErrorAction Stop

        Write-Success "HTTP server is reachable (HTTP $($response.StatusCode))"
        return $true
    } catch {
        Write-Failure "HTTP server is not reachable: $_"
        return $false
    }
}

function Test-HealthEndpoint {
    Write-Info "Checking /health endpoint..."

    try {
        $response = Invoke-RestMethod -Uri "http://${Host}:${Port}/health" `
            -TimeoutSec $Timeout `
            -ErrorAction Stop

        if ($Verbose) {
            Write-Host "Response: $($response | ConvertTo-Json -Compress)"
        }

        # Check if response indicates health
        $responseStr = $response | ConvertTo-Json -Compress
        if ($responseStr -match "(ok|healthy|running|status|state)") {
            Write-Success "Health endpoint responding"
            return $true
        } else {
            Write-Warning "Health endpoint returned unexpected response"
            return $false
        }
    } catch {
        Write-Failure "Health endpoint failed: $_"
        return $false
    }
}

function Test-ApiEndpoint {
    Write-Info "Checking API endpoints..."

    try {
        $response = Invoke-RestMethod -Uri "http://${Host}:${Port}/v1/models" `
            -TimeoutSec $Timeout `
            -ErrorAction Stop

        if ($Verbose) {
            Write-Host "Models: $($response | ConvertTo-Json -Compress)"
        }

        Write-Success "API endpoint accessible"
        return $true
    } catch {
        Write-Warning "Could not check API endpoints (may not be implemented)"
        return $true  # Not critical
    }
}

function Test-McpConnectivity {
    Write-Info "Checking MCP server connectivity..."

    # Check if MCP server processes are running
    $mcpProcesses = Get-Process | Where-Object {
        $_.ProcessName -like "*mcp*" -or
        $_.Path -like "*mcp*server*"
    }

    if ($mcpProcesses) {
        Write-Success "MCP servers detected ($($mcpProcesses.Count) processes)"
        return $true
    } else {
        Write-Warning "No MCP servers detected (may not be configured)"
        return $true  # Not critical
    }
}

function Test-ResourceUsage {
    Write-Info "Checking resource usage..."

    # Find mistralrs-server process
    $process = Get-Process | Where-Object {
        $_.ProcessName -like "mistralrs-server*" -or
        $_.Path -like "*mistralrs-server*"
    } | Select-Object -First 1

    if (-not $process) {
        Write-Warning "mistralrs-server process not found"
        return $true  # May be in container
    }

    # Memory usage
    $memMB = [math]::Round($process.WorkingSet64 / 1MB, 2)
    Write-Success "Memory usage: ${memMB}MB"

    # CPU usage
    $cpuPercent = [math]::Round($process.CPU, 2)
    Write-Success "CPU time: ${cpuPercent}s"

    # Check GPU if nvidia-smi available
    try {
        $gpuInfo = nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits 2>$null
        if ($gpuInfo) {
            Write-Success "GPU memory usage: ${gpuInfo}MB"
        }
    } catch {
        # nvidia-smi not available
    }

    return $true
}

function Test-ModelLoaded {
    Write-Info "Checking if model is loaded..."

    try {
        $body = @{
            prompt = "test"
            max_tokens = 1
        } | ConvertTo-Json

        $response = Invoke-RestMethod -Uri "http://${Host}:${Port}/v1/completions" `
            -Method Post `
            -ContentType "application/json" `
            -Body $body `
            -TimeoutSec $Timeout `
            -ErrorAction Stop

        if ($Verbose) {
            Write-Host "Completion response: $($response | ConvertTo-Json -Compress)"
        }

        Write-Success "Model appears to be loaded"
        return $true
    } catch {
        Write-Warning "Could not verify model loading (may need authentication)"
        return $true  # Not critical for basic health check
    }
}

function Test-DiskSpace {
    Write-Info "Checking disk space..."

    # Check drive where models are stored
    $modelDrive = if (Test-Path "C:\codedev\llm\.models") {
        "C:"
    } else {
        "C:"
    }

    try {
        $drive = Get-PSDrive -Name $modelDrive.TrimEnd(':') -ErrorAction Stop
        $freeGB = [math]::Round($drive.Free / 1GB, 2)
        Write-Success "Available disk space on ${modelDrive}: ${freeGB}GB"
        return $true
    } catch {
        Write-Warning "Could not check disk space"
        return $true
    }
}

# ============================================================================
# Main Execution
# ============================================================================

Write-Host ""
Write-Info "mistral.rs Health Check"
Write-Info "Target: http://${Host}:${Port}"
Write-Host ""

$exitCode = $EXIT_SUCCESS

# Run checks
if (-not (Test-HttpConnectivity)) { $exitCode = $EXIT_HTTP_FAILED }
if (-not (Test-HealthEndpoint)) { $exitCode = $EXIT_HEALTH_FAILED }
Test-ApiEndpoint | Out-Null
Test-McpConnectivity | Out-Null
Test-ModelLoaded | Out-Null
if (-not (Test-ResourceUsage)) { $exitCode = $EXIT_RESOURCE_FAILED }
Test-DiskSpace | Out-Null

Write-Host ""
if ($exitCode -eq $EXIT_SUCCESS) {
    Write-Success "All critical health checks passed"
} else {
    Write-Failure "Some health checks failed"
}
Write-Host ""

exit $exitCode
