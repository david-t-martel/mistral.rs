# Mistral.rs Development Agent Setup - Complete Guide

**Date**: 2025-10-02  
**Status**: Environment Configured, Build In Progress  
**Location**: `T:\projects\rust-mistral\mistral.rs`

## Executive Summary

This document provides a complete setup guide for building mistral.rs as a local TUI agent optimized for code generation, with full CUDA acceleration and MCP (Model Context Protocol) integration.

## System Configuration ✓

### Hardware
- **GPU**: NVIDIA GeForce RTX 5060 Ti (16GB VRAM)
- **Driver**: 576.88
- **CUDA**: 12.9 (with versions 12.1, 12.6, 12.8, 13.0 available)

### Software Environment
- **OS**: Windows 11
- **Shell**: PowerShell 7.5.3
- **Rust**: 1.89.0 (MSVC toolchain)
- **Cargo**: 1.89.0
- **Visual Studio**: 2022 Build Tools
  - MSVC: 14.44.35207
  - cl.exe: `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\cl.exe`

### Development Tools
- **UV**: Python environment manager (`C:\users\david\.local\bin\uv.exe`)
- **Python**: 3.13.7 (via UV)
- **huggingface-cli**: Installed via `uv pip install`
- **Git**: 2.51.0
- **CMake**: 4.1.2

### Key Paths
- **User Tools**: `C:\Users\david\.local\bin` (✓ on PATH)
- **Additional Tools**: `C:\Users\david\bin`
- **CUDA**: `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9`
- **cuDNN**: `C:\Program Files\NVIDIA\CUDNN` (v9.8)
- **Intel MKL**: `C:\Program Files (x86)\Intel\oneAPI\mkl\latest`
- **HF_HOME**: `C:\codedev\llm\.cache\huggingface`

## Environment Variables (Session)

```powershell
$env:NVCC_CCBIN = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\cl.exe"
$env:CUDA_PATH = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9"
$env:CUDNN_PATH = "C:\Program Files\NVIDIA\CUDNN"
$env:MKLROOT = "C:\Program Files (x86)\Intel\oneAPI\mkl\latest"
$env:ONEAPI_ROOT = "C:\Program Files (x86)\Intel\oneAPI"
$env:HF_HOME = "C:\codedev\llm\.cache\huggingface"
```

## Build Process

### Current Build Command
```powershell
cargo build --release --package mistralrs-server --features "cuda flash-attn cudnn mkl"
```

### Build Features
- **cuda**: CUDA 12.9 support with RTX 5060 Ti
- **flash-attn**: Flash Attention V2/V3 for fast inference
- **cudnn**: cuDNN 9.8 acceleration
- **mkl**: Intel MKL BLAS backend

### Expected Artifacts
- `target\release\mistralrs-server.exe` - Main server binary

### Build Status
- ✓ Environment configured
- ✓ NVCC compiler linked
- ⏳ Compilation in progress
- ⏸ Estimated completion: 30-60 minutes (first build)

## Model Configuration

### Gemma 3 Model Options

#### Option 1: Download from Hugging Face (Recommended)
```powershell
# Run the download script
.\download-gemma3.ps1

# Or manually with huggingface-cli
huggingface-cli download google/gemma-3-4b-it `
  --local-dir C:\codedev\llm\.models\gemma-3-4b-it-hf
```

**Model Details:**
- Model ID: `google/gemma-3-4b-it`
- Format: Safetensors (Hugging Face format)
- Size: ~8GB download
- Target: `C:\codedev\llm\.models\gemma-3-4b-it-hf\`

#### Option 2: Existing Local Model (Incompatible)
- **Location**: `C:\codedev\llm\.models\gemma-3-gemmaCpp-3.0-4b-it-sfp-v1\`
- **Format**: `.sbs` (gemma.cpp custom format)
- **Status**: ⚠️ **NOT COMPATIBLE** with mistral.rs
- **Note**: The `.sbs` format is gemma.cpp-specific. Conversion tools are not publicly available.

## MCP (Model Context Protocol) Integration

### Available MCP Servers

From `C:\Users\david\mcp.json`:

1. **memory** - Session state management
   - Command: `bun x @modelcontextprotocol/server-memory@2025.8.4`
   
2. **filesystem** - File operations
   - Command: `bun x @modelcontextprotocol/server-filesystem@2025.8.21`
   
3. **sequential-thinking** - Multi-step reasoning
   - Command: `bun x @modelcontextprotocol/server-sequential-thinking@2025.7.1`
   
4. **github** - Repository metadata
   - Command: `bun x @modelcontextprotocol/server-github@2025.4.8`
   - Requires: `GITHUB_PERSONAL_ACCESS_TOKEN`
   
5. **fetch** - HTTP requests
   - Command: `bun x @modelcontextprotocol/server-fetch@0.6.3`
   
6. **time** - Time/date utilities
   - Command: `bun x @modelcontextprotocol/server-time@0.2.2`
   
7. **serena-claude** - Custom MCP wrapper
   - Command: `uv run python T:\projects\mcp_servers\serena\scripts\mcp_server.py`
   
8. **python-fileops-enhanced (desktop-commander)** - Advanced file operations
   - Command: `uv --directory C:\Users\david\.claude\python_fileops run python -m desktop_commander.mcp_server`
   
9. **rag-redis** - RAG with Redis backend
   - Command: `C:\users\david\bin\rag-redis-mcp-server.exe`
   - Requires: Redis running on `127.0.0.1:6379`

### MCP Configuration for mistral.rs

Create `mcp-config.json` in the format expected by mistral.rs:

```json
{
  "servers": [
    {
      "name": "Filesystem Tools",
      "source": {
        "type": "Process",
        "command": "bun",
        "args": ["x", "@modelcontextprotocol/server-filesystem@2025.8.21", "C:/codedev"]
      }
    },
    {
      "name": "Memory",
      "source": {
        "type": "Process",
        "command": "bun",
        "args": ["x", "@modelcontextprotocol/server-memory@2025.8.4"]
      }
    },
    {
      "name": "Sequential Thinking",
      "source": {
        "type": "Process",
        "command": "bun",
        "args": ["x", "@modelcontextprotocol/server-sequential-thinking@2025.7.1"]
      }
    }
  ],
  "auto_register_tools": true,
  "tool_timeout_secs": 180,
  "max_concurrent_calls": 1
}
```

## Running the Agent

### Interactive Mode (TUI)

Once the build completes:

```powershell
# Set environment variables (if not already set)
.\setup-dev-env.ps1

# Launch interactive mode
.\target\release\mistralrs-server.exe -i `
  --isq Q4_K `
  plain `
  -m "C:\codedev\llm\.models\gemma-3-4b-it-hf" `
  -a gemma3
```

### With MCP Integration

```powershell
.\target\release\mistralrs-server.exe -i `
  --mcp-config mcp-config.json `
  --isq Q4_K `
  plain `
  -m "C:\codedev\llm\.models\gemma-3-4b-it-hf" `
  -a gemma3
```

### HTTP Server Mode

```powershell
.\target\release\mistralrs-server.exe `
  --port 11434 `
  --mcp-config mcp-config.json `
  plain `
  -m "C:\codedev\llm\.models\gemma-3-4b-it-hf" `
  -a gemma3
```

## Inference Optimization for Coding Tasks

### Recommended Settings
- **Temperature**: 0.1-0.3 (deterministic code generation)
- **top_p**: 0.9
- **top_k**: 40
- **Quantization**: Q4_K_M or Q5_K_M for 4B model (good balance)
- **Context**: 8192 tokens (adjust based on VRAM)

### ISQ (In-Situ Quantization) Options
- `Q4_K`: Fastest, 4-bit quantization
- `Q5_K_M`: Balanced quality/speed
- `Q8_0`: Highest quality, slower

## Helper Scripts Created

### 1. `setup-dev-env.ps1`
Configures environment variables for CUDA, cuDNN, MKL, and Python.

**Usage:**
```powershell
.\setup-dev-env.ps1
```

### 2. `download-gemma3.ps1`
Downloads Gemma 3 4B model from Hugging Face.

**Usage:**
```powershell
.\download-gemma3.ps1
```

**Options:**
```powershell
# Custom model
.\download-gemma3.ps1 -ModelId "google/gemma-3-1b-it"

# Custom location
.\download-gemma3.ps1 -LocalDir "D:\models\gemma-3-4b"

# Force re-download
.\download-gemma3.ps1 -Force
```

## Symlinks Created

### Python
- `C:\users\david\.local\bin\python.exe` → Latest Python via UV
- `C:\users\david\.local\bin\python3.exe` → Latest Python via UV

### Visual Studio Tools
- `C:\users\david\.local\bin\vswhere.exe` → VS Installer tool

## Project Structure

```
T:\projects\rust-mistral\mistral.rs\
├── target\release\
│   └── mistralrs-server.exe      # Main binary (after build)
├── setup-dev-env.ps1               # Environment setup script
├── download-gemma3.ps1             # Model download script
├── mcp-config.json                 # MCP configuration
├── BUILD_STATUS.md                 # Build status and requirements
├── SETUP_COMPLETE.md               # This file
└── build.log                       # Build output log
```

## Next Steps

### Immediate (Automated)
1. ⏳ **Wait for build to complete** (~30-60 min)
2. ✅ **Verify binary**: `.\target\release\mistralrs-server.exe --help`

### Manual Steps Required
3. **Download Gemma 3 model**:
   ```powershell
   .\download-gemma3.ps1
   ```

4. **Create MCP config** (see MCP Configuration section above)

5. **Test interactive mode**:
   ```powershell
   .\target\release\mistralrs-server.exe -i `
     plain -m "C:\codedev\llm\.models\gemma-3-4b-it-hf" -a gemma3
   ```

6. **Test with MCP integration**:
   ```powershell
   .\target\release\mistralrs-server.exe -i `
     --mcp-config mcp-config.json `
     plain -m "C:\codedev\llm\.models\gemma-3-4b-it-hf" -a gemma3
   ```

## Troubleshooting

### Build Issues

**NVCC compiler not found:**
```powershell
# Run setup script again
.\setup-dev-env.ps1
```

**Python not found error:**
- Build only the server package: `cargo build --release --package mistralrs-server ...`
- Python bindings are not needed for the server binary

**cuDNN DLLs not found at runtime:**
```powershell
# Add cuDNN bin to PATH
$env:PATH = "C:\Program Files\NVIDIA\CUDNN\v9.8\bin;$env:PATH"
```

### Model Issues

**Model not found:**
- Ensure Gemma 3 is downloaded in Hugging Face format
- Check path: `C:\codedev\llm\.models\gemma-3-4b-it-hf\`

**Out of memory:**
- Use smaller quantization: `--isq Q4_K`
- Reduce context length
- Use 2B model instead of 4B

### MCP Issues

**MCP servers not connecting:**
- Verify `bun` is installed: `bun --version`
- Check Redis is running (for rag-redis): `redis-cli ping`
- Test individual servers manually

## Performance Expectations

### Gemma 3 4B on RTX 5060 Ti (16GB)

| Quantization | VRAM Usage | Tokens/sec | Quality |
|--------------|------------|------------|---------|
| Q4_K         | ~3-4GB     | 40-60      | Good    |
| Q5_K_M       | ~4-5GB     | 35-50      | Better  |
| Q8_0         | ~6-8GB     | 25-40      | Best    |
| FP16         | ~8-10GB    | 20-35      | Highest |

*Actual performance depends on batch size, context length, and system load*

## References

- [mistral.rs README](README.md)
- [mistral.rs AGENTS.md](AGENTS.md)
- [MCP Quick Start](examples/MCP_QUICK_START.md)
- [MCP Documentation](docs/MCP/README.md)
- [Build Status](BUILD_STATUS.md)

## Credits & Compliance

- **PATH Preservation**: ✓ `C:\Users\david\.local\bin` maintained throughout
- **Canonical File Editing**: ✓ All files edited in place, no duplicate variants created
- **MCP Configuration**: Single canonical source at `C:\Users\david\mcp.json`

---

**Last Updated**: 2025-10-02 21:47 UTC  
**Status**: Environment ready, build in progress, model download pending
