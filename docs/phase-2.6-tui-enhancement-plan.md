# Phase 2.6: TUI Enhancement Plan for Agent Tools Integration

**Date**: 2025-10-05
**Status**: Planning Complete - Ready for Implementation

## Executive Summary

This document outlines the integration plan for connecting the `mistralrs-tui` crate with the `mistralrs-agent-tools` ecosystem. The goal is to enable the TUI to support full agent workflows with access to all 90+ tools from `AgentToolkit`.

## Current State Analysis

### What Exists

- ✅ `mistralrs-tui` crate with full terminal UI infrastructure
- ✅ Session management with SQLite persistence
- ✅ Model inventory and discovery system
- ✅ Chat interface with message history
- ✅ Workspace feature flag `tui-agent` (defined but not implemented)
- ✅ `mistralrs-agent-tools` with 90+ tools via `AgentToolkit`
- ✅ `tool_registry` infrastructure for tool discovery and execution
- ✅ `interactive_mode.rs` as reference implementation

### What's Missing

- ❌ No dependency on `mistralrs-agent-tools` in TUI `Cargo.toml`
- ❌ No agent mode UI components (tool execution panel, tool status)
- ❌ No integration with `tool_registry`
- ❌ No tool call display or interaction flow
- ❌ No agent-specific configuration options

### Gaps Identified

1. **Dependency Wiring**: TUI doesn't import agent-tools crate
1. **UI Components**: No widgets for displaying tool calls/results
1. **Agent Flow**: No integration point for agent request/response cycle
1. **Tool Discovery**: TUI doesn't know about available tools
1. **Configuration**: No agent-specific settings in TUI config

## Enhancement Objectives

### Primary Goals

1. **Enable Agent Mode**: Implement the `tui-agent` feature flag functionality
1. **Tool Integration**: Connect TUI to `AgentToolkit` via `tool_registry`
1. **UI Enhancement**: Add agent-specific UI components
1. **Configuration**: Add agent settings to TUI config
1. **Documentation**: Document agent mode usage

### Secondary Goals

1. **Tool Browser**: Add tool discovery/inspection panel
1. **Tool History**: Track and display tool execution history
1. **Interactive Tool Calls**: Allow manual tool invocation
1. **Agent Templates**: Provide pre-configured agent workflows

## Technical Architecture

### New Dependencies (in `mistralrs-tui/Cargo.toml`)

```toml
[dependencies]
mistralrs-agent-tools = { workspace = true, optional = true }
mistralrs-core = { workspace = true }

[features]
tui-agent = ["dep:mistralrs-agent-tools"]
```

### New Modules

1. **`src/agent/mod.rs`**: Agent mode coordinator
1. **`src/agent/toolkit.rs`**: AgentToolkit wrapper
1. **`src/agent/tool_calls.rs`**: Tool call execution and tracking
1. **`src/agent/ui.rs`**: Agent-specific UI widgets
1. **`src/config.rs`** (enhancement): Add agent configuration section

### Integration Points

#### 1. Session Context Enhancement

```rust
// In src/session.rs
pub struct SessionContext {
    // ... existing fields ...
    
    #[cfg(feature = "tui-agent")]
    pub agent_mode: Option<AgentMode>,
    #[cfg(feature = "tui-agent")]
    pub tool_calls: Vec<ToolCallRecord>,
}

#[cfg(feature = "tui-agent")]
pub struct AgentMode {
    pub enabled: bool,
    pub toolkit: Arc<AgentToolkit>,
    pub allowed_tools: Option<Vec<String>>,
    pub security_level: SecurityLevel,
}
```

#### 2. App State Enhancement

```rust
// In src/app.rs
pub struct App {
    // ... existing fields ...
    
    #[cfg(feature = "tui-agent")]
    agent_state: Option<AgentState>,
}

#[cfg(feature = "tui-agent")]
pub struct AgentState {
    toolkit: Arc<AgentToolkit>,
    tool_browser_cursor: usize,
    show_tool_panel: bool,
    tool_filter: String,
}
```

#### 3. UI Layout Enhancement

```
┌─────────────────────────────────────────────────────┐
│ mistral.rs TUI (Agent Mode)                         │
├───────────────┬─────────────────────┬───────────────┤
│ Sessions (S)  │ Chat (C)            │ Tools (T)     │
│               │                     │               │
│ • Active      │ User: Generate...  │ 📁 filesystem │
│   New Session │ Assistant: ...      │ 🌐 web        │
│   History     │ 🔧 [Tool: read_fi..│ 💻 command    │
│               │ Tool Result: ...   │ 🧮 compute    │
│               │                     │ 🔍 search     │
│               │ User: ...          │ ...           │
├───────────────┴─────────────────────┴───────────────┤
│ Status: Agent Mode | Tools: 90+ | Session: active  │
└─────────────────────────────────────────────────────┘
```

#### 4. Key Bindings (Agent Mode)

- `Ctrl+T`: Toggle tool panel
- `Ctrl+B`: Open tool browser
- `Ctrl+H`: Show tool call history
- `/`: Filter tools (in tool panel)
- `Enter`: Execute selected tool (in tool browser)

## Implementation Plan

### Phase 2.6.1: Foundation (2-3 hours)

**Priority**: CRITICAL
**Status**: Not Started

1. ✅ Add `mistralrs-agent-tools` dependency with feature gate
1. ✅ Create `src/agent/` module structure
1. ✅ Add `AgentToolkit` initialization in agent mode
1. ✅ Extend `SessionContext` with agent fields
1. ✅ Update TUI config to support agent settings

**Files to Modify:**

- `mistralrs-tui/Cargo.toml`
- `mistralrs-tui/src/lib.rs`
- `mistralrs-tui/src/session.rs`
- `mistralrs-tui/src/config.rs`

**Files to Create:**

- `mistralrs-tui/src/agent/mod.rs`
- `mistralrs-tui/src/agent/toolkit.rs`

### Phase 2.6.2: UI Components (3-4 hours)

**Priority**: HIGH
**Status**: Not Started

1. Create tool panel widget
1. Create tool call display component
1. Add tool result rendering
1. Implement tool browser modal
1. Add agent status indicators

**Files to Create:**

- `mistralrs-tui/src/agent/ui.rs`
- `mistralrs-tui/src/agent/widgets/tool_panel.rs`
- `mistralrs-tui/src/agent/widgets/tool_browser.rs`

**Files to Modify:**

- `mistralrs-tui/src/ui.rs`
- `mistralrs-tui/src/app.rs`

### Phase 2.6.3: Integration Logic (2-3 hours)

**Priority**: HIGH
**Status**: Not Started

1. Implement tool call execution flow
1. Add tool result handling
1. Integrate with message streaming
1. Add tool call persistence
1. Implement tool filtering/search

**Files to Create:**

- `mistralrs-tui/src/agent/tool_calls.rs`
- `mistralrs-tui/src/agent/executor.rs`

**Files to Modify:**

- `mistralrs-tui/src/app.rs`
- `mistralrs-tui/src/session.rs`

### Phase 2.6.4: Testing & Polish (2-3 hours)

**Priority**: MEDIUM
**Status**: Not Started

1. Add unit tests for agent components
1. Add integration tests for tool execution
1. Test all 90+ tools availability
1. Performance testing (tool call latency)
1. UI/UX polish and refinements

**Files to Create:**

- `mistralrs-tui/tests/agent_mode.rs`
- `mistralrs-tui/tests/tool_execution.rs`

### Phase 2.6.5: Documentation (1-2 hours)

**Priority**: MEDIUM
**Status**: Not Started

1. Update TUI README with agent mode section
1. Add agent mode usage guide
1. Document tool configuration
1. Add troubleshooting section
1. Create example configurations

**Files to Modify:**

- `mistralrs-tui/README.md`

**Files to Create:**

- `docs/tui-agent-mode-guide.md`
- `examples/tui-agent-config.toml`

## Configuration Schema

### New Section in `tui.toml`

```toml
[agent]
enabled = true
security_level = "Restricted"  # "Unrestricted", "Restricted", or "Sandboxed"

# Optional: Limit available tools
allowed_tools = [
    "filesystem::read_file",
    "web::fetch",
    "command::execute"
]

# Optional: Tool-specific configuration
[agent.tools.filesystem]
sandbox_paths = ["/home/user/projects", "/tmp"]

[agent.tools.command]
allowed_commands = ["ls", "cat", "grep"]
blocked_commands = ["rm", "dd"]

[agent.display]
show_tool_panel = true
show_tool_calls_inline = true
tool_call_animation = "dots"  # "dots", "spinner", "none"
```

## Database Schema Extensions

### New Table: `tool_calls`

```sql
CREATE TABLE tool_calls (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    message_id TEXT,
    tool_name TEXT NOT NULL,
    arguments TEXT NOT NULL,  -- JSON
    result TEXT,              -- JSON
    error TEXT,
    duration_ms INTEGER,
    created_at DATETIME NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    FOREIGN KEY (message_id) REFERENCES messages(id)
);

CREATE INDEX idx_tool_calls_session ON tool_calls(session_id);
CREATE INDEX idx_tool_calls_created ON tool_calls(created_at);
```

### New Table: `agent_settings`

```sql
CREATE TABLE agent_settings (
    session_id TEXT PRIMARY KEY,
    security_level TEXT NOT NULL,
    allowed_tools TEXT,  -- JSON array
    configuration TEXT,  -- JSON object
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);
```

## Testing Strategy

### Unit Tests

- ✅ Tool discovery and registration
- ✅ Tool call serialization/deserialization
- ✅ Security level enforcement
- ✅ Configuration validation
- ✅ UI widget rendering (snapshot tests)

### Integration Tests

- ✅ End-to-end tool execution flow
- ✅ Session persistence with tool calls
- ✅ Agent mode enable/disable
- ✅ Tool filtering and search
- ✅ Error handling and recovery

### Performance Tests

- ✅ Tool call latency (\<100ms overhead)
- ✅ UI responsiveness with many tool calls
- ✅ Database query performance
- ✅ Memory usage with agent mode enabled

## Success Criteria

### Must Have (Phase 2.6)

- ✅ TUI compiles with `tui-agent` feature
- ✅ Agent mode can be enabled via config
- ✅ AgentToolkit accessible with all 90+ tools
- ✅ Tool calls display in chat interface
- ✅ Tool results render correctly
- ✅ Basic tool browser functional
- ✅ Configuration persists across sessions

### Should Have (Phase 2.7)

- ✅ Interactive tool invocation
- ✅ Tool execution history view
- ✅ Advanced tool filtering
- ✅ Tool call analytics
- ✅ Agent workflow templates

### Nice to Have (Phase 2.8)

- ✅ Visual tool call graph
- ✅ Tool performance metrics
- ✅ Custom tool registration
- ✅ Multi-agent orchestration
- ✅ Tool call debugging mode

## Risk Assessment

### Technical Risks

1. **Performance**: Tool execution in TUI event loop

   - **Mitigation**: Use async tasks with cancellation

1. **UI Complexity**: Too many panels/widgets

   - **Mitigation**: Collapsible panels, modal overlays

1. **Security**: Unsafe tool execution

   - **Mitigation**: Leverage existing SecurityLevel system

### Implementation Risks

1. **Scope Creep**: Adding too many features

   - **Mitigation**: Stick to phase 2.6 must-haves only

1. **Breaking Changes**: Modifying session schema

   - **Mitigation**: Use database migrations

1. **Testing Coverage**: Insufficient integration tests

   - **Mitigation**: Test each tool category

## Timeline Estimate

- **Phase 2.6.1 Foundation**: 2-3 hours
- **Phase 2.6.2 UI Components**: 3-4 hours
- **Phase 2.6.3 Integration**: 2-3 hours
- **Phase 2.6.4 Testing**: 2-3 hours
- **Phase 2.6.5 Documentation**: 1-2 hours

**Total Estimated**: 10-15 hours of focused development

## Next Steps

1. ✅ Get approval on this enhancement plan
1. ⏳ Implement Phase 2.6.1 (Foundation)
1. ⏳ Implement Phase 2.6.2 (UI Components)
1. ⏳ Implement Phase 2.6.3 (Integration)
1. ⏳ Implement Phase 2.6.4 (Testing)
1. ⏳ Implement Phase 2.6.5 (Documentation)
1. ⏳ Create comprehensive PR for Phase 2.6
1. ⏳ Plan Phase 2.7 enhancements

## References

- TUI Roadmap: `mistralrs-tui/README.md`
- Agent Tools: `mistralrs-agent-tools/src/lib.rs`
- Interactive Mode: `mistralrs-server/src/interactive_mode.rs`
- Tool Registry: `mistralrs-agent-tools/src/tool_registry.rs`
- Phase 2.4 CLI Flags: `docs/phase-2.4-cli-flags-summary.md`

______________________________________________________________________

**Document Status**: Complete and ready for implementation
**Last Updated**: 2025-10-05
**Next Review**: After Phase 2.6 implementation
