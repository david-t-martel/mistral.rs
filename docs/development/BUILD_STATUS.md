# Mistral.rs Build Status & Plan

## Current Status (2025-10-02)

### Environment Verified ✓
- **Location**: `T:\projects\rust-mistral\mistral.rs`
- **CUDA**: v12.9 available (`C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9`)
  - Additional versions: v12.1, v12.6, v12.8, v13.0
- **GPU**: NVIDIA GeForce RTX 5060 Ti (16GB VRAM, Driver 576.88)
- **Intel oneAPI**: MKL available (`C:\Program Files (x86)\Intel\oneAPI\mkl`)
- **Rust**: 1.89.0 (MSVC toolchain)
- **Cargo**: 1.89.0
- **Tools**: git, cmake, nvcc all available
- **PATH invariant**: ✓ `C:\Users\david\.local\bin` is on PATH

###  Build Failures Encountered

#### 1. Python Not Found
- **Issue**: PyO3 bindings require Python 3.x
- **Solution**: Build only `mistralrs-server` package (not the whole workspace)
- **Status**: BYPASSED ✓

#### 2. NVCC Host Compiler Issue  
- **Error**: `nvcc fatal: Failed to preprocess host compiler properties`
- **Root Cause**: NVCC cannot find/use MSVC compiler
- **Required Fix**: Set `NVCC_CCBIN` environment variable to point to Visual Studio's `cl.exe`

## Required Prerequisites

### 1. Visual Studio 2022 Build Tools
Must have these components installed:
- MSVC v143 - VS 2022 C++ x64/x86 build tools
- Windows 11 SDK (10.0.22621.0 or later)
- C++ CMake tools for Windows
- C++ ATL for latest v143 build tools (x86 & x64)

### 2. Python 3.11+ (if building Python bindings)
- Not strictly required for `mistralrs-server` binary only
- Recommended: `python` and `python3` on PATH

### 3. cuDNN 9.x for CUDA 12.x
- Location: `C:\Program Files\NVIDIA\CUDNN` (✓ exists)
- Ensure `bin\`, `lib\`, `include\` subdirectories exist

## Build Command (Corrected)

### Step 1: Set Visual Studio Environment
```powershell
# Find Visual Studio installation
$vsPath = &"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe" `
  -latest -products * `
  -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
  -property installationPath

# Set NVCC_CCBIN to MSVC compiler
$env:NVCC_CCBIN = "$vsPath\VC\Tools\MSVC\<version>\bin\Hostx64\x64\cl.exe"

# Or use vcvarsall.bat (recommended)
& "$vsPath\VC\Auxiliary\Build\vcvarsall.bat" x64
```

### Step 2: Build with Correct Features
```powershell
cd T:\projects\rust-mistral\mistral.rs

# Build mistralrs-server with CUDA, Flash Attention, cuDNN, and MKL
cargo build --release `
  --package mistralrs-server `
  --features "cuda flash-attn cudnn mkl"
```

### Expected Output
- Binary: `target\release\mistralrs-server.exe`
- Build time: ~30-60 minutes (first build)
- VRAM requirement: ~6-8GB for Gemma 3 4B model

## Model Configuration

### Local Gemma 3 Model
- **Path**: `C:\codedev\llm\.models\gemma-3-gemmaCpp-3.0-4b-it-sfp-v1\`
- **Files**:
  - `4b-it-sfp.sbs` (5.4GB) - Model weights in custom format
  - `tokenizer.spm` (4.7MB) - SentencePiece tokenizer
  - `model.tar.gz` (4.2GB) - Archive

**Note**: The `.sbs` format is gemma.cpp specific. For mistral.rs, we need either:
1. GGUF format (if supported for Gemma 3)
2. Hugging Face safetensors format

### Alternative: Download Gemma 3 in Compatible Format
```powershell
# Using Hugging Face CLI (if installed)
huggingface-cli download google/gemma-3-4b-it `
  --local-dir C:\codedev\llm\.models\gemma-3-4b-it-hf
```

## MCP Integration Plan

### MCP Servers Available (from `C:\Users\david\mcp.json`)
1. **memory** - Session state management
2. **filesystem** - File operations  
3. **sequential-thinking** - Multi-step reasoning
4. **github** - Repository metadata
5. **fetch** - HTTP requests
6. **time** - Time/date utilities
7. **serena-claude** - Custom MCP wrapper
8. **python-fileops-enhanced/desktop-commander** - Advanced file ops
9. **rag-redis** - RAG with Redis backend

### MCP Configuration Format
Mistral.rs expects MCP config in this format (see `examples/MCP_QUICK_START.md`):
```json
{
  "servers": [{
    "name": "Filesystem Tools",
    "source": {
      "type": "Process",
      "command": "npx",
      "args": ["@modelcontextprotocol/server-filesystem", "."]
    }
  }],
  "auto_register_tools": true
}
```

## Next Steps

1. **Fix NVCC_CCBIN** - Point to Visual Studio's cl.exe
2. **Retry Build** - With corrected environment
3. **Verify Binary** - Test `mistralrs-server.exe --help`
4. **Convert/Download Model** - Get Gemma 3 in compatible format
5. **Create MCP Config** - Merge `mcp.json` servers into mistral.rs format
6. **Test Interactive Mode** - Launch with `cargo run --release --features cuda,flash-attn,cudnn,mkl -- -i ...`
7. **Document Setup** - Create `llms.txt` and `MISTRAL_AGENT_SETUP.md`

## Build Logs
- Location: `T:\projects\rust-mistral\mistral.rs\build.log`
- Contains: Full cargo output with error details

## References
- [mistral.rs README](README.md)
- [AGENTS.md](AGENTS.md) - AI agent guidelines
- [MCP Quick Start](examples/MCP_QUICK_START.md)
- [Building Guide](docs/README.md)
