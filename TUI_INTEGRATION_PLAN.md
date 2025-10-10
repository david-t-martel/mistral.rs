# mistralrs-tui Comprehensive Integration Plan

**Created**: 2024-12-XX\
**Status**: Phase 0 - Planning\
**Goal**: Integrate mistralrs-tui with full mistral.rs capabilities including agent tools, vision models, MCP client/server, and all modalities

## Executive Summary

The mistralrs-tui is currently a standalone terminal UI with basic chat functionality and session management. This plan outlines the integration of:

1. **Agent Tools Integration** - Enable tool calling within TUI using mistralrs-agent-tools
1. **Vision Model Support** - Add multimodal capabilities (images, diagrams)
1. **MCP Client Integration** - Connect to external MCP servers for enhanced functionality
1. **MCP Server Mode** - Allow TUI to act as MCP server for other clients
1. **Speech/Audio Support** - Integrate speech models when available
1. **Diffusion Support** - Add image generation capabilities
1. **Advanced UI Features** - Metrics, tool visualization, multimodal previews

## Current State Analysis

### Existing Components

#### mistralrs-tui (v0.6.0)

- **Status**: Basic functional TUI
- **Features**:
  - Session management with SQLite
  - Model inventory and discovery
  - Terminal-based chat interface
  - GPU-accelerated rendering (optional)
  - Agent feature flag exists but limited integration
- **Dependencies**: ratatui, sqlx, tokio, crossterm
- **Entry Point**: `src/main.rs`
- **Backend**: `src/backend/` (terminal.rs, gpu.rs)

#### mistralrs-agent-tools

- **Status**: Comprehensive tool library
- **Tools**: File ops, text processing, search, shell execution
- **Catalog**: `src/catalog.rs` - Tool definitions for LLM
- **Integration**: Limited to server-core currently
- **TODO Items**: Multiple unimplemented utilities (output, system, numeric, security)

#### mistralrs-mcp

- **Status**: MCP client implementation
- **Features**: Client connection, tool registration, reliability
- **Missing**: Server implementation, TUI integration
- **Files**: client.rs, transport.rs, capabilities.rs, tools.rs

#### mistralrs-vision

- **Status**: Vision model support in core
- **Integration**: Not wired to TUI
- **Capabilities**: Image processing, multimodal pipelines

### Gap Analysis

#### Critical Gaps

1. **No TUI Agent Execution** - Agent tools not executable from TUI
1. **No Vision in TUI** - Cannot handle image inputs/outputs
1. **No MCP in TUI** - Cannot act as client or server
1. **Limited Tool Visualization** - No UI for tool call feedback
1. **No Multimodal Preview** - Cannot display images/audio in terminal

#### Technical Debt

- TODO items across 30+ files
- Unwrap calls throughout codebase (potential panics)
- Incomplete error handling in agent tools
- Missing documentation for TUI features

## Integration Architecture

### Component Relationships

```
mistralrs-tui (Terminal UI)
├── mistralrs-core (Model Engine)
│   ├── mistralrs-vision (Vision Models)
│   └── mistralrs-audio (Speech Models)
├── mistralrs-agent-tools (Tool Execution)
│   └── Sandbox & Security
├── mistralrs-mcp (MCP Protocol)
│   ├── Client Mode (→ External Servers)
│   └── Server Mode (← External Clients)
└── mistralrs-server-core (Shared Logic)
```

### Data Flow

```
User Input (TUI)
    ↓
Command Router
    ↓
    ├→ Chat Message → Model → Response (Text/Vision/Speech)
    ├→ Tool Call → Agent Tools → Execution → Result
    ├→ MCP Request → MCP Client → External Tool → Response
    └→ File Upload → Vision Pipeline → Analysis
```

## Implementation Phases

### Phase 1: Agent Tools in TUI (1-2 weeks)

#### Goals

- Execute agent tools directly from TUI chat
- Display tool execution progress and results
- Integrate with existing tool catalog

#### Tasks

##### 1.1 Update TUI Config

- [ ] Add agent configuration section
- [ ] Define sandbox root for tool execution
- [ ] Configure tool permissions and restrictions
- [ ] Add MCP client config for TUI

**File**: `mistralrs-tui/src/config.rs`

##### 1.2 Create Agent Controller

- [ ] Implement `AgentController` in `src/agent/controller.rs`
- [ ] Handle tool call parsing from LLM responses
- [ ] Execute tools via mistralrs-agent-tools
- [ ] Manage tool execution state and history

**New Files**:

- `mistralrs-tui/src/agent/controller.rs`
- `mistralrs-tui/src/agent/state.rs`

##### 1.3 Update UI Components

- [ ] Add tool execution panel
- [ ] Show tool call visualization
- [ ] Display tool results inline with chat
- [ ] Add progress indicators for long-running tools

**Files**: `mistralrs-tui/src/agent/ui.rs`, `mistralrs-tui/src/ui.rs`

##### 1.4 Wire to Main App

- [ ] Update `App::initialise` to include agent controller
- [ ] Handle tool call events in main loop
- [ ] Persist tool history to SQLite

**File**: `mistralrs-tui/src/app.rs`

#### Success Criteria

- User can chat with model that calls tools
- Tool execution visible in UI
- Results integrated into conversation
- No crashes from tool execution

### Phase 2: Vision Model Support (1 week)

#### Goals

- Load vision models in TUI
- Upload and display images
- Receive image-based responses

#### Tasks

##### 2.1 Extend Model Inventory

- [ ] Detect vision-capable models
- [ ] Add vision model metadata
- [ ] Update model loading logic

**File**: `mistralrs-tui/src/inventory.rs`

##### 2.2 Image Upload

- [ ] Add file picker UI component
- [ ] Support drag-and-drop (terminal permitting)
- [ ] Validate and preview images
- [ ] Encode for vision pipeline

**New Files**:

- `mistralrs-tui/src/input/file_picker.rs`
- `mistralrs-tui/src/ui/image_preview.rs`

##### 2.3 Vision Pipeline Integration

- [ ] Connect to mistralrs-vision
- [ ] Handle vision-plain model loading
- [ ] Process image inputs with prompts
- [ ] Display vision model responses

**Files**: `mistralrs-tui/src/backend/mod.rs`, `src/session.rs`

##### 2.4 Image Display

- [ ] ASCII art preview for terminal
- [ ] Kitty/iTerm2 image protocol support
- [ ] Sixel graphics support
- [ ] Fallback to file path display

**New File**: `mistralrs-tui/src/ui/image_display.rs`

#### Success Criteria

- Load Qwen2-VL or similar
- Upload image through UI
- Ask questions about image
- See responses in chat

### Phase 3: MCP Client Integration (1-2 weeks)

#### Goals

- Connect TUI to external MCP servers
- Use MCP tools alongside native tools
- Visualize MCP tool calls

#### Tasks

##### 3.1 MCP Config in TUI

- [ ] Load MCP_CONFIG.json
- [ ] Initialize MCP client pool
- [ ] Handle MCP server lifecycle
- [ ] Auto-register tools on startup

**Files**: `mistralrs-tui/src/config.rs`, `src/main.rs`

##### 3.2 MCP Tool Registry

- [ ] Merge MCP tools with native tools
- [ ] Provide unified tool catalog to LLM
- [ ] Route tool calls to correct handler
- [ ] Handle MCP-specific errors

**New File**: `mistralrs-tui/src/agent/mcp_bridge.rs`

##### 3.3 MCP UI Components

- [ ] Show MCP server status
- [ ] Display MCP tool list
- [ ] Visualize MCP tool execution
- [ ] Show MCP errors distinctly

**Files**: `mistralrs-tui/src/agent/ui.rs`, `src/ui.rs`

##### 3.4 Testing

- [ ] Test with Memory MCP server
- [ ] Test with GitHub MCP server
- [ ] Test with Filesystem MCP server
- [ ] Handle server failures gracefully

#### Success Criteria

- TUI connects to 3+ MCP servers
- Can call MCP tools from chat
- Server status visible in UI
- Errors handled without crash

### Phase 4: MCP Server Mode (1 week)

#### Goals

- Expose TUI as MCP server
- Allow external clients to use TUI's model
- Serve tool capabilities to other apps

#### Tasks

##### 4.1 MCP Server Implementation

- [ ] Implement MCP server protocol
- [ ] Expose model inference as MCP tool
- [ ] Provide session management via MCP
- [ ] Handle concurrent client connections

**New Files**:

- `mistralrs-tui/src/mcp_server/mod.rs`
- `mistralrs-tui/src/mcp_server/protocol.rs`
- `mistralrs-tui/src/mcp_server/handlers.rs`

##### 4.2 Tool Exposure

- [ ] Expose agent tools as MCP resources
- [ ] Provide tool schemas via MCP
- [ ] Handle tool execution requests
- [ ] Return results in MCP format

**File**: `mistralrs-tui/src/mcp_server/tools.rs`

##### 4.3 CLI Integration

- [ ] Add `--mcp-port` flag
- [ ] Enable headless mode for MCP server
- [ ] Log MCP requests
- [ ] Graceful shutdown

**File**: `mistralrs-tui/src/main.rs`

##### 4.4 Documentation

- [ ] Document MCP server usage
- [ ] Provide example client code
- [ ] Add to README

#### Success Criteria

- TUI starts with --mcp-port 3000
- External client connects successfully
- Can invoke model via MCP
- Can call exposed tools

### Phase 5: Speech & Diffusion (2 weeks)

#### Goals

- Support audio models
- Support diffusion models
- Provide appropriate UI

#### Tasks

##### 5.1 Speech Model Support

- [ ] Detect speech models in inventory
- [ ] Add audio upload UI
- [ ] Integrate mistralrs-audio
- [ ] Display audio transcription
- [ ] Show audio waveforms (if possible)

**Files**: Multiple across inventory, backend, ui

##### 5.2 Diffusion Model Support

- [ ] Detect diffusion models
- [ ] Add text-to-image interface
- [ ] Integrate diffusion pipeline
- [ ] Display generated images
- [ ] Save images to disk

**Files**: Multiple across inventory, backend, ui

##### 5.3 Multimodal Sessions

- [ ] Support mixed modality in one session
- [ ] Handle modality switching
- [ ] Persist multimodal history
- [ ] Export with attachments

**File**: `mistralrs-tui/src/session.rs`

#### Success Criteria

- Can transcribe audio
- Can generate images
- Multimodal sessions work
- All outputs saved correctly

### Phase 6: Advanced Features (Ongoing)

#### Goals

- Polish UI/UX
- Add metrics and monitoring
- Improve error handling
- Complete TODO items

#### Tasks

##### 6.1 Metrics Dashboard

- [ ] Real-time token usage
- [ ] Latency charts
- [ ] VRAM monitoring
- [ ] Request queue depth

**New File**: `mistralrs-tui/src/ui/metrics.rs`

##### 6.2 Error Handling

- [ ] Remove all unwrap() calls
- [ ] Add proper error types
- [ ] Display errors in UI
- [ ] Add retry logic

**Multiple files**

##### 6.3 Performance

- [ ] Profile rendering bottlenecks
- [ ] Optimize SQLite queries
- [ ] Cache model metadata
- [ ] Reduce memory allocations

**Multiple files**

##### 6.4 Documentation

- [ ] Complete API docs
- [ ] Add usage examples
- [ ] Create user guide
- [ ] Document troubleshooting

**docs/ directory**

## TODO Items to Address

### Critical TODOs (From Code Scan)

1. **mistralrs-agent-tools/src/tools/output/mod.rs**

   - Implement output utilities (echo, printf, yes)

1. **mistralrs-agent-tools/src/tools/system/mod.rs**

   - Implement system utilities (arch, hostname, whoami)

1. **mistralrs-agent-tools/src/tools/numeric/mod.rs**

   - Implement numeric utilities (expr, factor, numfmt)

1. **mistralrs-agent-tools/src/tools/shell/mod.rs**

   - Implement specialized shell executors (pwsh, cmd, bash)

1. **mistralrs-agent-tools/src/tools/file/mod.rs**

   - Implement remaining file operations (cp, mv, rm, etc.)

1. **mistralrs-agent-tools/src/tools/security/mod.rs**

   - Implement security utilities (shred, truncate, mktemp)

1. **mistralrs-agent-tools/src/tools/text/mod.rs**

   - Implement text processing (cut, tr, base64, etc.)

1. **mistralrs-agent-tools/src/tools/analysis/mod.rs**

   - Implement analysis utilities (wc, du, cksum, etc.)

1. **mistralrs-agent-tools/src/tools/search/mod.rs**

   - Complete search tool implementations

1. **mistralrs-agent-tools/src/tools/testing/mod.rs**

   - Implement testing utilities

### Non-Critical TODOs

- Various benchmark descriptions
- Example comments in tool definitions
- Documentation improvements

## Unwrap Elimination Strategy

### Scan Results

- Approximately 150-200 unwrap() calls across codebase
- Highest concentration in:
  - mistralrs-quant
  - mistralrs-paged-attn
  - mistralrs-tui/src/agent
  - mistralrs-agent-tools

### Elimination Approach

1. **Test/Example Code** - Lower priority, mark with comments
1. **Configuration Loading** - Replace with `context()` and proper errors
1. **Data Parsing** - Add validation and return `Result`
1. **Indexing** - Use `get()` with proper error handling
1. **Lock Acquisition** - Handle poison errors
1. **Type Conversion** - Use `ok_or()` or `map_err()`

### Priority Order

1. Public API functions (user-facing)
1. Core library code (engine, pipelines)
1. Server endpoints (HTTP, MCP)
1. TUI interaction code
1. Internal utilities
1. Test/benchmark code

## Development Workflow

### Setup

```bash
# Enable TUI agent feature
cd mistralrs-tui
cargo build --features tui-agent

# Run with agent support
cargo run --features tui-agent -- --verbose
```

### Testing

```bash
# Test TUI components
make test-tui

# Test agent tools
cd mistralrs-agent-tools
cargo test

# Test MCP integration
cd mistralrs-mcp
cargo test
```

### Validation

```bash
# Check builds
make check

# Lint
make lint

# Format
make fmt

# Full CI pipeline
make ci
```

## Git Workflow

### Branch Strategy

- `main` - Stable release branch
- `fork/main` - Fork of upstream
- Feature branches: `feature/tui-agent-integration`, etc.
- Chore branches: `chore/unwrap-elimination`, etc.

### Commit Convention

```
feat(tui): Add agent tool execution
fix(tui): Handle MCP connection errors
chore(tui): Remove unwrap calls
docs(tui): Add integration guide
test(tui): Add agent tool tests
```

### PR Tags

- Tag reviewers: `@codex`, `@gemini`
- Labels: `enhancement`, `bug`, `documentation`, `testing`
- Link to issues/milestones

## Success Metrics

### Phase 1 Success

- [ ] 10+ agent tools callable from TUI
- [ ] Tool execution visible in UI
- [ ] No panics during tool use
- [ ] Tests pass

### Phase 2 Success

- [ ] Load Qwen2-VL-2B model
- [ ] Upload and analyze image
- [ ] Display image in terminal
- [ ] Vision responses work

### Phase 3 Success

- [ ] Connect to 3+ MCP servers
- [ ] Call GitHub API via MCP
- [ ] Use Memory MCP for persistence
- [ ] Filesystem operations via MCP

### Phase 4 Success

- [ ] TUI runs as MCP server
- [ ] External client connects
- [ ] Model inference via MCP
- [ ] Tool exposure works

### Phase 5 Success

- [ ] Audio transcription works
- [ ] Image generation works
- [ ] Multimodal session persists
- [ ] All modalities documented

### Phase 6 Success

- [ ] Zero unwrap() in public APIs
- [ ] Full test coverage (>80%)
- [ ] Performance benchmarks
- [ ] Complete documentation

## Risk Mitigation

### Technical Risks

1. **Terminal Limitations** - Not all terminals support images

   - Mitigation: Provide fallback text descriptions

1. **MCP Complexity** - Protocol implementation errors

   - Mitigation: Use existing mistralrs-mcp, extensive testing

1. **Performance** - UI lag with heavy tool use

   - Mitigation: Async execution, progress indicators

1. **Memory** - Large models + images + history

   - Mitigation: Configurable limits, cleanup strategies

### Project Risks

1. **Scope Creep** - Too many features at once

   - Mitigation: Phased approach, MVP first

1. **Breaking Changes** - Upstream mistral.rs updates

   - Mitigation: Pin versions, regular rebasing

1. **Testing Gaps** - Hard to test UI components

   - Mitigation: Separate logic from rendering, integration tests

## Resources & References

### Documentation

- `AGENTS.md` - Project guidelines
- `CLAUDE.md` - Detailed workflows
- `mistralrs-tui/README.md` - TUI roadmap
- `mistralrs-agent-tools/TODO_STATUS.md` - Agent tools status
- `TODO.md` - Project TODO items

### Examples

- `examples/server/` - Server integration examples
- `examples/MCP_QUICK_START.md` - MCP usage
- `mistralrs-agent-tools/examples/` - Tool usage

### Code References

- `mistralrs-server-core/src/routes.rs` - API endpoints
- `mistralrs-core/src/pipeline/` - Model pipelines
- `mistralrs-mcp/src/client.rs` - MCP client implementation
- `mistralrs-agent-tools/src/catalog.rs` - Tool definitions

## Next Steps

### Immediate Actions

1. Review and approve this plan
1. Create GitHub issues for each phase
1. Set up project board
1. Begin Phase 1 implementation

### Week 1 Goals

- [ ] Complete Phase 1.1 (Config updates)
- [ ] Start Phase 1.2 (Agent controller)
- [ ] Document API decisions
- [ ] Write initial tests

### Month 1 Goals

- [ ] Complete Phase 1 (Agent tools)
- [ ] Complete Phase 2 (Vision support)
- [ ] Begin Phase 3 (MCP client)
- [ ] 50% unwrap elimination

### Quarter Goals

- [ ] Complete all phases 1-5
- [ ] 100% unwrap elimination
- [ ] Full test coverage
- [ ] Production-ready release

______________________________________________________________________

**Maintainer Notes**: This is a living document. Update after each milestone. Track progress in linked GitHub issues/PRs.
