# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

mistral.rs is a blazing-fast LLM inference engine written in Rust. It supports text, vision, image generation, and speech models with multiple APIs (Rust, Python, OpenAI HTTP, MCP).

**Current Version**: 0.6.0
**Rust Version Required**: 1.86+

## 🚨 CRITICAL: Use Makefile, NOT Cargo Directly

**This project has a comprehensive Makefile that MUST be used for all builds.**

### Why Use the Makefile?

❌ **DON'T** run `cargo build` directly
✅ **DO** run `make build` instead

The Makefile handles:
- Automatic platform detection (Windows/Linux/macOS)
- CUDA environment setup (NVCC_CCBIN, paths)
- Correct feature flag combinations
- Build caching with sccache
- Error logging and validation

See `.claude/CLAUDE.md` for complete Makefile documentation.

## Essential Commands (via Makefile)

### Quick Reference

```bash
make help          # Show all available targets
make check         # Quick compilation check
make dev           # Development build (debug)
make build         # Release build (CPU only)
make test          # Run tests
make fmt           # Format all code
make ci            # Full CI pipeline
```

### Building

**Windows with CUDA** (most common for this project):
```bash
# Full CUDA build with all features
make build-cuda-full

# Output: target\release\mistralrs-server.exe
```

**Linux with CUDA**:
```bash
make build-cuda-full
```

**macOS with Metal**:
```bash
make build-metal
```

**Basic CPU build** (any platform):
```bash
make build
```

### Why This Replaces Direct Cargo Commands

| Old Way (❌ Don't Use) | New Way (✅ Use This) | Reason |
|------------------------|----------------------|---------|
| `cargo build --release` | `make build` | Missing feature flags |
| `cargo build --release --features cuda,flash-attn,cudnn,mkl` | `make build-cuda-full` | Env vars not set, long command |
| `cargo check` | `make check` | No logging, no validation |
| `cargo test` | `make test` | No test isolation |
| `cargo fmt` | `make fmt` | Only formats Rust (not Python/C/CUDA) |

### Testing & Quality

```bash
# Run all tests
make test

# Test specific packages
make test-core        # Core inference engine
make test-server      # Server components
make test-quant       # Quantization
make test-vision      # Vision models

# Quick compilation check (ALWAYS run before committing)
make check

# Format all code (Rust + Python + C/CUDA)
make fmt

# Verify formatting
make fmt-check

# Run clippy lints
make lint

# Auto-fix linting issues
make lint-fix

# Full CI pipeline (check + lint + test + format)
make ci
```

### Running Models

**Via Makefile** (recommended):
```bash
# Run TUI with smallest test model (auto-selected from MODEL_INVENTORY.json)
make run-tui

# Run HTTP server on port 8080
make run-server MODEL=/path/to/model

# Run with MCP integration
make run-with-mcp MODEL_DIR=/path MODEL_FILE=model.gguf
```

**Direct binary usage** (after building with `make build-cuda-full`):
```bash
# Interactive TUI mode
./target/release/mistralrs-server -i gguf -m /path/to/model -f model.gguf

# With Hugging Face model
./target/release/mistralrs-server -i plain -m meta-llama/Llama-3.2-3B-Instruct

# HTTP Server mode
./target/release/mistralrs-server --port 1234 gguf -m /path -f model.gguf

# With MCP integration
./target/release/mistralrs-server --port 1234 --mcp-config mcp-config.json gguf -m /path -f model.gguf
```

**Windows PowerShell launch scripts** (project-specific):
```powershell
# Quick test with smallest model (Qwen 1.5B)
.\launch-qwen-fast.ps1

# Load Gemma 2 2B model
.\launch-gemma2.ps1

# General-purpose launcher
.\start-mistralrs.ps1
```

## Models

When integrating a new model, make sure it respects all of the varbuilder `.pp` calls. In Candle, a VarBuilder maintains an internal path vector that acts like a “current working directory” for model weights; every call to pp("sub") (alias for push_prefix) clones the builder and appends sub, so successive calls accumulate a dotted prefix such as transformer.h.0 while leaving the original builder untouched . When you eventually call get(...), Candle joins that prefix with the tensor name (prefix + "." + name) and looks it up in the checkpoint backend, producing keys that exactly match the dot-separated names emitted by PyTorch’s state_dict/named_parameters, which means PyTorch-trained weights can be loaded without any renaming  ￼. This lets you recreate the PyTorch module tree in Rust by “walking” it: e.g. vb.pp("word_embeddings") grabs word_embeddings.*, while a chain like vb.pp("encoder").pp("layers").pp(i.to_string()) targets keys such as encoder.layers.0.*, exactly as shown in community tutorials porting Transformers models to Candle  ￼. As one maintainer put it, the prefix system lets you “cd” around the parameter hierarchy, giving a lightweight namespace mechanism that keeps Candle fully compatible with PyTorch naming conventions while remaining ergonomic to use.

You should also look for a model.safetensors.index.json file for the model at hand to verify correct structure.

## Architecture Overview

### Workspace Structure
- `mistralrs-core/` - Core inference engine, model implementations, pipelines
- `mistralrs-server/` - CLI binary entry point
- `mistralrs-server-core/` - HTTP server routing, OpenAI API implementation
- `mistralrs-pyo3/` - Python bindings (PyO3)
- `mistralrs/` - High-level Rust API
- `mistralrs-vision/` - Vision model support
- `mistralrs-quant/` - Quantization implementations (ISQ, GGUF, GPTQ, etc.)
- `mistralrs-paged-attn/` - PagedAttention implementation
- `mistralrs-audio/` - Audio processing
- `mistralrs-mcp/` - Model Context Protocol client
- `mistralrs-bench/` - Benchmarking tools

### Key Design Patterns

1. **Pipeline Architecture**: All models implement the `Pipeline` trait in `mistralrs-core/src/pipeline/mod.rs`. Different model types (Plain, GGUF, GGML, Vision) have their own pipeline implementations.

2. **Model Loading**: Models are loaded through `Loader` traits that handle different formats and quantizations. See `mistralrs-core/src/loader.rs`.

3. **Request Handling**: The server uses message passing with `MistralRs` struct managing a background thread pool. Requests flow through `mistralrs-core/src/engine/mod.rs`.

4. **Device Management**: Automatic and manual device mapping for multi-GPU setups handled in `mistralrs-core/src/device_map.rs`.

### Adding New Features

When adding new model architectures:
1. Implement the model in `mistralrs-core/src/models/`
2. Add pipeline support in `mistralrs-core/src/pipeline/`
3. Update model detection in `mistralrs-core/src/pipeline/normal.rs`
4. Add architecture enum variant in `mistralrs-core/src/lib.rs`
5. Update CLI args in `mistralrs-server/src/main.rs`

When adding new quantization methods:
1. Implement in `mistralrs-quant/src/`
2. Add to quantization loading logic in pipelines
3. Update documentation in `docs/QUANTIZATION.md`

### Important Files to Know

- `mistralrs-core/src/engine/mod.rs` - Main engine orchestration
- `mistralrs-core/src/pipeline/mod.rs` - Pipeline trait and common logic
- `mistralrs-server-core/src/routes.rs` - HTTP API endpoints
- `mistralrs-pyo3/src/lib.rs` - Python API entry point
- `mistralrs/examples/` - Usage examples for Rust API

### Testing Approach

You should **always** run `cargo check` before returning to make sure code compiles. If code does not compile, only make edits.

Avoid returning TODOs.

- Unit tests are colocated with source files
- Integration tests in `tests/` directories
- Use `cargo test -p <crate>` to test specific components
- Python tests require building and installing the package first
- **Project-specific**: Use `MODEL_INVENTORY.json` to identify available test models
- **TUI Testing**: Smallest model (Qwen2.5-1.5B-Instruct-Q4_K_M, 940MB) recommended for quick testing

### Development Workflow

1. **Before making changes**:
   ```bash
   cargo check  # Verify current state compiles
   ```

2. **During development**:
   ```bash
   cargo clippy --fix  # Auto-fix linting issues
   make fmt            # Format all code
   ```

3. **After changes**:
   ```bash
   cargo check         # Verify changes compile
   cargo test -p <affected-crate>  # Run relevant tests
   cargo clippy        # Check for issues
   ```

4. **For new models**:
   - Add to `mistralrs-core/src/models/`
   - Update pipeline in `mistralrs-core/src/pipeline/`
   - Add architecture to CLI in `mistralrs-server/src/main.rs`
   - Test with smallest GGUF model first

### MCP Integration

This project includes extensive MCP (Model Context Protocol) support:

**MCP Client** (connect to external tools):
```bash
# Create mcp-config.json
{
  "servers": [{
    "name": "Filesystem",
    "source": {
      "type": "Process",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
    }
  }],
  "auto_register_tools": true
}

# Launch with MCP tools
./mistralrs-server --port 1234 --mcp-config mcp-config.json gguf -m /path -f model.gguf
```

**MCP Server** (serve mistral.rs via MCP):
```bash
# Parallel to HTTP API on port 4321
./mistralrs-server --mcp-port 4321 plain -m Qwen/Qwen3-4B
```

**Available MCP servers** (from project's MCP_CONFIG.json):
- memory - Session state management
- filesystem - File operations
- sequential-thinking - Multi-step reasoning
- github - Repository operations
- fetch - HTTP requests
- time - Time/date utilities
- rag-redis - RAG with Redis backend (requires Redis running)

### Common Pitfalls

1. **Feature Flags**: Many features are gated behind Cargo features. Always check what features are needed for your use case.
2. **Device Indices**: CUDA device selection uses 0-based indexing
3. **Chat Templates**: Models may need specific chat templates - check `chat_templates/` directory
4. **Quantization**: Different quantization methods have different hardware requirements
5. **Windows NVCC**: Must set `NVCC_CCBIN` environment variable to MSVC compiler path
6. **PyO3 Bindings**: Require Python 3.x; use `--package mistralrs-server` to build server only
7. **Model Formats**:
   - Use `run` subcommand for text/vision models only
   - Use `diffusion` subcommand for image generation (FLUX, etc.)
   - Use `speech` subcommand for audio generation (Dia, etc.)
8. **MCP stdio protocol**: MCP servers communicate via JSON-RPC over stdin/stdout, not HTTP

### Project-Specific Notes

**Environment**:
- GPU: NVIDIA GeForce RTX 5060 Ti (16GB VRAM)
- CUDA: 12.9 (with additional versions: 12.1, 12.6, 12.8, 13.0)
- cuDNN: 9.8
- Platform: Windows 11 with PowerShell
- Build tools: Visual Studio 2022, Rust 1.89.0

**Current Development State**:
- Binary builds successfully to `target\release\mistralrs-server.exe`
- Phase 1 testing complete (infrastructure validation)
- Phase 2 MCP testing: 2/9 servers validated (Time, RAG-Redis)
- TUI and HTTP server testing in progress
- PyO3 bindings: Not yet built (optional)

**Available Models** (see `MODEL_INVENTORY.json`):
- Qwen2.5-1.5B-Instruct-Q4_K_M (940MB) - Fastest, use for testing
- Gemma 2 2B-it-Q4_K_M (1.67GB)
- Qwen2.5-Coder-3B-Instruct-Q4_K_M (1.93GB)
- Qwen2.5-7B-Instruct-Q4_K_M (4.37GB)
- Gemma 3 4B-it-hf (8.5GB, safetensors)

**Testing Scripts Available**:
- `test-mcp-servers.ps1` - Validate MCP server configurations
- `test-phase2-mcp-servers.ps1` - Detailed MCP server testing
- `launch-*.ps1` - Quick model launch scripts
- `start-mistralrs.ps1` - General-purpose server launcher