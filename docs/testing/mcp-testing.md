# MCP Testing Guide

## Overview

Model Context Protocol (MCP) testing validates the integration between mistral.rs and various MCP servers. MCP servers communicate via JSON-RPC over stdio (stdin/stdout), providing tools and context to language models during inference.

## MCP Architecture

### Protocol Basics

MCP servers are **NOT HTTP servers**. They communicate through:

- **Transport**: stdio (stdin/stdout)
- **Protocol**: JSON-RPC 2.0
- **Message Format**: Line-delimited JSON
- **Version**: `2025-06-18` (current)

### Communication Flow

```
mistralrs-server.exe
    ├── MCP Client (built-in)
    │   ├── Spawns MCP server process
    │   ├── Sends JSON-RPC requests via stdin
    │   └── Receives JSON-RPC responses via stdout
    └── Inference Engine
        └── Uses MCP tools during generation
```

## MCP Server Configuration

### Configuration File Structure

**Location**: `tests/mcp/MCP_CONFIG.json`

```json
{
  "servers": [
    {
      "name": "Memory",
      "source": {
        "type": "Process",
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-memory@2025.8.4"],
        "env": {
          "MEMORY_FILE_PATH": "T:/projects/rust-mistral/mistral.rs/memory.json",
          "MCP_PROTOCOL_VERSION": "2025-06-18"
        }
      }
    }
  ],
  "auto_register_tools": true,
  "tool_timeout_secs": 180,
  "max_concurrent_calls": 3
}
```

### Available MCP Servers

| Server              | Purpose               | Command                                                | Key Features                           |
| ------------------- | --------------------- | ------------------------------------------------------ | -------------------------------------- |
| Memory              | Session state         | `npx @modelcontextprotocol/server-memory`              | Persistent memory across conversations |
| Filesystem          | File operations       | `npx @modelcontextprotocol/server-filesystem`          | Read, write, list files                |
| Sequential Thinking | Multi-step reasoning  | `npx @modelcontextprotocol/server-sequential-thinking` | Chain-of-thought prompting             |
| GitHub              | Repository operations | `npx @modelcontextprotocol/server-github`              | Issues, PRs, commits                   |
| Fetch               | HTTP requests         | `npx @modelcontextprotocol/server-fetch`               | Web scraping, API calls                |
| Time                | Date/time utilities   | `npx @theo.foobar/mcp-time`                            | Current time, timezone conversion      |
| RAG-Redis           | Vector search         | `rag-redis-mcp-server.exe`                             | Redis-backed RAG system                |
| Serena Claude       | Code analysis         | `uv run python mcp_server.py`                          | Python code intelligence               |
| Python FileOps      | Enhanced file ops     | `uv run python -m desktop_commander.mcp_server`        | Advanced file management               |

## Testing MCP Servers

### 1. Configuration Validation

**Test**: `tests/mcp/test-mcp-config.ps1`

Validates that MCP configuration is correct:

```powershell
# Load and validate MCP configuration
$configPath = "tests/mcp/MCP_CONFIG.json"

try {
    $config = Get-Content $configPath -Raw | ConvertFrom-Json

    # Validate structure
    if (-not $config.servers) {
        throw "Missing 'servers' array in config"
    }

    foreach ($server in $config.servers) {
        # Check required fields
        if (-not $server.name) {
            throw "Server missing 'name' field"
        }

        if (-not $server.source.command) {
            throw "Server '$($server.name)' missing command"
        }

        # Validate command exists
        $cmd = $server.source.command
        if ($cmd -eq "npx") {
            # NPX should be available via Node.js
            $npx = Get-Command npx -ErrorAction SilentlyContinue
            if (-not $npx) {
                throw "npx not found - Node.js required for $($server.name)"
            }
        } elseif ($cmd -notlike "*.exe") {
            # Check for Python/uv commands
            if ($cmd -ne "uv" -and $cmd -ne "python") {
                Write-Warning "Unknown command type: $cmd"
            }
        } else {
            # Binary should exist
            if (-not (Test-Path $cmd)) {
                throw "Binary not found: $cmd for $($server.name)"
            }
        }
    }

    Write-Success "Configuration valid"
} catch {
    Write-Error "Configuration invalid: $_"
    exit 1
}
```

### 2. Server Lifecycle Testing

**Test**: `tests/mcp/test-mcp-servers.ps1`

Tests starting, running, and stopping MCP servers:

```powershell
# Test MCP server lifecycle
function Test-MCPServerLifecycle {
    param(
        [PSCustomObject]$Server
    )

    Write-Info "Testing server: $($Server.name)"

    try {
        # Start server process
        $processArgs = @{
            FilePath = $Server.source.command
            ArgumentList = $Server.source.args
            NoNewWindow = $true
            PassThru = $true
            RedirectStandardInput = "tests/results/mcp-$($Server.name).in"
            RedirectStandardOutput = "tests/results/mcp-$($Server.name).out"
            RedirectStandardError = "tests/results/mcp-$($Server.name).err"
        }

        # Set environment variables
        if ($Server.source.env) {
            foreach ($key in $Server.source.env.PSObject.Properties.Name) {
                [Environment]::SetEnvironmentVariable($key, $Server.source.env.$key)
            }
        }

        $process = Start-Process @processArgs

        # Wait for server to initialize
        Start-Sleep -Seconds 2

        if ($process.HasExited) {
            $errorOutput = Get-Content "tests/results/mcp-$($Server.name).err" -Raw
            throw "Server exited immediately. Error: $errorOutput"
        }

        # Send initialization request
        $initRequest = @{
            jsonrpc = "2.0"
            method = "initialize"
            params = @{
                protocolVersion = "2025-06-18"
                capabilities = @{
                    tools = @{ listing = $true }
                }
            }
            id = 1
        } | ConvertTo-Json -Compress

        # Write request to stdin
        $initRequest | Out-File "tests/results/mcp-$($Server.name).in" -Encoding UTF8

        # Wait for response
        Start-Sleep -Seconds 2

        # Read response
        $response = Get-Content "tests/results/mcp-$($Server.name).out" -Raw

        if ($response) {
            $responseObj = $response | ConvertFrom-Json
            if ($responseObj.result) {
                Write-Success "$($Server.name) initialized successfully"
                return @{
                    Success = $true
                    Server = $Server.name
                    Process = $process
                    Capabilities = $responseObj.result
                }
            }
        }

        throw "No valid response received"

    } catch {
        Write-Error "Failed to test $($Server.name): $_"
        return @{
            Success = $false
            Server = $Server.name
            Error = $_.ToString()
        }
    } finally {
        # Cleanup
        if ($process -and -not $process.HasExited) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        }
    }
}
```

### 3. Tool Discovery Testing

**Test**: `tests/mcp/test-tool-discovery.ps1`

Validates that MCP servers expose expected tools:

```powershell
# Test tool discovery for each MCP server
function Test-MCPToolDiscovery {
    param(
        [PSCustomObject]$Server,
        [System.Diagnostics.Process]$Process
    )

    Write-Info "Discovering tools for $($Server.name)"

    # Send list_tools request
    $listToolsRequest = @{
        jsonrpc = "2.0"
        method = "tools/list"
        params = @{}
        id = 2
    } | ConvertTo-Json -Compress

    # Send request
    $listToolsRequest | Add-Content "tests/results/mcp-$($Server.name).in"

    # Wait for response
    Start-Sleep -Seconds 2

    # Read latest output
    $output = Get-Content "tests/results/mcp-$($Server.name).out" -Tail 10 |
              Where-Object { $_ -match '"id"\s*:\s*2' }

    if ($output) {
        $response = $output | ConvertFrom-Json

        if ($response.result.tools) {
            Write-Success "Found $($response.result.tools.Count) tools"

            foreach ($tool in $response.result.tools) {
                Write-Info "  - $($tool.name): $($tool.description)"
            }

            return @{
                Success = $true
                Server = $Server.name
                Tools = $response.result.tools
            }
        }
    }

    throw "No tools discovered"
}
```

### 4. Tool Invocation Testing

**Test**: `tests/mcp/test-tool-invocation.ps1`

Tests actual tool execution:

```powershell
# Test tool invocation
function Test-MCPToolInvocation {
    param(
        [PSCustomObject]$Server,
        [System.Diagnostics.Process]$Process,
        [string]$ToolName,
        [hashtable]$Arguments
    )

    Write-Info "Invoking tool: $ToolName on $($Server.name)"

    # Create tool call request
    $toolCallRequest = @{
        jsonrpc = "2.0"
        method = "tools/call"
        params = @{
            name = $ToolName
            arguments = $Arguments
        }
        id = 3
    } | ConvertTo-Json -Depth 10 -Compress

    # Send request
    $toolCallRequest | Add-Content "tests/results/mcp-$($Server.name).in"

    # Wait for response
    Start-Sleep -Seconds 5

    # Read response
    $output = Get-Content "tests/results/mcp-$($Server.name).out" -Tail 20 |
              Where-Object { $_ -match '"id"\s*:\s*3' }

    if ($output) {
        $response = $output | ConvertFrom-Json

        if ($response.result) {
            Write-Success "Tool invoked successfully"
            return @{
                Success = $true
                Result = $response.result
            }
        } elseif ($response.error) {
            throw "Tool error: $($response.error.message)"
        }
    }

    throw "No response from tool invocation"
}

# Example test cases for specific servers
$testCases = @{
    "Time" = @{
        Tool = "get_current_time"
        Args = @{ timezone = "UTC" }
        Validate = { param($result) $result.time -match '\d{4}-\d{2}-\d{2}' }
    }
    "Filesystem" = @{
        Tool = "list_directory"
        Args = @{ path = "." }
        Validate = { param($result) $result.entries.Count -gt 0 }
    }
    "Memory" = @{
        Tool = "store"
        Args = @{ key = "test"; value = "data" }
        Validate = { param($result) $result.success -eq $true }
    }
}
```

### 5. Integration Testing with mistralrs

**Test**: `tests/mcp/test-mistralrs-mcp-integration.ps1`

Tests MCP integration with the inference engine:

```powershell
# Test mistralrs with MCP servers
function Test-MistralRSWithMCP {
    param(
        [string]$ModelPath,
        [string]$ModelFile,
        [string]$MCPConfig = "tests/mcp/MCP_CONFIG.json"
    )

    Write-Info "Testing mistralrs with MCP integration"

    $binary = "target\release\mistralrs-server.exe"

    # Start server with MCP config
    $server = Start-Process -FilePath $binary -ArgumentList @(
        "--port", "8080",
        "--mcp-config", $MCPConfig,
        "gguf",
        "-m", $ModelPath,
        "-f", $ModelFile
    ) -NoNewWindow -PassThru

    try {
        # Wait for server startup
        Start-Sleep -Seconds 10

        # Test chat completion with tool use
        $chatRequest = @{
            model = "mistral"
            messages = @(
                @{
                    role = "user"
                    content = "What time is it in Tokyo?"
                }
            )
            tools = @(
                @{
                    type = "function"
                    function = @{
                        name = "get_current_time"
                        description = "Get current time in a timezone"
                        parameters = @{
                            type = "object"
                            properties = @{
                                timezone = @{
                                    type = "string"
                                    description = "Timezone name"
                                }
                            }
                        }
                    }
                }
            )
            tool_choice = "auto"
        } | ConvertTo-Json -Depth 10

        $response = Invoke-RestMethod -Uri "http://localhost:8080/v1/chat/completions" `
            -Method Post `
            -Body $chatRequest `
            -ContentType "application/json"

        # Verify tool was called
        if ($response.choices[0].message.tool_calls) {
            Write-Success "MCP tool invoked during inference"
            return $true
        } else {
            throw "No tool calls in response"
        }

    } finally {
        Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
    }
}
```

## Server-Specific Testing

### Memory Server

Tests persistence and retrieval:

```powershell
# Test memory persistence
$storeResult = Invoke-MCPTool -Server "Memory" -Tool "store" -Args @{
    key = "test_key"
    value = "test_value"
}

$retrieveResult = Invoke-MCPTool -Server "Memory" -Tool "retrieve" -Args @{
    key = "test_key"
}

if ($retrieveResult.value -ne "test_value") {
    throw "Memory retrieval failed"
}
```

### Filesystem Server

Tests file operations:

```powershell
# Test file operations
$testFile = "test-mcp-file.txt"
$testContent = "MCP test content"

# Write file
$writeResult = Invoke-MCPTool -Server "Filesystem" -Tool "write_file" -Args @{
    path = $testFile
    content = $testContent
}

# Read file
$readResult = Invoke-MCPTool -Server "Filesystem" -Tool "read_file" -Args @{
    path = $testFile
}

if ($readResult.content -ne $testContent) {
    throw "File content mismatch"
}

# Cleanup
Remove-Item $testFile -Force
```

### GitHub Server

Tests repository operations:

```powershell
# Test GitHub API (requires token)
if ($env:GITHUB_TOKEN) {
    $repoInfo = Invoke-MCPTool -Server "GitHub" -Tool "get_repository" -Args @{
        owner = "EricLBuehler"
        repo = "mistral.rs"
    }

    if (-not $repoInfo.name) {
        throw "Failed to fetch repository info"
    }
}
```

### RAG-Redis Server

Tests vector search functionality:

```powershell
# Ensure Redis is running
$redis = Get-Process -Name "redis-server" -ErrorAction SilentlyContinue
if (-not $redis) {
    Write-Warning "Redis not running - starting it"
    Start-Process "redis-server" -NoNewWindow
    Start-Sleep -Seconds 2
}

# Test document indexing
$indexResult = Invoke-MCPTool -Server "RAG-Redis" -Tool "index_document" -Args @{
    content = "This is a test document about MCP testing"
    metadata = @{ type = "test" }
}

# Test search
$searchResult = Invoke-MCPTool -Server "RAG-Redis" -Tool "search" -Args @{
    query = "MCP testing"
    limit = 5
}

if ($searchResult.results.Count -eq 0) {
    throw "Search returned no results"
}
```

## Debugging MCP Issues

### 1. Enable Debug Logging

```powershell
# Set environment variables for debugging
$env:RUST_LOG = "debug"
$env:MCP_DEBUG = "true"
$env:NODE_DEBUG = "mcp"

# Run test with debug output
.\tests\mcp\test-mcp-servers.ps1 -Verbose
```

### 2. Monitor stdio Communication

```powershell
# Create monitoring script
$monitor = {
    param($InputFile, $OutputFile)

    Write-Host "Monitoring MCP communication" -ForegroundColor Cyan

    # Monitor input (requests)
    Get-Content $InputFile -Wait | ForEach-Object {
        Write-Host "[REQUEST] $_" -ForegroundColor Yellow
    }

    # Monitor output (responses) in another window
    Start-Process powershell -ArgumentList "-Command", "Get-Content '$OutputFile' -Wait"
}

# Start monitoring
Start-Job -ScriptBlock $monitor -ArgumentList @(
    "tests/results/mcp-Memory.in",
    "tests/results/mcp-Memory.out"
)
```

### 3. Test Individual Server

```powershell
# Isolate and test single server
function Test-SingleMCPServer {
    param([string]$ServerName)

    $config = Get-Content "tests/mcp/MCP_CONFIG.json" -Raw | ConvertFrom-Json
    $server = $config.servers | Where-Object { $_.name -eq $ServerName }

    if (-not $server) {
        throw "Server '$ServerName' not found in config"
    }

    # Start server manually
    Write-Host "Starting $ServerName manually..." -ForegroundColor Cyan
    Write-Host "Command: $($server.source.command) $($server.source.args -join ' ')"

    $proc = Start-Process -FilePath $server.source.command `
        -ArgumentList $server.source.args `
        -NoNewWindow -PassThru

    Write-Host "Server PID: $($proc.Id)"
    Write-Host "Press Enter to send initialize request..."
    Read-Host

    # Send test requests manually
    # ... interactive debugging ...
}

Test-SingleMCPServer -ServerName "Memory"
```

### 4. Validate JSON-RPC Messages

```powershell
# Validate JSON-RPC format
function Test-JSONRPCMessage {
    param([string]$Message)

    try {
        $obj = $Message | ConvertFrom-Json

        # Check required fields
        if (-not $obj.jsonrpc -or $obj.jsonrpc -ne "2.0") {
            throw "Invalid JSON-RPC version"
        }

        if (-not $obj.method -and -not $obj.result -and -not $obj.error) {
            throw "Message must have method (request) or result/error (response)"
        }

        if ($obj.method -and -not $obj.id) {
            Write-Warning "Request without ID (notification)"
        }

        Write-Success "Valid JSON-RPC message"
        return $true

    } catch {
        Write-Error "Invalid JSON-RPC: $_"
        return $false
    }
}
```

## Performance Testing

### MCP Latency Benchmarks

```powershell
# Benchmark MCP tool invocation latency
function Benchmark-MCPLatency {
    param(
        [string]$ServerName,
        [string]$ToolName,
        [hashtable]$Arguments,
        [int]$Iterations = 100
    )

    $results = @()

    for ($i = 1; $i -le $Iterations; $i++) {
        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

        $result = Invoke-MCPTool -Server $ServerName -Tool $ToolName -Args $Arguments

        $stopwatch.Stop()
        $results += $stopwatch.ElapsedMilliseconds

        if ($i % 10 -eq 0) {
            Write-Progress -Activity "Benchmarking" -Status "$i/$Iterations" `
                -PercentComplete (($i / $Iterations) * 100)
        }
    }

    $stats = $results | Measure-Object -Average -Minimum -Maximum -StandardDeviation

    [PSCustomObject]@{
        Server = $ServerName
        Tool = $ToolName
        Iterations = $Iterations
        AverageMs = [math]::Round($stats.Average, 2)
        MinMs = $stats.Minimum
        MaxMs = $stats.Maximum
        StdDev = [math]::Round($stats.StandardDeviation, 2)
        P95 = ($results | Sort-Object)[[int]($Iterations * 0.95)]
        P99 = ($results | Sort-Object)[[int]($Iterations * 0.99)]
    }
}

# Run benchmarks
$benchmarks = @(
    @{ Server = "Time"; Tool = "get_current_time"; Args = @{} },
    @{ Server = "Memory"; Tool = "retrieve"; Args = @{ key = "test" } },
    @{ Server = "Filesystem"; Tool = "list_directory"; Args = @{ path = "." } }
)

foreach ($bench in $benchmarks) {
    $result = Benchmark-MCPLatency @bench
    $result | Format-Table
}
```

### Concurrent Load Testing

```powershell
# Test concurrent MCP calls
function Test-MCPConcurrency {
    param(
        [int]$ConcurrentCalls = 10,
        [string]$ServerName = "Memory"
    )

    $jobs = @()

    for ($i = 1; $i -le $ConcurrentCalls; $i++) {
        $jobs += Start-Job -ScriptBlock {
            param($Index, $Server)

            $result = Invoke-MCPTool -Server $Server -Tool "store" -Args @{
                key = "concurrent_$Index"
                value = "value_$Index"
            }

            return @{
                Index = $Index
                Success = $result.success
                Duration = $result.duration
            }
        } -ArgumentList $i, $ServerName
    }

    # Wait for all jobs
    $results = $jobs | Wait-Job | Receive-Job

    $successCount = ($results | Where-Object { $_.Success }).Count
    Write-Host "Concurrent test: $successCount/$ConcurrentCalls successful"

    return $results
}
```

## Common Issues and Solutions

### Issue: "Server exited immediately"

**Cause**: Missing dependencies or incorrect path

**Solution**:

```powershell
# Check Node.js installation
node --version
npm --version

# Install/update MCP server package
npm install -g @modelcontextprotocol/server-memory@latest

# For Python servers, check uv
uv --version
```

### Issue: "No response from server"

**Cause**: Server not fully initialized

**Solution**:

```powershell
# Increase initialization timeout
$initTimeout = 10  # seconds
Start-Sleep -Seconds $initTimeout

# Check server logs
Get-Content "tests/results/mcp-$ServerName.err" -Tail 20
```

### Issue: "Invalid JSON-RPC response"

**Cause**: Protocol version mismatch or malformed request

**Solution**:

```powershell
# Ensure correct protocol version
$env:MCP_PROTOCOL_VERSION = "2025-06-18"

# Validate JSON formatting
$request | ConvertTo-Json -Depth 10 | Test-Json
```

### Issue: "Tool not found"

**Cause**: Tool not registered or wrong name

**Solution**:

```powershell
# List available tools first
$tools = Get-MCPTools -Server $ServerName
$tools | ForEach-Object { Write-Host "- $($_.name)" }

# Use exact tool name (case-sensitive)
$result = Invoke-MCPTool -Server $ServerName -Tool "exact_tool_name"
```

## CI/CD Integration

### GitHub Actions Workflow

```yaml
# .github/workflows/mcp-validation.yml
name: MCP Server Validation

on:
  push:
    paths:
      - 'tests/mcp/**'
      - 'mistralrs-mcp/**'
  pull_request:
    paths:
      - 'tests/mcp/**'

jobs:
  test-mcp:
    runs-on: windows-latest

    steps:
      - uses: actions/checkout@v3

      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '20'

      - name: Install MCP servers
        run: |
          npm install -g @modelcontextprotocol/server-memory
          npm install -g @modelcontextprotocol/server-filesystem

      - name: Build mistralrs
        run: make build

      - name: Run MCP tests
        run: |
          pwsh -File tests/run-all-tests.ps1 -Suite mcp -CI

      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: mcp-test-results
          path: tests/results/
```

## Best Practices

### 1. Server Initialization

Always properly initialize servers before use:

```powershell
# Proper initialization sequence
1. Start server process
2. Wait for startup (2-5 seconds)
3. Send initialize request
4. Wait for initialize response
5. Verify capabilities
6. Send initialized notification
7. Server ready for tool calls
```

### 2. Error Handling

Handle all error conditions:

```powershell
try {
    $result = Invoke-MCPTool -Server $server -Tool $tool -Args $args
} catch [System.TimeoutException] {
    Write-Error "MCP call timed out"
    # Retry logic
} catch [System.IO.IOException] {
    Write-Error "Server communication failed"
    # Restart server
} catch {
    Write-Error "Unexpected error: $_"
    # Fallback behavior
}
```

### 3. Resource Management

Clean up resources properly:

```powershell
$servers = @()
try {
    # Start servers
    $servers = Start-MCPServers

    # Run tests
    # ...

} finally {
    # Always cleanup
    foreach ($server in $servers) {
        Stop-MCPServer -Server $server
    }
}
```

### 4. Logging

Maintain comprehensive logs:

```powershell
# Log all MCP communication
$global:MCPLog = @()

function Log-MCPMessage {
    param($Direction, $Server, $Message)

    $entry = @{
        Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss.fff"
        Direction = $Direction  # "Request" or "Response"
        Server = $Server
        Message = $Message
    }

    $global:MCPLog += $entry

    # Also write to file
    $entry | ConvertTo-Json -Compress |
        Add-Content "tests/results/mcp-communication.log"
}
```

## Next Steps

- Review [Integration Testing Guide](integration-testing.md) for end-to-end testing
- See [Build Testing Guide](build-testing.md) for compilation tests
- Check [CI/CD Testing Guide](ci-cd-testing.md) for automation setup
- Read [Testing Migration Guide](../development/testing-migration.md) for migration help

______________________________________________________________________

*Last Updated: 2025*
*Version: 1.0.0*
*Component: MCP Testing*
