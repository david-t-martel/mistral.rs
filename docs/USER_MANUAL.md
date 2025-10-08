# mistral.rs User Manual

**Version:** 1.0\
**Last Updated:** 2025-02-10\
**For:** mistralrs-server v0.4.3 with CUDA support

______________________________________________________________________

## Table of Contents

1. [Quick Start](#quick-start)
1. [Installation Verification](#installation-verification)
1. [Starting the Server](#starting-the-server)
1. [Model Selection](#model-selection)
1. [API Usage](#api-usage)
1. [MCP Integration](#mcp-integration)
1. [Performance Tuning](#performance-tuning)
1. [Monitoring](#monitoring)
1. [Troubleshooting](#troubleshooting)
1. [Advanced Configuration](#advanced-configuration)

______________________________________________________________________

## Quick Start

### Prerequisites

- ✅ Windows 11 with PowerShell 7+
- ✅ NVIDIA GPU with CUDA 12.8+
- ✅ 16GB+ VRAM recommended
- ✅ Models downloaded (see setup)

### 5-Minute Setup

```powershell
# 1. Navigate to project
cd T:\projects\rust-mistral\mistral.rs

# 2. Verify installation
.\test-mistralrs.ps1 -Quick

# 3. Start server with default model
.\launch-mistralrs.ps1

# 4. Test in another terminal
curl http://localhost:8080/health
```

**Expected Output:**

```json
{"status":"ok","model":"gemma-2-2b-it"}
```

______________________________________________________________________

## Installation Verification

### Check Binary

```powershell
Get-Item "C:\Users\david\.cargo\shared-target\release\mistralrs-server.exe"
```

**Expected:** File exists, ~382 MB

### Check CUDA

```powershell
nvidia-smi
```

**Expected:** Shows RTX 5060 Ti, 16GB VRAM

### Check Models

```powershell
Get-ChildItem "C:\codedev\llm\.models" -Recurse -Filter "*.gguf"
```

**Expected:** 4 GGUF files (8.67 GB total)

### Run Full Verification

```powershell
.\test-mistralrs.ps1 -Quick
```

**Expected:** 5+ tests passing (62%+)

______________________________________________________________________

## Starting the Server

### Default Start (Gemma 2B)

```powershell
.\launch-mistralrs.ps1
```

**Parameters:**

- Port: 8080
- Model: Gemma 2 2B Instruct
- VRAM: ~3 GB
- Tokens/sec: 60-80 (estimated)

### Specific Model

```powershell
# Fast responses (1.5B model)
.\launch-mistralrs.ps1 -Model qwen1.5b

# Code analysis (3B model)
.\launch-mistralrs.ps1 -Model qwen3b

# Complex reasoning (7B model)
.\launch-mistralrs.ps1 -Model qwen7b
```

### With MCP Integration

```powershell
.\launch-mistralrs.ps1 -EnableMCP
```

**Note:** MCP servers must be configured in `MCP_CONFIG.json`

### Custom Port

```powershell
.\launch-mistralrs.ps1 -Port 8081
```

### Background Mode

```powershell
Start-Job -ScriptBlock {
    & "T:\projects\rust-mistral\mistral.rs\launch-mistralrs.ps1"
}
```

______________________________________________________________________

## Model Selection

### Available Models

| Model        | Size    | VRAM  | Speed        | Best For                    |
| ------------ | ------- | ----- | ------------ | --------------------------- |
| **qwen1.5b** | 0.92 GB | ~2 GB | 80-100 tok/s | Quick queries, simple tasks |
| **gemma2**   | 1.59 GB | ~3 GB | 60-80 tok/s  | General use, balanced       |
| **qwen3b**   | 1.80 GB | ~4 GB | 50-70 tok/s  | Code analysis, refactoring  |
| **qwen7b**   | 4.36 GB | ~8 GB | 25-40 tok/s  | Complex reasoning, research |

### Model Capabilities

#### Qwen 1.5B Instruct

- **Strengths:** Speed, low memory, quick responses
- **Use Cases:**
  - Command generation
  - Simple Q&A
  - Text completion
- **Limitations:** Less nuanced understanding

#### Gemma 2 2B Instruct

- **Strengths:** Balance of speed and quality
- **Use Cases:**
  - General chat
  - Document summarization
  - Creative writing
- **Limitations:** Not specialized for code

#### Qwen 2.5 Coder 3B

- **Strengths:** Code understanding, syntax analysis
- **Use Cases:**
  - Code review
  - Refactoring suggestions
  - Bug identification
  - Documentation generation
- **Limitations:** Smaller context window

#### Qwen 2.5 7B Instruct

- **Strengths:** Best reasoning, largest context
- **Use Cases:**
  - Architecture design
  - Complex problem solving
  - Research assistance
  - Multi-step reasoning
- **Limitations:** Slower, more VRAM

### Choosing the Right Model

**Decision Tree:**

```
Need code analysis? → qwen3b
Need speed? → qwen1.5b
Need balanced performance? → gemma2
Need deep reasoning? → qwen7b
```

______________________________________________________________________

## API Usage

### Health Check

```powershell
curl http://localhost:8080/health
```

### Chat Completion (Basic)

```powershell
$body = @{
    model = "gemma2"
    messages = @(
        @{ role = "user"; content = "Explain CUDA in one sentence" }
    )
    max_tokens = 100
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://localhost:8080/v1/chat/completions" `
    -Method Post -Body $body -ContentType "application/json"
```

### Chat Completion (Advanced)

```powershell
$body = @{
    model = "qwen3b"
    messages = @(
        @{ role = "system"; content = "You are a helpful coding assistant" }
        @{ role = "user"; content = "Optimize this Python loop: for i in range(len(arr)):" }
    )
    temperature = 0.7
    top_p = 0.9
    max_tokens = 500
    stream = $false
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://localhost:8080/v1/chat/completions" `
    -Method Post -Body $body -ContentType "application/json"
```

### Streaming Response

```powershell
$body = @{
    model = "gemma2"
    messages = @(
        @{ role = "user"; content = "Write a short story" }
    )
    stream = $true
} | ConvertTo-Json

# Stream handling requires special processing
$response = Invoke-WebRequest -Uri "http://localhost:8080/v1/chat/completions" `
    -Method Post -Body $body -ContentType "application/json" -UseBasicParsing

# Parse SSE stream
$response.Content -split "`n" | Where-Object { $_ -match "data: " } | ForEach-Object {
    $json = $_ -replace "data: ", ""
    if ($json -ne "[DONE]") {
        $data = $json | ConvertFrom-Json
        Write-Host $data.choices[0].delta.content -NoNewline
    }
}
```

### Python Example

```python
import requests

url = "http://localhost:8080/v1/chat/completions"
payload = {
    "model": "gemma2",
    "messages": [
        {"role": "user", "content": "Hello!"}
    ],
    "max_tokens": 100
}

response = requests.post(url, json=payload)
result = response.json()
print(result['choices'][0]['message']['content'])
```

### cURL Example

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemma2",
    "messages": [{"role": "user", "content": "Hello!"}],
    "max_tokens": 100
  }'
```

______________________________________________________________________

## MCP Integration

### What is MCP?

Model Context Protocol (MCP) allows the LLM to use external tools like:

- File system operations
- Database queries
- Web searches
- API calls
- Memory/context persistence

### Configuring MCP Servers

Edit `MCP_CONFIG.json`:

```json
{
  "servers": [
    {
      "name": "Filesystem",
      "source": {
        "type": "Process",
        "command": "bun",
        "args": ["x", "@modelcontextprotocol/server-filesystem@2025.8.21", "T:/projects"],
        "env": {
          "BUN_RUNTIME": "bun",
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

| Server              | Status         | Purpose                                |
| ------------------- | -------------- | -------------------------------------- |
| Memory              | ✅ Working     | Persistent context between sessions    |
| Filesystem          | ✅ Working     | Read/write files, directory operations |
| Sequential Thinking | ✅ Working     | Step-by-step reasoning chains          |
| GitHub              | ⚠️ Needs Token | GitHub API operations                  |
| Fetch               | ✅ Working     | HTTP requests                          |
| Time                | ❌ Deprecated  | Date/time operations (broken)          |

### Testing MCP Servers

```powershell
.\test-mcp-servers.ps1 -Verbose
```

### Using MCP in Requests

When MCP is enabled, the model can automatically invoke tools:

```powershell
$body = @{
    model = "qwen7b"
    messages = @(
        @{ role = "user"; content = "Read the contents of README.md" }
    )
    tools_enabled = $true
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://localhost:8080/v1/chat/completions" `
    -Method Post -Body $body -ContentType "application/json"
```

The model will:

1. Recognize it needs to read a file
1. Call the Filesystem MCP server
1. Return the file contents in its response

______________________________________________________________________

## Performance Tuning

### GPU Optimization

#### Check VRAM Usage

```powershell
nvidia-smi dmon -s mu -c 10
```

**Optimal VRAM:** 70-90% utilization

#### Monitor Temperature

```powershell
nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader -l 1
```

**Safe Range:** 65-85°C

### Parameter Tuning

#### Temperature

- **0.1-0.3:** Factual, deterministic responses
- **0.7-0.9:** Creative, varied responses
- **1.0+:** Very creative, unpredictable

#### Top-P (Nucleus Sampling)

- **0.9:** Recommended default
- **0.95:** More variety
- **0.5-0.7:** More focused

#### Max Tokens

- **50-100:** Short answers
- **500:** Paragraph responses
- **2000+:** Long-form content

### Batch Processing

For multiple requests:

```powershell
# Process files in parallel
$files = Get-ChildItem *.txt
$files | ForEach-Object -Parallel {
    $content = Get-Content $_.FullName -Raw
    $body = @{
        model = "gemma2"
        messages = @(@{ role = "user"; content = "Summarize: $content" })
    } | ConvertTo-Json
    
    Invoke-RestMethod -Uri "http://localhost:8080/v1/chat/completions" `
        -Method Post -Body $body -ContentType "application/json"
} -ThrottleLimit 3
```

______________________________________________________________________

## Monitoring

### Server Logs

Logs are written to stdout. Capture them:

```powershell
.\launch-mistralrs.ps1 2>&1 | Tee-Object -FilePath "server.log"
```

### Performance Metrics

#### Tokens Per Second

```powershell
# Measure inference speed
Measure-Command {
    $body = @{
        model = "gemma2"
        messages = @(@{ role = "user"; content = "Count to 100" })
        max_tokens = 100
    } | ConvertTo-Json
    
    Invoke-RestMethod -Uri "http://localhost:8080/v1/chat/completions" `
        -Method Post -Body $body -ContentType "application/json"
}
```

#### Time to First Token

Monitor in server logs for `TTFT` metric.

### GPU Monitoring Dashboard

```powershell
# Continuous monitoring
while ($true) {
    Clear-Host
    Write-Host "=== GPU Status ===" -ForegroundColor Cyan
    nvidia-smi --query-gpu=name,temperature.gpu,utilization.gpu,utilization.memory,memory.used,memory.total --format=csv,noheader
    Write-Host ""
    Write-Host "=== Server Status ===" -ForegroundColor Cyan
    try {
        $health = Invoke-RestMethod "http://localhost:8080/health"
        Write-Host "Status: $($health.status)" -ForegroundColor Green
    } catch {
        Write-Host "Status: OFFLINE" -ForegroundColor Red
    }
    Start-Sleep -Seconds 2
}
```

______________________________________________________________________

## Troubleshooting

### Server Won't Start

**Problem:** Binary not found

```
Solution: Check binary location
Get-Item "C:\Users\david\.cargo\shared-target\release\mistralrs-server.exe"
```

**Problem:** Port already in use

```powershell
Solution: Use different port or kill existing process
Get-Process | Where-Object { $_.Name -match "mistralrs" } | Stop-Process
```

**Problem:** CUDA not detected

```
Solution: Check NVIDIA drivers
nvidia-smi
# Update drivers if needed
```

### Model Loading Fails

**Problem:** Model file not found

```powershell
Solution: Verify model path
Test-Path "C:\codedev\llm\.models\gemma-2-2b-it-gguf\gemma-2-2b-it-Q4_K_M.gguf"
```

**Problem:** Out of VRAM

```
Solution: Use smaller model or close other GPU applications
nvidia-smi
# Switch to qwen1.5b (smallest) or kill other GPU processes
```

**Problem:** Slow loading

```
Check: SSD vs HDD, model size, available RAM
Monitor with: nvidia-smi dmon
```

### Inference Issues

**Problem:** Very slow responses

```
Causes:
1. Wrong model for task (use smaller model)
2. High temperature (reduce to 0.7)
3. Large max_tokens (reduce)
4. Background GPU usage (check nvidia-smi)

Solution: Restart server, use qwen1.5b for testing
```

**Problem:** Poor quality responses

```
Causes:
1. Model too small (upgrade to qwen7b)
2. Low temperature (increase to 0.8-0.9)
3. Truncated context (increase max_tokens)

Solution: Use appropriate model for task complexity
```

**Problem:** Timeout errors

```
Causes:
1. Server overloaded
2. Model too large for prompt
3. Network issues

Solution: Increase timeout, reduce concurrent requests
```

### MCP Issues

**Problem:** MCP server won't start

```powershell
Solution: Test individually
.\test-mcp-servers.ps1 -ServerName "Memory" -Verbose
```

**Problem:** Tool not being invoked

```
Causes:
1. MCP not enabled in request
2. Tool not registered
3. Server crashed

Solution: Check MCP logs, restart with -EnableMCP
```

**Problem:** Bun command not found

```powershell
Solution: Install bun or use node alternative
npm install -g bun
# Or update config to use node instead
```

### Common Error Messages

#### "CUDA out of memory"

```
Fix: Use smaller model or reduce batch size
.\launch-mistralrs.ps1 -Model qwen1.5b
```

#### "Model format not supported"

```
Fix: Ensure using GGUF format
Supported: *.gguf
Not supported: *.sbs, *.bin (for some models)
```

#### "Connection refused"

```
Fix: Server not running or wrong port
Check: curl http://localhost:8080/health
```

______________________________________________________________________

## Advanced Configuration

### Environment Variables

```powershell
# CUDA settings
$env:CUDA_VISIBLE_DEVICES = "0"  # Use first GPU
$env:CUDA_LAUNCH_BLOCKING = "1"  # Synchronous CUDA calls (debugging)

# Performance
$env:OMP_NUM_THREADS = "8"  # OpenMP threads for CPU fallback
$env:MKL_NUM_THREADS = "8"  # MKL threads

# Logging
$env:RUST_LOG = "info"  # Log level: trace, debug, info, warn, error
$env:RUST_BACKTRACE = "1"  # Enable backtraces
```

### Custom Launch Script

Create `my-config.ps1`:

```powershell
# Custom configuration
$env:CUDA_VISIBLE_DEVICES = "0"
$env:RUST_LOG = "warn"

# Model path
$modelPath = "C:\codedev\llm\.models\qwen2.5-coder-3b-gguf\Qwen2.5-Coder-3B-Instruct-Q4_K_M.gguf"

# Launch
& "C:\Users\david\.cargo\shared-target\release\mistralrs-server.exe" `
    --port 8080 `
    gguf `
    -m $modelPath `
    -t 4 `
    --prompt-template chatml
```

### Model-Specific Settings

#### For Code Tasks (qwen3b)

````powershell
$body = @{
    model = "qwen3b"
    temperature = 0.3  # More deterministic
    top_p = 0.9
    max_tokens = 1000
    stop = @("```", "END")  # Stop at code block end
}
````

#### For Creative Writing (gemma2)

```powershell
$body = @{
    model = "gemma2"
    temperature = 0.9  # More creative
    top_p = 0.95
    max_tokens = 2000
    presence_penalty = 0.6  # Encourage variety
}
```

#### For Factual Q&A (qwen7b)

```powershell
$body = @{
    model = "qwen7b"
    temperature = 0.2  # Very deterministic
    top_p = 0.85
    max_tokens = 500
}
```

### Multi-Model Setup

Run multiple models simultaneously:

```powershell
# Terminal 1: Fast model on port 8080
.\launch-mistralrs.ps1 -Model qwen1.5b -Port 8080

# Terminal 2: Code model on port 8081
.\launch-mistralrs.ps1 -Model qwen3b -Port 8081

# Terminal 3: Large model on port 8082
.\launch-mistralrs.ps1 -Model qwen7b -Port 8082
```

Load balancer script:

```powershell
function Invoke-SmartInference {
    param([string]$Prompt, [string]$Task = "general")
    
    $port = switch ($Task) {
        "code" { 8081 }
        "complex" { 8082 }
        default { 8080 }
    }
    
    $body = @{
        model = "auto"
        messages = @(@{ role = "user"; content = $Prompt })
    } | ConvertTo-Json
    
    Invoke-RestMethod -Uri "http://localhost:$port/v1/chat/completions" `
        -Method Post -Body $body -ContentType "application/json"
}

# Usage
Invoke-SmartInference -Prompt "Optimize this code" -Task "code"
```

______________________________________________________________________

## Appendix A: API Reference

### Endpoints

| Endpoint               | Method | Purpose                  |
| ---------------------- | ------ | ------------------------ |
| `/health`              | GET    | Server health check      |
| `/v1/models`           | GET    | List available models    |
| `/v1/chat/completions` | POST   | Generate chat completion |
| `/v1/completions`      | POST   | Generate text completion |

### Request Schema

```json
{
  "model": "string (required)",
  "messages": [
    {
      "role": "system|user|assistant",
      "content": "string"
    }
  ],
  "temperature": 0.7,
  "top_p": 0.9,
  "max_tokens": 500,
  "stream": false,
  "stop": ["string"],
  "presence_penalty": 0.0,
  "frequency_penalty": 0.0
}
```

### Response Schema

```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1234567890,
  "model": "gemma2",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "response text"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 20,
    "total_tokens": 30
  }
}
```

______________________________________________________________________

## Appendix B: Keyboard Shortcuts

When running interactively in terminal:

- **Ctrl+C**: Stop server
- **Ctrl+Z**: Suspend (then `fg` to resume on Unix)
- **Ctrl+L**: Clear terminal (doesn't affect server)

______________________________________________________________________

## Appendix C: Quick Reference Card

```
QUICK COMMANDS
├─ Start: .\launch-mistralrs.ps1
├─ Test: .\test-mistralrs.ps1 -Quick
├─ Health: curl http://localhost:8080/health
├─ GPU: nvidia-smi
└─ MCP: .\test-mcp-servers.ps1

MODEL SELECTION
├─ Fast: -Model qwen1.5b
├─ Balanced: -Model gemma2
├─ Code: -Model qwen3b
└─ Complex: -Model qwen7b

PORTS
├─ Default: 8080
├─ Alternative: -Port 8081
└─ Multiple: Run separate instances

TROUBLESHOOTING
├─ Check binary: Test-Path "C:\Users\david\.cargo\shared-target\release\mistralrs-server.exe"
├─ Check CUDA: nvidia-smi
├─ Check models: Get-ChildItem "C:\codedev\llm\.models" -Recurse -Filter "*.gguf"
└─ View logs: .\launch-mistralrs.ps1 2>&1 | Tee-Object -FilePath "server.log"
```

______________________________________________________________________

## Support & Resources

- **Documentation**: See `TEST_RESULTS.md` for performance benchmarks
- **Project**: T:\\projects\\rust-mistral\\mistral.rs
- **Logs**: Enable with `$env:RUST_LOG = "debug"`
- **Issues**: Check `TODO.md` for known issues

______________________________________________________________________

**End of User Manual**
