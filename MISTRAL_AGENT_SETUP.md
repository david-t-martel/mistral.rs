# Mistral.rs Agent Setup Guide

_Reference: Review the [Repository Guidelines](AGENTS.md) for shared contribution standards before executing this setup._

**Complete setup guide for mistral.rs with CUDA 12.9, Flash Attention, cuDNN 9.8, Intel MKL, and MCP integration**

## Table of Contents

1. [Quick Start](#quick-start)
1. [System Requirements](#system-requirements)
1. [Installation](#installation)
1. [Configuration](#configuration)
1. [Usage](#usage)
1. [MCP Integration](#mcp-integration)
1. [Performance](#performance)
1. [Troubleshooting](#troubleshooting)

## Quick Start

```powershell
# Navigate to project
cd T:\projects\rust-mistral\mistral.rs

# Start server (basic)
.\start-mistralrs.ps1

# Start server with MCP tools
.\start-mistralrs.ps1 -EnableMCP

# Test the API
curl http://localhost:11434/v1/chat/completions `
  -H "Content-Type: application/json" `
  -d '{"model":"default","messages":[{"role":"user","content":"Hello!"}]}'
```

## System Requirements

### Hardware

- **GPU**: NVIDIA RTX 5060 Ti (16 GB VRAM)
  - Compute Capability: 12.0 (Blackwell architecture)
  - Minimum: Any CUDA-capable GPU with 4+ GB VRAM
- **RAM**: 16 GB minimum, 32 GB recommended
- **Disk**: 10 GB free space for models and cache

### Software

- **OS**: Windows 10/11 (64-bit)
- **CUDA**: 12.9
- **cuDNN**: 9.8 for CUDA 12.x
- **Driver**: NVIDIA 576.88 or later
- **Rust**: 1.89.0-msvc
- **Visual Studio**: 2022 Build Tools with MSVC v143

### Optional (for MCP)

- **Bun**: Latest version (for most MCP servers)
- **Python**: 3.11+ with uv (for Python MCP servers)
- **Redis**: 7.x (for RAG server)
- **Node.js**: 18+ (alternative to Bun)

## Installation

### 1. Prerequisites Verification

```powershell
# Check GPU and CUDA
nvidia-smi
nvcc --version

# Check Rust
rustc --version  # Should show 1.89.0

# Check Visual Studio Build Tools
vswhere -latest

# Verify PATH includes user bin
$env:PATH -split ';' | Select-String 'david\.local'
# Must show: C:\Users\david\.local\bin
```

### 2. Environment Variables

The following are automatically set by `start-mistralrs.ps1`:

- `CUDA_PATH`: C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA\\v12.9
- `CUDNN_PATH`: C:\\Program Files\\NVIDIA\\CUDNN\\v9.8
- `MKLROOT`: C:\\Program Files (x86)\\Intel\\oneAPI\\mkl\\latest

For persistent setup, run once in elevated PowerShell:

```powershell
[System.Environment]::SetEnvironmentVariable("CUDA_PATH", "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9", "Machine")
[System.Environment]::SetEnvironmentVariable("CUDNN_PATH", "C:\Program Files\NVIDIA\CUDNN\v9.8", "Machine")
[System.Environment]::SetEnvironmentVariable("MKLROOT", "C:\Program Files (x86)\Intel\oneAPI\mkl\latest", "Machine")
```

### 3. Build the Server

#### First Build (20 minutes)

```powershell
cd T:\projects\rust-mistral\mistral.rs

# Set build environment
$env:CUDNN_LIB = "C:\Program Files\NVIDIA\CUDNN\v9.8\lib\12.8\x64"
$env:LIB = "$env:CUDNN_LIB;$env:LIB"

# Build with all features
cargo build -p mistralrs-server --release --features "cuda,flash-attn,cudnn,mkl"
```

#### Fix DLL Dependencies

```powershell
# Copy Intel oneAPI DLLs to binary directory
Copy-Item "C:\Program Files (x86)\Intel\oneAPI\2025.0\bin\*.dll" `
  -Destination "C:\Users\david\.cargo\shared-target\release\" -Force
```

#### Verify Build

```powershell
& "C:\Users\david\.cargo\shared-target\release\mistralrs-server.exe" --version
# Should output: mistralrs-server 0.6.0
```

### 4. Download Model

The setup includes a Gemma 2 2B Instruct model (GGUF Q4 quantization).

**Already downloaded**:

- Location: `C:\codedev\llm\.models\gemma-2-2b-it-gguf\gemma-2-2b-it-Q4_K_M.gguf`
- Size: 1.59 GB

**To download other models**:

```powershell
# Example: Download Gemma 2 4B
$ModelUrl = "https://huggingface.co/bartowski/gemma-2-4b-it-GGUF/resolve/main/gemma-2-4b-it-Q4_K_M.gguf"
$LocalPath = "C:\codedev\llm\.models\gemma-2-4b-it-gguf\gemma-2-4b-it-Q4_K_M.gguf"
Invoke-WebRequest -Uri $ModelUrl -OutFile $LocalPath
```

## Configuration

### Build Configuration

Project uses optimized build configuration in `.cargo/config.toml`:

- **sccache**: Compilation cache for 75% faster rebuilds
- **rust-lld**: Fast linker (30-50% improvement over link.exe)
- **release-dev profile**: Fast development builds (opt-level=2, thin LTO)

### MCP Configuration

MCP servers are defined in `MCP_CONFIG.json`. The configuration includes 9 servers:

1. Memory - Session state persistence
1. Filesystem - Local file operations (scoped to project)
1. Sequential Thinking - Multi-step reasoning
1. GitHub - Repository management
1. Fetch - HTTP requests
1. Time - Date/time operations
1. Serena Claude - Advanced reasoning
1. Python FileOps Enhanced - High-performance file operations
1. RAG Redis - Vector search and semantic retrieval

**Note**: This file is based on your canonical `C:\Users\david\mcp.json` and should not be duplicated.

## Usage

### Starting the Server

#### Basic Mode (No MCP)

```powershell
.\start-mistralrs.ps1
```

#### With MCP Tools

```powershell
.\start-mistralrs.ps1 -EnableMCP
```

#### Custom Port

```powershell
.\start-mistralrs.ps1 -Port 8080
```

#### Different Model

```powershell
.\start-mistralrs.ps1 -ModelPath "C:\path\to\your\model.gguf"
```

### API Examples

#### Python (OpenAI SDK)

```python
import openai

client = openai.OpenAI(
    base_url="http://localhost:11434/v1",
    api_key="EMPTY"
)

response = client.chat.completions.create(
    model="default",
    messages=[
        {"role": "system", "content": "You are a helpful coding assistant."},
        {"role": "user", "content": "Explain Rust ownership"}
    ],
    max_tokens=500,
    temperature=0.7
)

print(response.choices[0].message.content)
```

#### cURL

```bash
curl http://localhost:11434/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "default",
    "messages": [
      {"role": "user", "content": "What is CUDA?"}
    ],
    "max_tokens": 300
  }'
```

#### With Tool Calling (MCP Enabled)

```python
# When started with -EnableMCP, tools are automatically available
response = client.chat.completions.create(
    model="default",
    messages=[
        {"role": "user", "content": "List all .rs files in the src directory"}
    ]
)
# The model will automatically use the Filesystem MCP tool
```

### Development Workflow

#### Rebuild After Code Changes

```powershell
# Fast development build (8-12 minutes)
cargo build -p mistralrs-server --profile release-dev --features "cuda,flash-attn,cudnn,mkl"

# Or use alias
cargo brd -p mistralrs-server --features "cuda,flash-attn,cudnn,mkl"
```

#### Check Build Cache Statistics

```powershell
cargo stats
# Or directly:
sccache --show-stats
```

#### Clean Build

```powershell
cargo clean -p mistralrs-server
```

## MCP Integration

### Prerequisites for MCP

**Required for most servers**:

```powershell
# Install Bun
irm bun.sh/install.ps1 | iex
```

**For Python servers** (already configured):

- Python 3.11+ with uv
- Installed at: `C:\Users\david\.local\bin`

**For RAG Redis**:

```powershell
# Install Redis (Windows)
# Or use WSL/Docker
docker run -d -p 6379:6379 redis:latest
```

### Testing MCP Servers

#### Test Individual Server

```powershell
# Test filesystem server
bun x @modelcontextprotocol/server-filesystem@2025.8.21 "T:/projects/rust-mistral/mistral.rs"

# Test memory server
bun x @modelcontextprotocol/server-memory@2025.8.4
```

#### Verify MCP in Server Logs

When starting with `-EnableMCP`, look for:

```
MCP enabled with config: T:\projects\rust-mistral\mistral.rs\MCP_CONFIG.json
[MCP] Connecting to 9 servers...
[MCP] Successfully connected to: Memory, Filesystem, Sequential Thinking, ...
```

### GitHub Token Setup (for GitHub Server)

```powershell
$env:GITHUB_PERSONAL_ACCESS_TOKEN = "ghp_your_token_here"

# Or permanently:
[System.Environment]::SetEnvironmentVariable("GITHUB_PERSONAL_ACCESS_TOKEN", "ghp_your_token", "User")
```

## Performance

### Expected Performance (Gemma 2 2B Q4)

| Metric               | Value                             |
| -------------------- | --------------------------------- |
| Tokens/second        | 30-50 tokens/sec                  |
| Time-to-first-token  | 200-500 ms                        |
| VRAM Usage           | 2-3 GB (model) + 1-2 GB (context) |
| Max Context          | 8192 tokens                       |
| Concurrent Sequences | Up to 16                          |

### Hardware Utilization

**GPU** (RTX 5060 Ti):

- Utilization: 70-90% during generation
- VRAM: ~4-5 GB total
- Compute: SM 12.0 (Blackwell)

**CPU**:

- Minimal load during inference
- Used for tokenization and post-processing

**Build Performance**:

- First build: ~20 minutes
- Rebuild (with sccache): 4-5 minutes
- Rebuild (fast profile): 8-12 minutes

### Optimization Tips

**For Faster Inference**:

- Use Q4 quantization (current setup)
- Enable Flash Attention (enabled)
- Reduce context length if not needed
- Use batch size = 1 for low latency

**For Higher Quality**:

- Use Q6 or Q8 quantization (requires more VRAM)
- Increase temperature (0.7-0.9)
- Use larger models (4B, 7B if VRAM allows)

**For Lower VRAM**:

- Use Q3 or Q2 quantization
- Reduce max sequence length
- Disable KV cache quantization

## Troubleshooting

### Build Issues

#### Error: "cudnn.lib not found"

```powershell
# Ensure CUDNN_LIB is set correctly
$env:CUDNN_LIB = "C:\Program Files\NVIDIA\CUDNN\v9.8\lib\12.8\x64"
$env:LIB = "$env:CUDNN_LIB;$env:LIB"
```

#### Error: "sccache server startup failed"

```powershell
# Change port or disable sccache temporarily
$env:SCCACHE_SERVER_PORT = "4227"

# Or disable
$env:RUSTC_WRAPPER = ""
```

#### Error: "Intel MKL link error"

Use the included MKL libraries or rebuild without MKL:

```powershell
cargo build -p mistralrs-server --release --features "cuda,flash-attn,cudnn"
```

### Runtime Issues

#### Error: "DLL not found" (exit code -1073741515)

```powershell
# Copy Intel oneAPI DLLs to binary directory
Copy-Item "C:\Program Files (x86)\Intel\oneAPI\2025.0\bin\*.dll" `
  -Destination "C:\Users\david\.cargo\shared-target\release\" -Force

# Or ensure PATH includes Intel oneAPI
$env:PATH = "C:\Program Files (x86)\Intel\oneAPI\2025.0\bin;$env:PATH"
```

#### Error: "CUDA initialization failed"

```powershell
# Check CUDA installation
nvidia-smi
nvcc --version

# Verify CUDA_PATH
$env:CUDA_PATH
```

#### Error: "Out of memory" (CUDA OOM)

- Reduce context length
- Use lower quantization (Q3, Q2)
- Use smaller model
- Reduce batch size

### MCP Issues

#### Error: "MCP server failed to start"

```powershell
# Check Bun installation
bun --version

# Test server manually
bun x @modelcontextprotocol/server-filesystem@2025.8.21 "."
```

#### Error: "No tools available"

1. Ensure server started with `-EnableMCP`
1. Check MCP_CONFIG.json exists
1. Verify bun/uv are in PATH
1. Check server logs for connection errors

#### Error: "GitHub tools not working"

```powershell
# Set GitHub token
$env:GITHUB_PERSONAL_ACCESS_TOKEN = "ghp_your_token"

# Verify token
curl -H "Authorization: Bearer $env:GITHUB_PERSONAL_ACCESS_TOKEN" https://api.github.com/user
```

#### Error: "RAG Redis connection failed"

```powershell
# Check Redis is running
redis-cli ping
# Should return: PONG

# Or start Redis
docker run -d -p 6379:6379 redis:latest
```

### PATH Issues

**Critical**: `C:\Users\david\.local\bin` must remain on PATH

```powershell
# Verify
$env:PATH -split ';' | Select-String 'david\.local'

# If missing, add it
$env:PATH = "C:\Users\david\.local\bin;$env:PATH"
```

## Maintenance

### Update Model

```powershell
# Download new model
Invoke-WebRequest -Uri "https://huggingface.co/..." -OutFile "path/to/model.gguf"

# Update start script or use -ModelPath
.\start-mistralrs.ps1 -ModelPath "path/to/new/model.gguf"
```

### Update Server

```powershell
cd T:\projects\rust-mistral\mistral.rs

# Pull latest changes
git pull

# Rebuild
cargo build -p mistralrs-server --release --features "cuda,flash-attn,cudnn,mkl"

# Copy DLLs again
Copy-Item "C:\Program Files (x86)\Intel\oneAPI\2025.0\bin\*.dll" `
  -Destination "C:\Users\david\.cargo\shared-target\release\" -Force
```

### Monitor Performance

```powershell
# Watch GPU utilization
nvidia-smi -l 1

# Check server logs
# Logs are output to console when server is running
```

### Clean Cache

```powershell
# Clear sccache
sccache --stop-server
Remove-Item "T:\projects\rust-mistral\sccache-cache\*" -Recurse -Force

# Clear Cargo cache
cargo clean
```

## Additional Resources

### Documentation

- [Project README](README.md)
- [MCP Integration Guide](docs/MCP/README.md)
- [HTTP API Documentation](docs/HTTP.md)
- [Build Optimization Summary](BUILD_OPTIMIZATION_SUMMARY.md)

### Files

- Binary: `C:\Users\david\.cargo\shared-target\release\mistralrs-server.exe`
- Model: `C:\codedev\llm\.models\gemma-2-2b-it-gguf\gemma-2-2b-it-Q4_K_M.gguf`
- MCP Config: `T:\projects\rust-mistral\mistral.rs\MCP_CONFIG.json`
- Launch Script: `T:\projects\rust-mistral\mistral.rs\start-mistralrs.ps1`
- Build Config: `T:\projects\rust-mistral\mistral.rs\.cargo\config.toml`
- AI Agent Guide: `T:\projects\rust-mistral\mistral.rs\llms.txt`

### Canonical Configuration

- MCP Servers: `C:\Users\david\mcp.json` (do not modify or duplicate)

### Support

- [mistral.rs Discord](https://discord.gg/SZrecqK8qw)
- [GitHub Issues](https://github.com/EricLBuehler/mistral.rs/issues)
- [MCP Documentation](https://github.com/modelcontextprotocol)

______________________________________________________________________

**Setup Complete!** You now have a fully functional mistral.rs server with CUDA acceleration and MCP tool integration.
