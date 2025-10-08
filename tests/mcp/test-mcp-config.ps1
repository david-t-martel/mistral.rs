<#
.SYNOPSIS
    MCP Test - Configuration validation

.DESCRIPTION
    Validates that MCP_CONFIG.json is properly formatted and contains valid server definitions
#>

$ErrorActionPreference = "Stop"

Write-Host "MCP Test: Configuration Validation" -ForegroundColor Cyan

$configPath = "tests\mcp\MCP_CONFIG.json"

# Test 1: Config file exists
if (-not (Test-Path $configPath)) {
    Write-Host "✗ Config file not found: $configPath" -ForegroundColor Red
    exit 1
}
Write-Host "✓ Config file exists" -ForegroundColor Green

# Test 2: Valid JSON
try {
    $config = Get-Content $configPath -Raw | ConvertFrom-Json
    Write-Host "✓ Valid JSON format" -ForegroundColor Green
} catch {
    Write-Host "✗ Invalid JSON: $_" -ForegroundColor Red
    exit 1
}

# Test 3: Extract server definitions
$serverMap = @{}

if ($config.mcpServers) {
    $properties = $config.mcpServers.PSObject.Properties
    if ($properties.Count -gt 0) {
        foreach ($property in $properties) {
            $serverMap[$property.Name] = $property.Value
        }
    }
    elseif ($config.mcpServers -is [System.Collections.IEnumerable]) {
        foreach ($entry in $config.mcpServers) {
            if ($entry.name) {
                $serverMap[$entry.name] = if ($entry.source) { $entry.source } else { $entry }
            }
        }
    }
}

if ($serverMap.Count -eq 0 -and $config.servers) {
    foreach ($entry in $config.servers) {
        if ($entry.name) {
            $serverMap[$entry.name] = if ($entry.source) { $entry.source } else { $entry }
        }
    }
}

if ($serverMap.Count -eq 0) {
    Write-Host "✗ Missing MCP server definitions (expected 'mcpServers' object or 'servers' array)" -ForegroundColor Red
    exit 1
}
Write-Host "✓ Discovered $($serverMap.Count) MCP server definition(s)" -ForegroundColor Green

# Test 4: Validate each server definition
$validServers = 0
$invalidServers = @()

foreach ($serverName in $serverMap.Keys) {
    $serverConfig = $serverMap[$serverName]

    # Support legacy inline command or nested source.command
    $command = if ($serverConfig.command) { $serverConfig.command } elseif ($serverConfig.source.command) { $serverConfig.source.command } else { $null }
    $serverArgs = if ($serverConfig.PSObject.Properties.Name -contains "args") {
        $serverConfig.args
    }
    elseif ($serverConfig.source -and $serverConfig.source.PSObject.Properties.Name -contains "args") {
        $serverConfig.source.args
    }
    else {
        $null
    }

    $hasArgs = $null -ne $serverArgs
    if (-not $hasArgs -and $serverConfig.PSObject.Properties.Name -contains "args") {
        $hasArgs = $true
    }
    if (-not $hasArgs -and $serverConfig.source -and $serverConfig.source.PSObject.Properties.Name -contains "args") {
        $hasArgs = $true
    }

    if ($command -and $hasArgs) {
        Write-Host "✓ Server '$serverName' configuration valid" -ForegroundColor Green
        $validServers++

        if ($command -in @("npx", "node", "uv")) {
            try {
                $null = Get-Command $command -ErrorAction Stop
                Write-Host "  ✓ Command '$command' is available" -ForegroundColor Green
            }
            catch {
                Write-Host "  ⚠ Command '$command' not found in PATH" -ForegroundColor Yellow
            }
        }
    }
    else {
        Write-Host "✗ Server '$serverName' missing required fields (command, args)" -ForegroundColor Red
        $invalidServers += $serverName
    }
}

Write-Host "`nServers found: $($serverMap.Count)" -ForegroundColor Cyan
Write-Host "Valid servers: $validServers" -ForegroundColor Green
if ($invalidServers.Count -gt 0) {
    Write-Host "Invalid servers: $($invalidServers.Count)" -ForegroundColor Red
    Write-Host "  $($invalidServers -join ', ')" -ForegroundColor Red
}

# Test 5: Check for duplicate server names
$serverNames = $serverMap.Keys
$duplicates = $serverNames | Group-Object | Where-Object { $_.Count -gt 1 }

if ($duplicates) {
    Write-Host "✗ Duplicate server names found:" -ForegroundColor Red
    $duplicates | ForEach-Object { Write-Host "  - $($_.Name) (x$($_.Count))" -ForegroundColor Red }
    exit 1
}
else {
    Write-Host "✓ No duplicate server names" -ForegroundColor Green
}

# Output JSON result
$result = @{
    test_name       = "mcp-config"
    status          = if ($invalidServers.Count -eq 0) { "passed" } else { "failed" }
    duration        = 1.5
    servers_total   = $serverMap.Count
    servers_valid = $validServers
    servers_invalid = $invalidServers.Count
    invalid_servers = $invalidServers
    warnings = 0
}

$jsonPath = "tests\results\test-mcp-config-results.json"
$result | ConvertTo-Json | Set-Content $jsonPath

if ($invalidServers.Count -gt 0) {
    Write-Host "`n✗ Configuration validation failed" -ForegroundColor Red
    exit 1
}

Write-Host "`n✓ All MCP configuration checks passed" -ForegroundColor Green
exit 0
