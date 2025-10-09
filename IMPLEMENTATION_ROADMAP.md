# Implementation Roadmap - Phase 1 Execution

**Status**: In Progress\
**Current Focus**: Agent Tools Completion & Unwrap Elimination\
**Target**: Production-Ready TUI with Full Integration

## Session Overview

This document tracks the actual implementation work based on the comprehensive analysis in `TUI_INTEGRATION_PLAN.md`.

## Current Priority: Critical Foundation Work

### 1. Complete Agent Tools Implementation (Week 1-2)

#### Status: IN PROGRESS

The mistralrs-agent-tools library has several TODO stubs that need implementation before full TUI integration can proceed.

#### Tasks

##### A. Output Utilities (`src/tools/output/mod.rs`)

**Status**: TODO\
**Priority**: Medium\
**Effort**: 2 days

Implement:

- `echo` - Print text to output
- `printf` - Formatted text output
- `yes` - Repeat output

**Files to create**:

- `mistralrs-agent-tools/src/tools/output/echo.rs`
- `mistralrs-agent-tools/src/tools/output/printf.rs`
- `mistralrs-agent-tools/src/tools/output/yes.rs`

##### B. System Utilities (`src/tools/system/mod.rs`)

**Status**: TODO\
**Priority**: High (needed for diagnostics)\
**Effort**: 2 days

Implement:

- `arch` - Display system architecture
- `hostname` - Display host name
- `whoami` - Display current user

**Files to create**:

- `mistralrs-agent-tools/src/tools/system/arch.rs`
- `mistralrs-agent-tools/src/tools/system/hostname.rs`
- `mistralrs-agent-tools/src/tools/system/whoami.rs`

##### C. Numeric Utilities (`src/tools/numeric/mod.rs`)

**Status**: TODO\
**Priority**: Low\
**Effort**: 3 days

Implement:

- `expr` - Evaluate expressions
- `factor` - Prime factorization
- `numfmt` - Number formatting

**Files to create**:

- `mistralrs-agent-tools/src/tools/numeric/expr.rs`
- `mistralrs-agent-tools/src/tools/numeric/factor.rs`
- `mistralrs-agent-tools/src/tools/numeric/numfmt.rs`

##### D. File Operations (`src/tools/file/mod.rs`)

**Status**: TODO\
**Priority**: CRITICAL\
**Effort**: 1 week

Currently only `ls` is implemented. Need:

- `cp` - Copy files/directories
- `mv` - Move/rename files
- `rm` - Remove files/directories
- `mkdir` - Create directories
- `touch` - Create/update files
- `find` - Find files by criteria

**Files to create**:

- `mistralrs-agent-tools/src/tools/file/cp.rs`
- `mistralrs-agent-tools/src/tools/file/mv.rs`
- `mistralrs-agent-tools/src/tools/file/rm.rs`
- `mistralrs-agent-tools/src/tools/file/mkdir.rs`
- `mistralrs-agent-tools/src/tools/file/touch.rs`
- `mistralrs-agent-tools/src/tools/file/find.rs`

##### E. Text Processing (`src/tools/text/mod.rs`)

**Status**: TODO\
**Priority**: High\
**Effort**: 4 days

Implement:

- `cut` - Extract columns
- `tr` - Translate characters
- `base64` - Base64 encoding/decoding
- `base32` - Base32 encoding
- `sed` - Stream editor (basic)

**Files to create**:

- `mistralrs-agent-tools/src/tools/text/cut.rs`
- `mistralrs-agent-tools/src/tools/text/tr.rs`
- `mistralrs-agent-tools/src/tools/text/base64.rs`
- `mistralrs-agent-tools/src/tools/text/base32.rs`

##### F. Analysis Utilities (`src/tools/analysis/mod.rs`)

**Status**: TODO\
**Priority**: Medium\
**Effort**: 3 days

Implement:

- `wc` - Word count (currently exists but may need enhancement)
- `du` - Disk usage
- `cksum` - Checksum
- `md5sum` - MD5 hash
- `sha256sum` - SHA256 hash

**Files to create**:

- `mistralrs-agent-tools/src/tools/analysis/du.rs`
- `mistralrs-agent-tools/src/tools/analysis/cksum.rs`
- `mistralrs-agent-tools/src/tools/analysis/md5sum.rs`
- `mistralrs-agent-tools/src/tools/analysis/sha256sum.rs`

##### G. Security Utilities (`src/tools/security/mod.rs`)

**Status**: TODO\
**Priority**: Low\
**Effort**: 2 days

Implement:

- `shred` - Secure file deletion
- `truncate` - Truncate files
- `mktemp` - Create temporary files

**Files to create**:

- `mistralrs-agent-tools/src/tools/security/shred.rs`
- `mistralrs-agent-tools/src/tools/security/truncate.rs`
- `mistralrs-agent-tools/src/tools/security/mktemp.rs`

### 2. Unwrap Elimination Campaign (Ongoing)

#### Status: IN PROGRESS

**Goal**: Eliminate all `unwrap()` calls from production code paths.

#### Strategy

1. **Tier 1: Public APIs** (This week)

   - mistralrs-tui/src/\*\* (all user-facing code)
   - mistralrs-agent-tools/src/\*\* (public tool interfaces)
   - mistralrs-mcp/src/client.rs
   - mistralrs-server-core/src/routes.rs

1. **Tier 2: Core Libraries** (Next week)

   - mistralrs-core/src/pipeline/\*\*
   - mistralrs-quant/src/\*\*
   - mistralrs-paged-attn/src/\*\*

1. **Tier 3: Internal Utilities** (Week 3)

   - Remaining support code
   - Build scripts
   - Benchmarks (can remain with // allow(unwrap))

#### Current Unwrap Count by Module

From scan:

- `mistralrs-quant`: ~50 unwraps (many in `bail!` error paths)
- `mistralrs-tui`: ~30 unwraps
- `mistralrs-agent-tools`: ~40 unwraps
- `mistralrs-paged-attn`: ~20 unwraps
- Other modules: ~50-100 unwraps

**Total Estimated**: 150-200 unwraps to address

#### Replacement Patterns

```rust
// BEFORE (panic-prone)
let value = map.get("key").unwrap();

// AFTER (safe)
let value = map.get("key")
    .ok_or_else(|| anyhow::anyhow!("Missing key in map"))?;
```

```rust
// BEFORE
let parsed = str.parse::<u64>().unwrap();

// AFTER
let parsed = str.parse::<u64>()
    .context("Failed to parse as u64")?;
```

```rust
// BEFORE  
let lock = mutex.lock().unwrap();

// AFTER
let lock = mutex.lock()
    .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?;
```

### 3. Bail! Optimization (Week 2)

#### Status: ANALYSIS NEEDED

Many `bail!` calls could be replaced with proper error handling or feature implementation.

#### Approach

1. Scan all `bail!("Unimplemented")` or `bail!("Not supported")` calls

1. Categorize:

   - Can be implemented now (do it)
   - Needs future work (document with TODO and proper error type)
   - Truly unsupported (keep bail with clear message)

1. For legitimate bail calls, ensure error messages are helpful:

```rust
// BEFORE
bail!("not supported");

// AFTER  
bail!(
    "Operation not supported for this model architecture. \
     Use --model-type=vision for multimodal inputs."
);
```

### 4. TUI Agent Integration (Week 2-3)

#### Status: PLANNED

Based on existing `mistralrs-tui/src/agent/` code which has solid foundation.

#### Immediate Work

##### A. Complete Agent Controller

**File**: `mistralrs-tui/src/agent/controller.rs` (NEW)

```rust
//! Agent execution controller for TUI
//!
//! Manages tool call lifecycle:
//! - Parse tool calls from LLM responses
//! - Execute via ToolExecutor
//! - Update UI state
//! - Persist to session history

pub struct AgentController {
    executor: ToolExecutor,
    event_bus: EventBus,
    active_calls: HashMap<Uuid, ToolCall>,
}

impl AgentController {
    pub async fn process_llm_response(&mut self, response: &str) -> Result<Vec<ToolCall>>;
    pub async fn execute_tool_call(&mut self, call: ToolCall) -> Result<ToolCallResult>;
    pub fn get_active_calls(&self) -> &HashMap<Uuid, ToolCall>;
}
```

##### B. Update App Integration

**File**: `mistralrs-tui/src/app.rs`

Modifications:

1. Add AgentController to App struct (behind feature flag)
1. Wire tool execution events to UI updates
1. Display tool calls inline with chat
1. Persist tool history to SQLite

##### C. Enhance UI Components

**File**: `mistralrs-tui/src/agent/ui.rs`

Additions:

1. Tool execution progress widget
1. Tool result display (success/failure)
1. Tool history panel
1. Interactive tool selector

### 5. Testing Infrastructure (Week 3)

#### Status: PLANNED

#### Test Coverage Goals

- **Unit Tests**: 80%+ for new agent code
- **Integration Tests**: Key workflows (chat + tools, vision, MCP)
- **E2E Tests**: Full TUI session with real model

#### Test Files to Create

```
mistralrs-tui/tests/
├── integration/
│   ├── agent_execution.rs
│   ├── mcp_integration.rs
│   └── vision_workflow.rs
├── unit/
│   ├── agent_controller.rs
│   └── tool_executor.rs
└── e2e/
    └── full_session.rs

mistralrs-agent-tools/tests/
├── tools/
│   ├── file_ops.rs
│   ├── text_proc.rs
│   └── system.rs
└── sandbox_security.rs
```

## Weekly Milestones

### Week 1: Foundation

**Dates**: Current week\
**Goals**:

- [x] Complete TUI integration plan
- [ ] Implement critical file operations (cp, mv, mkdir, rm)
- [ ] Implement system utilities (arch, hostname, whoami)
- [ ] Begin unwrap elimination in mistralrs-tui
- [ ] Document 50+ files

**Deliverables**:

- TUI_INTEGRATION_PLAN.md
- 4 new tool implementations
- 30+ unwraps fixed
- Test suite for new tools

### Week 2: Integration

**Dates**: Next week\
**Goals**:

- [ ] Complete AgentController implementation
- [ ] Wire agent to TUI app
- [ ] Implement text processing tools
- [ ] Continue unwrap elimination (mistralrs-agent-tools)
- [ ] Add integration tests

**Deliverables**:

- Working tool execution in TUI
- 5+ tool implementations
- 40+ unwraps fixed
- 10+ integration tests

### Week 3: Polish & Vision

**Dates**: Week after next\
**Goals**:

- [ ] Begin vision model integration
- [ ] Complete unwrap elimination in Tier 1
- [ ] Add metrics dashboard
- [ ] Performance profiling
- [ ] Documentation updates

**Deliverables**:

- Vision support in TUI
- Zero unwraps in public APIs
- Performance benchmarks
- User documentation

### Week 4: MCP & Advanced Features

**Dates**: Fourth week\
**Goals**:

- [ ] MCP client integration in TUI
- [ ] Expose TUI as MCP server
- [ ] Complete all tier 2 unwraps
- [ ] End-to-end testing
- [ ] Release candidate

**Deliverables**:

- Full MCP support
- Production-ready build
- Complete test coverage
- Release notes

## Git Workflow for This Session

### Branch Strategy

**Current branch**: `chore/todo-warning`\
**Next branches**:

- `feat/agent-tools-completion` - New tool implementations
- `chore/unwrap-elimination` - Unwrap fixes
- `feat/tui-agent-controller` - Agent controller
- `feat/tui-vision-support` - Vision integration
- `feat/tui-mcp-integration` - MCP support

### Commit Convention

```
<type>(<scope>): <subject>

<body>

Tags: gemini, codex
```

Types: `feat`, `fix`, `docs`, `test`, `chore`, `refactor`

Scopes: `tui`, `agent-tools`, `mcp`, `core`, `server`, etc.

## Success Criteria

### Phase 1 Success (End of Week 2)

- [ ] 10+ new tool implementations
- [ ] Tools executable from TUI
- [ ] 100+ unwraps eliminated
- [ ] Zero panics in normal operation
- [ ] 70%+ test coverage for new code

### Phase 2 Success (End of Week 4)

- [ ] Vision models work in TUI
- [ ] MCP client connected to 3+ servers
- [ ] TUI acts as MCP server
- [ ] Zero unwraps in public APIs
- [ ] Full documentation

### MVP Complete (End of Month)

- [ ] All modalities supported (text, vision, speech if available)
- [ ] All core tools implemented
- [ ] Production-ready error handling
- [ ] Performance benchmarks documented
- [ ] Ready for PR to upstream

## Daily Log

### Day 1 (Today)

**Completed**:

- [x] Comprehensive codebase analysis
- [x] Created TUI_INTEGRATION_PLAN.md
- [x] Created IMPLEMENTATION_ROADMAP.md
- [x] Committed and pushed to chore/todo-warning

**In Progress**:

- [ ] Beginning file operations implementation

**Blockers**: None

**Next**: Start with critical file ops (cp, mv, mkdir, rm, touch)

______________________________________________________________________

**Last Updated**: 2024-12-XX\
**Maintainer**: Auto-Claude + Gemini + Codex\
**Review Cadence**: Daily updates, weekly milestone reviews
