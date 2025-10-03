# MCP Server Validation Script
# Tests all configured MCP servers for functionality and compatibility

param(
    [switch]$Verbose,
    [string]$ServerName = $null,
    [switch]$UseInspector
)

$ErrorActionPreference = "Continue"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "MCP Server Validation Suite" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

# Configuration
$mcpConfigPath = "T:\projects\rust-mistral\mistral.rs\MCP_CONFIG.json"
$resultsPath = "T:\projects\rust-mistral\mistral.rs\mcp-validation-results.json"
$logPath = "T:\projects\rust-mistral\mistral.rs\mcp-test.log"

# Load MCP configuration
Write-Host "[INFO] Loading MCP configuration..." -ForegroundColor Gray
if (-not (Test-Path $mcpConfigPath)) {
    Write-Host "[ERROR] MCP config not found: $mcpConfigPath" -ForegroundColor Red
    exit 1
}

try {
    $mcpConfig = Get-Content $mcpConfigPath -Raw | ConvertFrom-Json
    $servers = $mcpConfig.servers
    Write-Host "[OK] Loaded $($servers.Count) servers from config" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] Failed to parse MCP config: $_" -ForegroundColor Red
    exit 1
}

Write-Host ""

# Test results storage
$testResults = @{
    timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    servers = @{}
}

# Helper function to test command availability
function Test-CommandAvailable {
    param([string]$Command)
    try {
        $null = Get-Command $Command -ErrorAction Stop
        return $true
    } catch {
        return $false
    }
}

# Helper function to test server startup
function Test-ServerStartup {
    param(
        [string]$Name,
        [object]$Source,
        [int]$TimeoutSeconds = 10
    )
    
    Write-Host "Testing: $Name" -ForegroundColor Yellow
    
    $result = @{
        name = $Name
        command = $Source.command
        args = $Source.args -join " "
        status = "unknown"
        errors = @()
        warnings = @()
        info = @()
    }
    
    # Check if command exists
    if (-not (Test-CommandAvailable $Source.command)) {
        $result.status = "command_not_found"
        $result.errors += "Command not found: $($Source.command)"
        Write-Host "  [ERROR] Command not found: $($Source.command)" -ForegroundColor Red
        return $result
    }
    
    $result.info += "Command available: $($Source.command)"
    Write-Host "  [OK] Command available" -ForegroundColor Green
    
    # Try to start the server
    try {
        Write-Host "  [INFO] Testing server startup..." -ForegroundColor Gray
        
        # Prepare environment variables
        $envVars = @{}
        if ($Source.env) {
            $Source.env.PSObject.Properties | ForEach-Object {
                $envVars[$_.Name] = $_.Value
            }
        }
        
        # Create process start info
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = $Source.command
        $psi.Arguments = $Source.args -join " "
        $psi.UseShellExecute = $false
        $psi.RedirectStandardOutput = $true
        $psi.RedirectStandardError = $true
        $psi.CreateNoWindow = $true
        
        # Add environment variables
        foreach ($key in $envVars.Keys) {
            $value = $envVars[$key]
            # Expand environment variables in value
            $value = [System.Environment]::ExpandEnvironmentVariables($value)
            $psi.EnvironmentVariables[$key] = $value
        }
        
        # Set working directory if specified
        if ($Source.work_dir) {
            $psi.WorkingDirectory = $Source.work_dir
        }
        
        # Start process
        $process = New-Object System.Diagnostics.Process
        $process.StartInfo = $psi
        
        $stdout = New-Object System.Text.StringBuilder
        $stderr = New-Object System.Text.StringBuilder
        
        $process.add_OutputDataReceived({
            param($sender, $e)
            if ($e.Data) {
                [void]$stdout.AppendLine($e.Data)
            }
        })
        
        $process.add_ErrorDataReceived({
            param($sender, $e)
            if ($e.Data) {
                [void]$stderr.AppendLine($e.Data)
            }
        })
        
        $started = $process.Start()
        $process.BeginOutputReadLine()
        $process.BeginErrorReadLine()
        
        if (-not $started) {
            $result.status = "failed_to_start"
            $result.errors += "Process failed to start"
            Write-Host "  [ERROR] Failed to start process" -ForegroundColor Red
            return $result
        }
        
        $result.info += "Process started with PID: $($process.Id)"
        Write-Host "  [OK] Process started (PID: $($process.Id))" -ForegroundColor Green
        
        # Wait a bit for initialization
        Start-Sleep -Milliseconds 2000
        
        # Check if still running
        if ($process.HasExited) {
            $result.status = "crashed"
            $result.errors += "Process exited with code: $($process.ExitCode)"
            
            $stdoutText = $stdout.ToString()
            $stderrText = $stderr.ToString()
            
            if ($stdoutText) {
                $result.info += "STDOUT: $stdoutText"
            }
            if ($stderrText) {
                $result.errors += "STDERR: $stderrText"
            }
            
            Write-Host "  [ERROR] Process crashed (exit code: $($process.ExitCode))" -ForegroundColor Red
            if ($Verbose -and $stderrText) {
                Write-Host "  [DEBUG] Error output:" -ForegroundColor Gray
                Write-Host "    $stderrText" -ForegroundColor Gray
            }
        } else {
            $result.status = "running"
            $result.info += "Server is running"
            Write-Host "  [OK] Server is running" -ForegroundColor Green
            
            # Kill the process
            $process.Kill()
            $process.WaitForExit(5000)
            Write-Host "  [INFO] Server stopped cleanly" -ForegroundColor Gray
        }
        
    } catch {
        $result.status = "error"
        $result.errors += "Exception: $_"
        Write-Host "  [ERROR] Exception: $_" -ForegroundColor Red
    }
    
    Write-Host ""
    return $result
}

# Test each server
$serverIndex = 1
foreach ($server in $servers) {
    Write-Host "[$serverIndex/$($servers.Count)] " -NoNewline -ForegroundColor Cyan
    
    # Skip if specific server requested and this isn't it
    if ($ServerName -and $server.name -ne $ServerName) {
        $serverIndex++
        continue
    }
    
    $result = Test-ServerStartup -Name $server.name -Source $server.source
    $testResults.servers[$server.name] = $result
    
    $serverIndex++
}

# Summary
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Validation Summary" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

$statusCounts = @{
    running = 0
    crashed = 0
    command_not_found = 0
    failed_to_start = 0
    error = 0
    unknown = 0
}

foreach ($result in $testResults.servers.Values) {
    $status = $result.status
    $statusCounts[$status]++
    
    $statusIcon = switch ($status) {
        "running" { "[OK]" }
        "crashed" { "[CRASH]" }
        "command_not_found" { "[MISSING]" }
        "failed_to_start" { "[FAIL]" }
        "error" { "[ERROR]" }
        default { "[?]" }
    }
    
    $color = switch ($status) {
        "running" { "Green" }
        "crashed" { "Red" }
        "command_not_found" { "Yellow" }
        "failed_to_start" { "Red" }
        "error" { "Red" }
        default { "Gray" }
    }
    
    Write-Host "$statusIcon $($result.name) - $status" -ForegroundColor $color
    
    if ($Verbose -and $result.errors.Count -gt 0) {
        foreach ($error in $result.errors) {
            Write-Host "    Error: $error" -ForegroundColor Gray
        }
    }
}

Write-Host ""
Write-Host "Statistics:" -ForegroundColor Yellow
Write-Host "  Running: $($statusCounts.running)" -ForegroundColor Green
Write-Host "  Crashed: $($statusCounts.crashed)" -ForegroundColor Red
Write-Host "  Missing: $($statusCounts.command_not_found)" -ForegroundColor Yellow
Write-Host "  Failed: $($statusCounts.failed_to_start)" -ForegroundColor Red
Write-Host "  Errors: $($statusCounts.error)" -ForegroundColor Red

$totalTested = $testResults.servers.Count
$successRate = if ($totalTested -gt 0) { 
    [math]::Round(($statusCounts.running / $totalTested) * 100, 1) 
} else { 0 }

Write-Host ""
Write-Host "Success Rate: $successRate% ($($statusCounts.running)/$totalTested)" -ForegroundColor $(
    if ($successRate -ge 80) { "Green" }
    elseif ($successRate -ge 50) { "Yellow" }
    else { "Red" }
)

# Save results
try {
    $testResults | ConvertTo-Json -Depth 10 | Out-File -FilePath $resultsPath -Encoding UTF8
    Write-Host ""
    Write-Host "[INFO] Results saved to: $resultsPath" -ForegroundColor Gray
} catch {
    Write-Host "[WARN] Failed to save results: $_" -ForegroundColor Yellow
}

# Recommendations
Write-Host ""
Write-Host "Recommendations:" -ForegroundColor Yellow

$missingServers = $testResults.servers.Values | Where-Object { $_.status -eq "command_not_found" }
if ($missingServers.Count -gt 0) {
    Write-Host "  [MISSING COMMANDS]" -ForegroundColor Red
    foreach ($srv in $missingServers) {
        Write-Host "    - Install: $($srv.command)" -ForegroundColor Gray
    }
}

$crashedServers = $testResults.servers.Values | Where-Object { $_.status -eq "crashed" }
if ($crashedServers.Count -gt 0) {
    Write-Host "  [CRASHED SERVERS]" -ForegroundColor Red
    foreach ($srv in $crashedServers) {
        Write-Host "    - Review: $($srv.name)" -ForegroundColor Gray
        Write-Host "      Command: $($srv.command) $($srv.args)" -ForegroundColor Gray
    }
}

Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  1. Fix missing/crashed servers" -ForegroundColor Gray
Write-Host "  2. Review results: $resultsPath" -ForegroundColor Gray
Write-Host "  3. Update MCP_CONFIG.json to remove broken servers" -ForegroundColor Gray
Write-Host "  4. Re-run validation: .\test-mcp-servers.ps1" -ForegroundColor Gray
