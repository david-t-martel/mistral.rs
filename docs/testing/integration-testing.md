# Integration Testing Guide

## Overview

Integration testing validates that mistral.rs components work together correctly. Unlike unit tests that verify individual functions, integration tests ensure the entire system functions as expected when components interact.

## What Qualifies as Integration Test

An integration test in mistral.rs context:

- Tests interaction between multiple crates/modules
- Validates binary functionality end-to-end
- Tests model loading and inference pipeline
- Validates HTTP API endpoints
- Tests MCP protocol integration
- Verifies cross-platform functionality

## Test Categories

### 1. Binary Validation Tests

**Purpose**: Ensure the compiled binary works correctly

**Location**: `tests/integration/test-binary-health.ps1`

**What it tests**:

- Binary exists and is executable
- Version information is correct
- Help command works
- Basic CLI argument parsing
- Error handling for invalid arguments

**Example Test**:

```powershell
# Test binary exists and runs
$binary = "target\release\mistralrs-server.exe"
if (-not (Test-Path $binary)) {
    Write-Error "Binary not found at $binary"
    exit 1
}

# Test version output
$version = & $binary --version 2>&1
if ($version -notmatch "mistralrs-server \d+\.\d+\.\d+") {
    Write-Error "Invalid version output: $version"
    exit 1
}

# Test help command
$help = & $binary --help 2>&1
if ($help -notmatch "mistral.rs inference engine") {
    Write-Error "Help output missing expected text"
    exit 1
}
```

### 2. Model Loading Tests

**Purpose**: Validate model loading across different formats

**Location**: `tests/integration/test-mistralrs.ps1`

**Supported formats**:

- GGUF (quantized models)
- SafeTensors (HuggingFace format)
- GGML (legacy format)

**Example Test**:

```powershell
# Test GGUF model loading
$modelPath = "C:\codedev\llm\.models\qwen2.5-1.5b-it-gguf"
$modelFile = "Qwen2.5-1.5B-Instruct-Q4_K_M.gguf"

$process = Start-Process -FilePath $binary -ArgumentList @(
    "-i", "gguf",
    "-m", $modelPath,
    "-f", $modelFile,
    "--dry-run"  # Load model but exit immediately
) -NoNewWindow -Wait -PassThru -RedirectStandardOutput "model-load.txt"

if ($process.ExitCode -ne 0) {
    Write-Error "Model loading failed with exit code: $($process.ExitCode)"
    exit 1
}

# Verify model was loaded
$output = Get-Content "model-load.txt" -Raw
if ($output -notmatch "Model loaded successfully") {
    Write-Error "Model load verification failed"
    exit 1
}
```

### 3. HTTP API Tests

**Purpose**: Validate OpenAI-compatible API endpoints

**Location**: `tests/integration/test-api-endpoints.ps1`

**Endpoints tested**:

- `/v1/chat/completions` - Chat completion
- `/v1/models` - List models
- `/v1/health` - Health check
- `/v1/completions` - Text completion

**Example Test**:

```powershell
# Start server in background
$server = Start-Process -FilePath $binary -ArgumentList @(
    "--port", "8080",
    "gguf",
    "-m", $modelPath,
    "-f", $modelFile
) -NoNewWindow -PassThru

# Wait for server to be ready
Start-Sleep -Seconds 5

try {
    # Test health endpoint
    $health = Invoke-RestMethod -Uri "http://localhost:8080/v1/health" -Method Get
    if ($health.status -ne "healthy") {
        throw "Server not healthy: $($health.status)"
    }

    # Test chat completion
    $chatRequest = @{
        model = "mistral"
        messages = @(
            @{
                role = "user"
                content = "Hello, how are you?"
            }
        )
        max_tokens = 50
        temperature = 0.7
    } | ConvertTo-Json

    $response = Invoke-RestMethod -Uri "http://localhost:8080/v1/chat/completions" `
        -Method Post `
        -Body $chatRequest `
        -ContentType "application/json"

    if (-not $response.choices -or $response.choices.Count -eq 0) {
        throw "No response from chat completion"
    }

    Write-Success "API test passed"
} finally {
    # Clean up
    Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
}
```

### 4. TUI (Terminal UI) Tests

**Purpose**: Validate interactive terminal interface

**Location**: `tests/integration/run-tui-test.ps1`

**What it tests**:

- TUI launches correctly
- Keyboard input handling
- Model selection
- Chat interface
- Error display

**Example Test**:

```powershell
# Create input script for automated TUI testing
$inputScript = @"
Hello, this is a test message.
/quit
"@
$inputScript | Out-File -FilePath "tui-input.txt" -Encoding UTF8

# Run TUI with scripted input
$process = Start-Process -FilePath $binary -ArgumentList @(
    "-i",
    "gguf",
    "-m", $modelPath,
    "-f", $modelFile
) -NoNewWindow -PassThru `
  -RedirectStandardInput "tui-input.txt" `
  -RedirectStandardOutput "tui-output.txt" `
  -RedirectStandardError "tui-error.txt"

# Wait for completion (max 30 seconds)
$timeout = 30
if (-not $process.WaitForExit($timeout * 1000)) {
    Stop-Process -Id $process.Id -Force
    Write-Error "TUI test timed out after $timeout seconds"
    exit 1
}

# Verify output
$output = Get-Content "tui-output.txt" -Raw
if ($output -notmatch "Model loaded") {
    Write-Error "TUI did not load model correctly"
    exit 1
}
```

### 5. Performance Tests

**Purpose**: Validate inference performance meets requirements

**Location**: `tests/integration/test-performance.ps1`

**Metrics tested**:

- Tokens per second
- Time to first token
- Memory usage
- GPU utilization

**Example Test**:

```powershell
# Performance benchmark configuration
$benchmarkConfig = @{
    model = $modelPath
    file = $modelFile
    prompt = "Write a detailed explanation of quantum computing"
    max_tokens = 500
    iterations = 3
}

$results = @()

for ($i = 1; $i -le $benchmarkConfig.iterations; $i++) {
    Write-Info "Running benchmark iteration $i"

    $startTime = Get-Date
    $startMemory = (Get-Process -Id $PID).WorkingSet64

    $output = & $binary benchmark `
        --model $benchmarkConfig.model `
        --file $benchmarkConfig.file `
        --prompt $benchmarkConfig.prompt `
        --max-tokens $benchmarkConfig.max_tokens `
        2>&1

    $endTime = Get-Date
    $endMemory = (Get-Process -Id $PID).WorkingSet64

    # Parse output for metrics
    if ($output -match "Tokens/sec: ([\d.]+)") {
        $tokensPerSec = [double]$matches[1]
    }

    $results += @{
        Iteration = $i
        Duration = ($endTime - $startTime).TotalSeconds
        TokensPerSec = $tokensPerSec
        MemoryUsed = ($endMemory - $startMemory) / 1MB
    }
}

# Calculate averages
$avgTokensPerSec = ($results | Measure-Object -Property TokensPerSec -Average).Average
$avgMemoryMB = ($results | Measure-Object -Property MemoryUsed -Average).Average

# Validate performance thresholds
if ($avgTokensPerSec -lt 30) {
    Write-Warning "Performance below threshold: $avgTokensPerSec tokens/sec (expected >= 30)"
}

Write-Success "Performance test complete: $avgTokensPerSec tokens/sec, $avgMemoryMB MB memory"
```

## Writing Integration Tests

### Test Structure

Every integration test should follow this structure:

```powershell
<#
.SYNOPSIS
    Brief description of what the test validates

.DESCRIPTION
    Detailed explanation of test scenarios and expected outcomes

.PARAMETER ModelPath
    Path to the model directory

.PARAMETER ModelFile
    Model file name

.PARAMETER Verbose
    Enable verbose output

.EXAMPLE
    .\test-example.ps1 -ModelPath "C:\models" -ModelFile "model.gguf"
#>

param(
    [string]$ModelPath = $env:TEST_MODEL_PATH,
    [string]$ModelFile = $env:TEST_MODEL_FILE,
    [switch]$Verbose
)

# Setup
$ErrorActionPreference = "Stop"
$testName = "Integration Test Name"
$binary = "target\release\mistralrs-server.exe"

Write-Host "Starting $testName" -ForegroundColor Cyan

try {
    # Pre-flight checks
    if (-not (Test-Path $binary)) {
        throw "Binary not found: $binary"
    }

    # Test implementation
    # ... your test code here ...

    # Validation
    # ... assertions here ...

    Write-Host "✓ $testName passed" -ForegroundColor Green
    exit 0

} catch {
    Write-Error "✗ $testName failed: $_"
    exit 1
} finally {
    # Cleanup
    # ... cleanup code here ...
}
```

### Best Practices

#### 1. Isolation

Each test should be independent and not rely on state from other tests:

```powershell
# Good: Create fresh test environment
$testDir = "test-temp-$(Get-Random)"
New-Item -ItemType Directory -Path $testDir

try {
    # Run test in isolated directory
} finally {
    Remove-Item -Path $testDir -Recurse -Force
}
```

#### 2. Timeouts

Always implement timeouts to prevent hanging tests:

```powershell
# Set maximum test duration
$timeout = 60  # seconds
$timer = [System.Diagnostics.Stopwatch]::StartNew()

while ($timer.Elapsed.TotalSeconds -lt $timeout) {
    # Check condition
    if (Test-Condition) {
        break
    }
    Start-Sleep -Milliseconds 500
}

if ($timer.Elapsed.TotalSeconds -ge $timeout) {
    throw "Test timed out after $timeout seconds"
}
```

#### 3. Resource Cleanup

Always clean up resources, even on failure:

```powershell
$processes = @()

try {
    # Start processes
    $processes += Start-Process -FilePath $binary -PassThru

    # Run tests
} finally {
    # Always cleanup
    foreach ($proc in $processes) {
        if ($proc -and -not $proc.HasExited) {
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        }
    }
}
```

#### 4. Meaningful Assertions

Make assertions specific and informative:

```powershell
# Bad: Generic assertion
if (-not $result) {
    throw "Test failed"
}

# Good: Specific assertion with context
if ($result.StatusCode -ne 200) {
    throw "Expected status 200, got $($result.StatusCode). Response: $($result.Content)"
}
```

#### 5. Test Data Management

Use consistent test data from known sources:

```powershell
# Define test data
$testPrompts = @(
    @{
        Name = "Simple greeting"
        Prompt = "Hello, how are you?"
        ExpectedPattern = "fine|good|well"
    },
    @{
        Name = "Code generation"
        Prompt = "Write a Python function to calculate factorial"
        ExpectedPattern = "def factorial|def facto"
    }
)

foreach ($test in $testPrompts) {
    Write-Info "Testing: $($test.Name)"
    $response = Invoke-ModelQuery -Prompt $test.Prompt

    if ($response -notmatch $test.ExpectedPattern) {
        throw "Response validation failed for '$($test.Name)'"
    }
}
```

## Using the Master Test Runner

The master test runner (`tests/run-all-tests.ps1`) provides unified execution for all integration tests:

### Running Integration Tests

```powershell
# Run all integration tests
.\tests\run-all-tests.ps1 -Suite integration

# Run with verbose output
.\tests\run-all-tests.ps1 -Suite integration -Verbose

# Stop on first failure
.\tests\run-all-tests.ps1 -Suite integration -FailFast

# Generate HTML report
.\tests\run-all-tests.ps1 -Suite integration -OutputFormat html
```

### Test Discovery

The runner automatically discovers tests matching these patterns:

- `tests/integration/*.ps1` - All PowerShell scripts
- `tests/integration/test-*.ps1` - Preferred naming

### Parallel Execution

Some integration tests can run in parallel:

```powershell
# Enable parallel execution (where safe)
.\tests\run-all-tests.ps1 -Suite integration -Parallel
```

**Safe for parallel**:

- Binary validation
- Model format tests
- Performance benchmarks

**NOT safe for parallel**:

- HTTP server tests (port conflicts)
- TUI tests (terminal conflicts)
- Resource-intensive tests

## Debugging Integration Tests

### Enable Verbose Logging

```powershell
# Set environment variable
$env:RUST_LOG = "debug"
$env:TEST_VERBOSE = "true"

# Run test with verbose output
.\tests\integration\test-mistralrs.ps1 -Verbose
```

### Capture All Output

```powershell
# Redirect all streams to file
.\tests\integration\test-mistralrs.ps1 *> test-debug.log

# Review output
Get-Content test-debug.log | Select-String "ERROR|WARN|FAIL"
```

### Interactive Debugging

```powershell
# Add breakpoints in test script
Set-PSBreakpoint -Script .\tests\integration\test-mistralrs.ps1 -Line 50

# Run with debugger
.\tests\integration\test-mistralrs.ps1

# Debugger commands:
# c - Continue
# s - Step Into
# v - Step Over
# o - Step Out
# l - List source
# h - Help
```

### Process Monitoring

```powershell
# Monitor mistralrs processes during test
while ($true) {
    Get-Process -Name "mistralrs*" -ErrorAction SilentlyContinue |
        Select-Object Id, ProcessName, CPU, WorkingSet64, StartTime
    Start-Sleep -Seconds 1
    Clear-Host
}
```

## Common Issues and Solutions

### Issue: "Binary not found"

**Cause**: Binary hasn't been built

**Solution**:

```bash
# Build the binary first
make build-cuda-full  # For CUDA systems
make build           # For CPU-only
```

### Issue: "Model file not found"

**Cause**: Test models not downloaded

**Solution**:

```powershell
# Download test models
.\scripts\tools\download-test-models.ps1

# Or set custom model path
$env:TEST_MODEL_PATH = "C:\my\models"
$env:TEST_MODEL_FILE = "model.gguf"
```

### Issue: "Port already in use"

**Cause**: Previous test didn't clean up properly

**Solution**:

```powershell
# Find and kill process using port
netstat -ano | findstr :8080
Stop-Process -Id <PID> -Force

# Or use different port
$env:TEST_PORT = "8081"
```

### Issue: "Out of memory"

**Cause**: Large model or insufficient VRAM

**Solution**:

```powershell
# Use smaller model
$env:TEST_MODEL_FILE = "Qwen2.5-1.5B-Instruct-Q4_K_M.gguf"

# Or reduce batch size
$env:TEST_BATCH_SIZE = "1"
```

### Issue: "CUDA error"

**Cause**: CUDA/cuDNN not properly configured

**Solution**:

```bash
# Verify CUDA setup
make check-cuda-env

# Rebuild with correct CUDA version
make clean
make build-cuda-full
```

## Performance Considerations

### Test Execution Time

Target execution times for integration tests:

| Test Type             | Target Time  | Maximum Time |
| --------------------- | ------------ | ------------ |
| Binary validation     | < 5 seconds  | 10 seconds   |
| Model loading         | < 30 seconds | 60 seconds   |
| API endpoint          | < 10 seconds | 20 seconds   |
| TUI test              | < 20 seconds | 40 seconds   |
| Performance benchmark | < 2 minutes  | 5 minutes    |

### Resource Usage

Monitor resource usage during tests:

```powershell
# Create resource monitor
$monitor = {
    param($ProcessName)
    while ($true) {
        $proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue
        if ($proc) {
            [PSCustomObject]@{
                Time = Get-Date -Format "HH:mm:ss"
                CPU = $proc.CPU
                MemoryMB = [math]::Round($proc.WorkingSet64 / 1MB, 2)
                ThreadCount = $proc.Threads.Count
            }
        }
        Start-Sleep -Seconds 1
    }
}

# Start monitoring in background
$job = Start-Job -ScriptBlock $monitor -ArgumentList "mistralrs-server"

# Run test
.\tests\integration\test-performance.ps1

# Get monitoring results
$results = Receive-Job -Job $job
$results | Export-Csv "resource-usage.csv"
```

## CI/CD Integration

### GitHub Actions Integration

Integration tests run automatically in CI:

```yaml
# .github/workflows/integration-tests.yml
name: Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [windows-latest, ubuntu-latest]

    steps:
      - uses: actions/checkout@v3

      - name: Build Binary
        run: make build

      - name: Run Integration Tests
        run: |
          pwsh -File tests/run-all-tests.ps1 -Suite integration -CI

      - name: Upload Results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: test-results-${{ matrix.os }}
          path: tests/results/
```

### Local CI Simulation

Run CI tests locally before pushing:

```powershell
# Simulate CI environment
.\tests\run-all-tests.ps1 -Suite integration -CI

# Full CI pipeline
make ci
```

## Test Reporting

### JSON Report Format

```json
{
  "suite": "integration",
  "timestamp": "2025-01-01T12:00:00Z",
  "duration": 120.5,
  "summary": {
    "total": 10,
    "passed": 9,
    "failed": 1,
    "skipped": 0
  },
  "tests": [
    {
      "name": "test-binary-health",
      "status": "passed",
      "duration": 5.2,
      "output": "Binary validation successful"
    },
    {
      "name": "test-api-endpoints",
      "status": "failed",
      "duration": 15.3,
      "error": "Connection refused on port 8080"
    }
  ]
}
```

### HTML Report

Generated reports include:

- Summary statistics
- Test execution timeline
- Failure details with stack traces
- Performance metrics
- Resource usage graphs

## Next Steps

- Read [MCP Testing Guide](mcp-testing.md) for MCP server integration tests
- See [Build Testing Guide](build-testing.md) for compilation and build tests
- Review [CI/CD Testing Guide](ci-cd-testing.md) for automation setup
- Check [Testing Migration Guide](../development/testing-migration.md) if migrating from old structure

______________________________________________________________________

*Last Updated: 2025*
*Version: 1.0.0*
*Component: Integration Testing*
