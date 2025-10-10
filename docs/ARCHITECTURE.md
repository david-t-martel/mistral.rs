# Architecture Documentation

**Project**: mistral.rs\
**Date**: 2025-10-09\
**Version**: 0.6.0

______________________________________________________________________

## Table of Contents

1. [System Overview](#system-overview)
1. [Component Architecture](#component-architecture)
1. [Data Flow](#data-flow)
1. [Integration Points](#integration-points)
1. [Feature Flags](#feature-flags)
1. [Build System](#build-system)
1. [Deployment Models](#deployment-models)

______________________________________________________________________

## 1. System Overview

mistral.rs is a high-performance, GPU-accelerated inference engine for large language models with multimodal capabilities (text, vision, diffusion, speech). The architecture is modular with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────┐
│                     User Interfaces                          │
├───────────────┬─────────────────┬──────────────────────────┤
│ mistralrs-tui │ mistralrs-server│ mistralrs-pyo3 (Python)  │
│  (Terminal UI)│  (HTTP Server)  │  (Python Bindings)       │
└───────┬───────┴────────┬────────┴──────────┬───────────────┘
        │                │                    │
        └────────────────┼────────────────────┘
                         │
        ┌────────────────▼────────────────┐
        │      mistralrs-core             │
        │  (Inference Engine & Pipeline)  │
        └────────┬────────────────┬───────┘
                 │                │
    ┌────────────▼───┐   ┌───────▼─────────────┐
    │ mistralrs-mcp  │   │ mistralrs-agent     │
    │ (Tool Protocol)│   │    -tools           │
    └────────────────┘   └─────────────────────┘
                 │                │
        ┌────────▼────────────────▼───────┐
        │     mistralrs-quant             │
        │  (Quantization & CUDA Kernels)  │
        └─────────────────────────────────┘
```

### Core Principles

1. **Zero-Copy Where Possible**: Memory-mapped tensors via safetensors
1. **Async-First**: Tokio runtime for concurrency
1. **Type Safety**: Strong Rust type system with minimal `unsafe`
1. **Modular Quantization**: Pluggable quant backends (GGUF, GPTQ, AWQ, HQQ, FP8)
1. **Tool Extensibility**: MCP protocol for external tool integration

______________________________________________________________________

## 2. Component Architecture

### 2.1 mistralrs-core

**Purpose**: Core inference engine, model loading, pipeline orchestration

**Key Responsibilities**:

- Model loading from HuggingFace Hub or local paths
- Pipeline dispatch (text, vision, diffusion, speech, speculative)
- Token sampling with temperature/top-k/top-p
- KV cache management (paged attention optional)
- Tool callback orchestration

**Public API**:

```rust
pub struct MistralRs { ... }

impl MistralRs {
    pub async fn new(config: MistralRsBuilder) -> Arc<Self>
    pub async fn send_request(&self, request: Request) -> Receiver<Response>
    pub fn config(&self) -> ModelConfig
}
```

**Dependencies**:

- `candle-core`: Tensor operations
- `hf-hub`: HuggingFace model downloads
- `tokenizers`: Fast BPE/SentencePiece tokenization
- `mistralrs-quant`: Quantized ops
- `mistralrs-mcp`: Tool protocol (optional)

______________________________________________________________________

### 2.2 mistralrs-server / mistralrs-server-core

**Purpose**: OpenAI-compatible HTTP API server

**Key Responsibilities**:

- HTTP endpoints (`/v1/chat/completions`, `/v1/completions`, `/v1/models`)
- SSE streaming for chunk responses
- Request validation and rate limiting
- Health checks and metrics export (Prometheus)

**Architecture**:

```
┌─────────────────────────────────────┐
│   mistralrs-server (binary)         │
│  - CLI parsing                      │
│  - Model builder configuration      │
│  - Tokio runtime setup              │
└───────────┬─────────────────────────┘
            │
┌───────────▼─────────────────────────┐
│  mistralrs-server-core (library)    │
│  - Axum routes                      │
│  - OpenAPI documentation            │
│  - Request/response types           │
└───────────┬─────────────────────────┘
            │
┌───────────▼─────────────────────────┐
│   mistralrs-core                    │
│  - Actual inference                 │
└─────────────────────────────────────┘
```

**Deployment**:

- Standalone binary: `mistralrs-server run <model>`
- Library integration: Embed in custom Axum/actix servers
- Docker: Multi-stage build with CUDA support

______________________________________________________________________

### 2.3 mistralrs-tui

**Purpose**: GPU-accelerated terminal user interface

**Key Responsibilities**:

- Session management (SQLite persistence)
- Model discovery from filesystem
- Interactive chat with streaming responses
- Optional agent tools integration (`tui-agent` feature)

**Architecture**:

```
┌────────────────────────────────────┐
│  Rendering Backend                 │
│  - GPU: ratatui + wgpu + winit     │
│  - CPU: ratatui + crossterm        │
└─────────────┬──────────────────────┘
              │
┌─────────────▼──────────────────────┐
│  App State                         │
│  - Active session                  │
│  - Model inventory                 │
│  - Agent toolkit (optional)        │
└─────────────┬──────────────────────┘
              │
┌─────────────▼──────────────────────┐
│  Persistence Layer                 │
│  - SessionStore (SQLite + sqlx)    │
│  - Model cache                     │
└────────────────────────────────────┘
```

**Integration**: TUI is a **frontend** only. It:

- Does NOT directly call mistralrs-core for inference
- DOES manage sessions/models/configuration
- CAN call mistralrs-server HTTP API (future work)

______________________________________________________________________

### 2.4 mistralrs-mcp

**Purpose**: Model Context Protocol client for external tool integration

**Key Responsibilities**:

- Connect to MCP servers (HTTP, WebSocket, stdio process)
- Discover tools from servers
- Execute tool calls with timeout/retry
- Provide `ToolCallback` wrappers for mistralrs-core

**Supported Transports**:

1. **HTTP**: REST API with bearer token auth
1. **WebSocket**: Bidirectional streaming
1. **Process**: Spawn subprocess and communicate via stdin/stdout

**Example Configuration**:

```rust
let config = McpClientConfig {
    servers: vec![
        McpServerConfig {
            id: "filesystem".to_string(),
            source: McpServerSource::Process {
                command: "npx".to_string(),
                args: vec!["-y", "@modelcontextprotocol/server-filesystem"],
                env: Some(HashMap::from([
                    ("ALLOWED_DIRS", "/tmp/sandbox"),
                ])),
            },
            tool_prefix: Some("fs".to_string()),
            enabled: true,
        },
    ],
    auto_register_tools: true,
};
```

______________________________________________________________________

### 2.5 mistralrs-agent-tools

**Purpose**: Sandboxed filesystem and shell utilities for autonomous agents

**Key Responsibilities**:

- 90+ Unix/Windows utilities (cat, ls, grep, wc, head, tail, sort, etc.)
- Sandbox enforcement (path traversal prevention, read-only paths)
- Cross-platform shell execution (PowerShell, cmd, bash)
- AgentToolProvider for mistralrs-core integration

**Security Model**:

```rust
pub struct SandboxConfig {
    pub root: PathBuf,              // All paths must be under this
    pub readonly_paths: Vec<PathBuf>,
    pub max_file_size: Option<u64>, // Prevent reading huge files
    pub allowed_exts: Option<Vec<String>>,
}
```

**Integration Status** (as of 2025-10-09):

- ✅ Core implementation complete (115 tests passing)
- ⚠️ Partially integrated: Only 8/90 tools exposed via `AgentToolProvider`
- 🔴 Not wired to server: Missing `--enable-agent-tools` CLI flag

______________________________________________________________________

### 2.6 mistralrs-quant

**Purpose**: Quantization backends and CUDA kernels

**Supported Formats**:

- **GGUF**: LLaMA.cpp format (Q4_K_M, Q5_K_M, Q8_0, etc.)
- **GPTQ**: GPU-optimized 4-bit (Marlin kernel, ExLlamaV2 backend)
- **AWQ**: Activation-aware weight quantization
- **HQQ**: Half-quadratic quantization
- **FP8**: NVIDIA H100 native FP8
- **ISQ**: In-situ quantization (post-load quantization)

**CUDA Integration**:

- cuBLASLt for FP8 matmul
- Custom kernels for GPTQ/AWQ unpacking
- NCCL for multi-GPU tensor parallelism

**Unsafe Code**: 80+ `unsafe` blocks (see [SAFETY.md](./SAFETY.md))

______________________________________________________________________

### 2.7 mistralrs-vision / mistralrs-audio

**Purpose**: Modality-specific processors

- **mistralrs-vision**: Image encoding (CLIP, SigLIP, VIT)
- **mistralrs-audio**: Speech processing (Whisper encoder)

______________________________________________________________________

## 3. Data Flow

### 3.1 Chat Completion Request Flow

```
User Request (HTTP POST /v1/chat/completions)
│
▼
mistralrs-server-core::routes::chat_completions()
│
├─ Validate request (OpenAI schema)
├─ Extract tools from request
├─ Create NormalRequest
│
▼
mistralrs-core::MistralRs::send_request()
│
├─ Enqueue to pipeline tx channel
├─ Return response rx channel
│
▼
Pipeline Worker (tokio::spawn)
│
├─ Load model if not cached
├─ Tokenize prompt
├─ Forward pass (prefill + decode loop)
│  │
│  ├─ KV cache update
│  ├─ Sampling (temperature, top-k, top-p)
│  ├─ Tool call detection
│  │  │
│  │  └─ If tool call: Execute via ToolCallback
│  │     │
│  │     ├─ MCP tool: HTTP/WS call to external server
│  │     ├─ Agent tool: Sandboxed filesystem operation
│  │     └─ Return result
│  │
│  └─ Yield chunk via response channel
│
▼
mistralrs-server-core (SSE stream)
│
▼
User (receives streamed chunks)
```

### 3.2 Tool Execution Flow

```
Model generates tool call:
{
  "name": "fs_read_file",
  "arguments": "{\"path\": \"/tmp/data.txt\"}"
}
│
▼
mistralrs-core::engine::handle_tool_calls()
│
├─ Lookup tool in tool_callbacks_with_tools HashMap
├─ Deserialize arguments JSON
│
▼
ToolCallback::call(called_function)
│
├─ For MCP tools:
│  └─ McpClient::call_tool() → HTTP request to MCP server
│
├─ For Agent tools:
│  └─ AgentToolProvider::execute() → Sandbox operation
│
▼
Tool result appended to conversation:
{
  "role": "tool",
  "name": "fs_read_file",
  "content": "<file contents>"
}
│
▼
Model continues generation with tool results
```

______________________________________________________________________

## 4. Integration Points

### 4.1 mistralrs-core ↔ mistralrs-mcp

**Integration**: Fully functional

```rust
// In mistralrs-core/src/lib.rs
let mut mcp_client = McpClient::new(config);
mcp_client.initialize().await?;
let mcp_callbacks = mcp_client.get_tool_callbacks_with_tools();

for (name, callback_with_tool) in mcp_callbacks {
    tool_callbacks_with_tools.insert(name, callback_with_tool);
}
```

**Status**: ✅ Production-ready

______________________________________________________________________

### 4.2 mistralrs-core ↔ mistralrs-agent-tools

**Integration**: Partial (8/90 tools)

**Current**:

```rust
// mistralrs-agent-tools/src/core_integration.rs
pub struct AgentToolProvider {
    toolkit: AgentToolkit,
}

impl AgentToolProvider {
    pub fn get_tool_callbacks_with_tools(&self) -> HashMap<String, ToolCallbackWithTool> {
        // Returns 8 tools: cat, ls, grep, head, tail, wc, sort, uniq, shell
    }
}
```

**Missing**:

- CLI flags in mistralrs-server (`--enable-agent-tools`, `--agent-sandbox-root`)
- Remaining 82 tool definitions in core_integration.rs
- Registration in MistralRsBuilder

**TODO**:

```rust
// mistralrs-server/src/main.rs - ADD THIS
if args.enable_agent_tools {
    let agent_provider = AgentToolProvider::new(SandboxConfig {
        root: args.agent_sandbox_root,
        ..Default::default()
    });
    let agent_callbacks = agent_provider.get_tool_callbacks_with_tools();
    
    for (name, callback) in agent_callbacks {
        tool_callbacks_with_tools.insert(name, callback);
    }
}
```

**Status**: ⚠️ Needs completion (Priority 1)

______________________________________________________________________

### 4.3 mistralrs-tui ↔ mistralrs-core

**Integration**: Indirect (TUI is frontend only)

**Current Architecture**:

- TUI manages sessions and model inventory
- TUI does NOT perform inference directly
- Future: TUI will call mistralrs-server HTTP API

**Status**: ✅ Correct design (no direct integration needed)

______________________________________________________________________

### 4.4 mistralrs-server ↔ mistralrs-agent-tools

**Integration**: Broken (using deprecated API)

**Problem**: `mistralrs-server/src/agent_mode.rs` was using old `AgentTools` type (since fixed to `AgentToolkit`)

**Status**: ✅ Fixed in recent commits

______________________________________________________________________

## 5. Feature Flags

### 5.1 Workspace-Level Features

**In root `Cargo.toml`**:

```toml
[features]
default = []

# Accelerators
cuda = ["mistralrs-core/cuda"]
metal = ["mistralrs-core/metal"]
mkl = ["mistralrs-core/mkl"]

# Flash Attention
flash-attn = ["mistralrs-core/flash-attn"]
flash-attn-v1 = ["mistralrs-core/flash-attn-v1"]

# Quantization
gguf = ["mistralrs-core/gguf"]

# Python bindings
pyo3_macros = ["mistralrs-pyo3/pyo3_macros"]

# TUI
tui = ["dep:mistralrs-tui"]
tui-agent = ["mistralrs-tui/tui-agent"]
```

### 5.2 Conditional Compilation Patterns

**Good Practice**:

```rust
#[cfg(feature = "cuda")]
use cudarc::driver::CudaDevice;

#[cfg(feature = "cuda")]
pub fn use_cuda() -> Result<()> {
    // CUDA-specific code
}

#[cfg(not(feature = "cuda"))]
pub fn use_cuda() -> Result<()> {
    Err(anyhow!("CUDA support not compiled"))
}
```

**Anti-Pattern** (avoid):

```rust
// Don't hide critical APIs behind features
#[cfg(feature = "optional")]
pub struct ImportantType { ... }  // ❌ BAD
```

______________________________________________________________________

## 6. Build System

### 6.1 Makefile Targets

**Standard Development**:

```bash
make check      # Fast compile validation
make build      # CPU-only build
make test       # Run all tests
make fmt        # Format code
make lint       # Clippy + checks
```

**CUDA Builds**:

```bash
make build-cuda       # CUDA + cuBLAS
make build-cuda-full  # CUDA + flash-attn + cudnn
```

**Quality Gates**:

```bash
make ci               # Full CI pipeline (fmt + lint + test + check)
```

### 6.2 Docker Images

**Base Image** (`Dockerfile`):

- Ubuntu 22.04 + Rust 1.75+
- CPU-only or CUDA 12.6
- Compiled mistralrs-server binary

**Usage**:

```bash
docker build -t mistralrs .
docker run -p 8080:8080 mistralrs run llama
```

______________________________________________________________________

## 7. Deployment Models

### 7.1 Standalone Binary

**Use Case**: Single-model inference server

```bash
mistralrs-server run \
  --port 8080 \
  --model-id mistralai/Mistral-7B-Instruct-v0.1 \
  --token-source hf-token \
  --max-seqs 32
```

### 7.2 Library Integration

**Use Case**: Embed in custom Rust application

```rust
use mistralrs_core::MistralRsBuilder;
use mistralrs_server_core::MistralRsServerRouterBuilder;

#[tokio::main]
async fn main() {
    let mistralrs = MistralRsBuilder::new(...)
        .with_mcp_client(mcp_config)
        .build()
        .await;
    
    let app = MistralRsServerRouterBuilder::new()
        .with_mistralrs(Arc::new(mistralrs))
        .build()
        .await
        .unwrap();
    
    axum::serve(listener, app).await.unwrap();
}
```

### 7.3 Python Bindings

**Use Case**: Python ML workflows

```python
import mistralrs

runner = mistralrs.Runner(
    which=mistralrs.Which.GGUF(
        tok_model_id="mistralai/Mistral-7B-Instruct-v0.1",
        quantized_model_id="TheBloke/Mistral-7B-Instruct-v0.1-GGUF",
        quantized_filename="mistral-7b-instruct-v0.1.Q4_K_M.gguf",
    ),
    mcp_client_config=mcp_config
)

result = runner.send_chat_completion_request(...)
```

______________________________________________________________________

## 8. Performance Characteristics

### 8.1 Latency Profile

| Stage                       | Typical Time (Mistral-7B, GGUF Q4_K_M) |
| --------------------------- | -------------------------------------- |
| Model load                  | 2-5 seconds (cold start)               |
| Prompt prefill (512 tokens) | 100-200ms (CUDA)                       |
| Token decode                | 20-50ms/token (CUDA)                   |
| Tool call overhead          | 50-200ms (depends on tool)             |

### 8.2 Memory Usage

| Model        | Quantization | GPU VRAM | System RAM |
| ------------ | ------------ | -------- | ---------- |
| Mistral-7B   | Q4_K_M       | 4.5 GB   | 8 GB       |
| Mistral-7B   | FP16         | 14 GB    | 16 GB      |
| Mixtral-8x7B | Q4_K_M       | 26 GB    | 32 GB      |

### 8.3 Throughput Optimization

**Key Techniques**:

1. **PagedAttention**: Reduce KV cache fragmentation
1. **Flash Attention**: 2-4x speedup on prefill
1. **Continuous Batching**: Process multiple requests concurrently
1. **Speculative Decoding**: Draft model + target model parallelism

______________________________________________________________________

## 9. Future Architecture

### 9.1 Planned Enhancements

1. **Distributed Inference**: Multi-GPU tensor parallelism (NCCL in progress)
1. **Streaming Ingestion**: WebSocket for low-latency chat
1. **TUI → Server Integration**: HTTP client in TUI for remote inference
1. **Agent Orchestration**: Multi-agent conversations with tool delegation
1. **Structured Generation**: Grammar-constrained sampling

### 9.2 Research Directions

- **Quantization**: Explore AQLM, QuIP#
- **Attention**: Investigate Mamba/SSM alternatives
- **Multimodal**: Add audio generation (TTS)

______________________________________________________________________

## 10. Troubleshooting

### 10.1 Common Issues

**Problem**: "Model not found"\
**Solution**: Check `HF_TOKEN` env var, verify model ID

**Problem**: CUDA out of memory\
**Solution**: Use lower quant (Q4_K_M), reduce `--max-seqs`, enable PagedAttention

**Problem**: Tool calls not executing\
**Solution**: Verify MCP server is running, check `--mcp-config` path

### 10.2 Debug Logging

```bash
MISTRALRS_DEBUG=1 RUST_LOG=mistralrs=debug mistralrs-server run llama
```

______________________________________________________________________

## 11. Contributing

### 11.1 Adding a New Model

1. Implement model in `mistralrs-core/src/models/<new_model>/`
1. Add pipeline variant in `mistralrs-core/src/pipeline/normal.rs`
1. Register in `ModelSelected` enum
1. Add example in `mistralrs/examples/`
1. Update docs in `docs/<MODEL>.md`

### 11.2 Adding a New Tool

1. Implement tool in `mistralrs-agent-tools/src/tools/<category>/`
1. Add to `AgentToolProvider::get_tools()` in `core_integration.rs`
1. Add executor function `execute_<tool>()`
1. Add integration test
1. Update `docs/AGENT_TOOLS.md`

______________________________________________________________________

**Document Maintained By**: GitHub Copilot\
**Last Updated**: 2025-10-09\
**Next Review**: After major architectural changes
