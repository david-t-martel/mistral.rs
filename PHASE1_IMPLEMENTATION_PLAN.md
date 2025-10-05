# Phase 1 Implementation Plan: Foundation Integration

**Status**: In Progress\
**Started**: 2025-10-05\
**Target Completion**: 2 weeks

## Overview

Phase 1 establishes the foundational integration between mistralrs-agent-tools and mistralrs-core, enabling seamless tool discovery, registration, and execution through the core's existing tool infrastructure.

## Current Architecture Analysis

### mistralrs-core Tool System

- **Location**: `mistralrs-core/src/lib.rs` (lines 84-86, 145-146, 270-271)
- **Key Types**:
  - `ToolCallback`: `Arc<dyn Fn(String) -> BoxFuture<'static, anyhow::Result<String>> + Send + Sync>`
  - `ToolCallbackWithTool`: Pairs a callback with a `Tool` definition (JSON schema)
  - `Tool`: MCP-style tool definition with name, description, input_schema
- **Storage**: `HashMap<String, Arc<ToolCallback>>` and `HashMap<String, ToolCallbackWithTool>`
- **Integration**: Passed to Engine during initialization, used by ReAct agent

### mistralrs-agent-tools Current State

- **Location**: `mistralrs-agent-tools/src/`
- **API**: High-level `AgentToolkit` with 90+ utilities
- **Sandboxing**: Full path validation and security enforcement
- **Coverage**: File ops, text processing, search, system info, shell execution
- **Gap**: No integration with mistralrs-core's tool system

### mistralrs-mcp Infrastructure

- **Location**: `mistralrs-mcp/src/`
- **Provides**: `Tool`, `ToolCallback`, `ToolCallbackWithTool`, `McpClient`
- **Features**: MCP client for external servers, tool discovery, automatic registration
- **Usage**: Already integrated into mistralrs-core (lines 84-90, 507-541)

## Phase 1 Tasks

### Task 1.1: Create AgentToolProvider ✅ CURRENT

**Goal**: Wire agent-tools into mistralrs-core's tool callback system

**Implementation**:

```rust
// File: mistralrs-agent-tools/src/core_integration.rs

pub struct AgentToolProvider {
    toolkit: AgentToolkit,
    tool_prefix: Option<String>,
}

impl AgentToolProvider {
    pub fn new(sandbox_config: SandboxConfig) -> Self {
        Self {
            toolkit: AgentToolkit::new(sandbox_config),
            tool_prefix: None,
        }
    }
    
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.tool_prefix = Some(prefix.into());
        self
    }
    
    /// Generate Tool definitions for all agent-tools utilities
    pub fn get_tools(&self) -> Vec<Tool> {
        vec![
            self.create_tool("cat", "Read file contents", cat_schema()),
            self.create_tool("ls", "List directory", ls_schema()),
            self.create_tool("grep", "Search in files", grep_schema()),
            // ... all 90+ tools
        ]
    }
    
    /// Generate ToolCallbackWithTool for mistralrs-core integration
    pub fn get_tool_callbacks_with_tools(&self) -> HashMap<String, ToolCallbackWithTool> {
        let mut callbacks = HashMap::new();
        
        // Register all tools
        for tool in self.get_tools() {
            let name = tool.name.clone();
            let toolkit = self.toolkit.clone();
            
            let callback = Arc::new(move |args: String| {
                let toolkit = toolkit.clone();
                let tool_name = name.clone();
                
                Box::pin(async move {
                    execute_agent_tool(&toolkit, &tool_name, &args).await
                }) as BoxFuture<'static, anyhow::Result<String>>
            });
            
            callbacks.insert(
                name.clone(),
                ToolCallbackWithTool {
                    callback,
                    tool: tool.clone(),
                },
            );
        }
        
        callbacks
    }
}

async fn execute_agent_tool(
    toolkit: &AgentToolkit,
    tool_name: &str,
    args: &str,
) -> anyhow::Result<String> {
    // Parse JSON args
    let args: serde_json::Value = serde_json::from_str(args)?;
    
    // Route to appropriate tool
    match tool_name {
        "cat" => {
            let paths: Vec<&Path> = ...; // parse from args
            let options = CatOptions::from_json(&args)?;
            let result = toolkit.cat(&paths, &options)?;
            Ok(result)
        }
        "ls" => { /* ... */ }
        "grep" => { /* ... */ }
        // ... all tools
        _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_name))
    }
}
```

**Files to Create**:

1. `mistralrs-agent-tools/src/core_integration.rs` - Provider implementation
1. `mistralrs-agent-tools/src/tool_schemas.rs` - JSON schemas for all tools
1. `mistralrs-agent-tools/src/tool_execution.rs` - Execution routing logic

**Files to Modify**:

1. `mistralrs-agent-tools/src/lib.rs` - Add `pub mod core_integration;`
1. `mistralrs-agent-tools/Cargo.toml` - Add `mistralrs-mcp` dependency

**Testing**:

```rust
#[tokio::test]
async fn test_agent_tool_provider() {
    let provider = AgentToolProvider::new(SandboxConfig::default());
    let callbacks = provider.get_tool_callbacks_with_tools();
    
    assert!(callbacks.len() >= 90);
    assert!(callbacks.contains_key("cat"));
    
    // Test tool execution
    let cat_callback = &callbacks["cat"];
    let result = (cat_callback.callback)(r#"{"paths": ["test.txt"]}"#.to_string()).await;
    assert!(result.is_ok());
}
```

### Task 1.2: Expose Agent-Tools as MCP Server

**Goal**: Make agent-tools available to any MCP client

**Implementation**:

```rust
// File: mistralrs-agent-tools/src/mcp_server.rs

pub struct AgentToolsMcpServer {
    provider: AgentToolProvider,
    port: u16,
}

impl AgentToolsMcpServer {
    pub async fn new(sandbox_config: SandboxConfig, port: u16) -> Self {
        Self {
            provider: AgentToolProvider::new(sandbox_config),
            port,
        }
    }
    
    pub async fn serve(&self) -> anyhow::Result<()> {
        // Use mistralrs-mcp's server infrastructure
        let server = McpServer::new()
            .with_tools(self.provider.get_tools())
            .with_tool_executor(Box::new(self.provider.clone()));
        
        server.serve(format!("0.0.0.0:{}", self.port)).await
    }
}
```

**CLI Addition**:

```rust
// File: mistralrs-agent-tools/src/bin/agent-tools-mcp-server.rs

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    let config = SandboxConfig::new(args.sandbox_root)
        .with_allowed_paths(args.allowed_paths);
    
    let server = AgentToolsMcpServer::new(config, args.port).await;
    
    info!("Agent Tools MCP Server listening on port {}", args.port);
    server.serve().await
}
```

### Task 1.3: Migrate agent_mode to Core Callbacks

**Goal**: Remove manual tool routing from agent_mode

**Current State** (`mistralrs-server/src/main.rs` lines 442-445):

- Manual tool registration
- Hardcoded list
- No dynamic discovery

**New Implementation**:

```rust
// In mistralrs-server/src/main.rs

// Old (remove):
let tool_callbacks = /* manual hashmap */;

// New (add):
let agent_provider = AgentToolProvider::new(sandbox_config)
    .with_prefix("agent");
let tool_callbacks = agent_provider.get_tool_callbacks_with_tools();

// Pass to builder
runner = runner.with_tool_callbacks_with_tools(tool_callbacks);
```

### Task 1.4: Add Async Execution Support

**Goal**: Enable non-blocking tool execution

**Implementation**: Already async-first in design (using `BoxFuture`)

**Enhancements**:

1. Add timeout support
1. Add cancellation tokens
1. Add progress callbacks

### Task 1.5: Integration Tests

**Test Coverage**:

1. Tool discovery (all 90+ tools present)
1. Tool execution (each tool category)
1. Error handling (sandbox violations, invalid args)
1. Async behavior (concurrency, cancellation)
1. MCP server endpoints
1. Callback integration with mistralrs-core

## Success Criteria

- [ ] All 90+ agent-tools available through mistralrs-core
- [ ] MCP server running and serving tools
- [ ] agent_mode using AgentToolProvider
- [ ] All tests passing
- [ ] No breaking changes to existing APIs
- [ ] Documentation updated

## Dependencies

- `mistralrs-mcp` - Tool definitions and MCP infrastructure
- `tokio` - Async runtime
- `serde_json` - JSON schema generation
- `anyhow` - Error handling

## Timeline

- Days 1-3: Task 1.1 implementation
- Days 4-5: Task 1.2 implementation
- Days 6-7: Task 1.3 implementation
- Day 8: Task 1.4 enhancements
- Days 9-10: Task 1.5 testing
- Days 11-14: Documentation, polish, PR review

## Next Steps After Phase 1

Phase 2 will build on this foundation to add:

- MCP client in mistralrs-core for external tools
- Unified tool registry combining agent-tools + MCP + custom tools
- Tool result caching for performance
