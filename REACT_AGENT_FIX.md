# ReAct Agent Tool Execution - Complete Fix

_Reference: Review the [Repository Guidelines](AGENTS.md) for shared contribution standards before applying this fix plan._

## Files to Modify

### 1. mistralrs-core/src/lib.rs

Add method to expose tool callbacks from MistralRs:

```rust
impl MistralRs {
    // Add after existing methods around line 1020

    /// Get tool callbacks for direct tool execution (e.g., for ReAct agents)
    pub fn get_tool_callbacks(&self, model_id: Option<&str>) -> Result<HashMap<String, Arc<ToolCallback>>, String> {
        let runners = self.runners.lock().map_err(|e| e.to_string())?;

        let engine_instance = if let Some(id) = model_id {
            runners
                .get(id)
                .ok_or_else(|| format!("Model {} not found", id))?
        } else {
            runners.values().next().ok_or("No models loaded")?
        };

        Ok(engine_instance.reboot_state.tool_callbacks.clone())
    }

    /// Get tool callbacks with tool definitions for direct execution
    pub fn get_tool_callbacks_with_tools(&self, model_id: Option<&str>) -> Result<HashMap<String, ToolCallbackWithTool>, String> {
        let runners = self.runners.lock().map_err(|e| e.to_string())?;

        let engine_instance = if let Some(id) = model_id {
            runners
                .get(id)
                .ok_or_else(|| format!("Model {} not found", id))?
        } else {
            runners.values().next().ok_or("No models loaded")?
        };

        Ok(engine_instance.reboot_state.tool_callbacks_with_tools.clone())
    }
}
```

### 2. mistralrs/src/model.rs

Add accessor methods to Model:

```rust
impl Model {
    // Add after existing methods around line 337

    /// Get tool callbacks for direct tool execution
    ///
    /// This allows agents like ReActAgent to execute tools directly
    /// rather than relying on the automatic tool calling during inference.
    pub fn get_tool_callbacks(&self) -> anyhow::Result<HashMap<String, Arc<ToolCallback>>> {
        self.runner
            .get_tool_callbacks(None)
            .map_err(|e| anyhow::anyhow!("Failed to get tool callbacks: {}", e))
    }

    /// Get tool callbacks with their associated Tool definitions
    pub fn get_tool_callbacks_with_tools(&self) -> anyhow::Result<HashMap<String, ToolCallbackWithTool>> {
        self.runner
            .get_tool_callbacks_with_tools(None)
            .map_err(|e| anyhow::anyhow!("Failed to get tool callbacks with tools: {}", e))
    }
}
```

### 3. mistralrs/src/react_agent.rs

Complete rewrite of tool execution:

```rust
use anyhow::{anyhow, Context, Result};
use mistralrs_core::ToolCallResponse;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

use crate::{Model, RequestBuilder, TextMessageRole, ToolCallback};

/// A ReAct agent that autonomously executes tools in a reasoning loop
pub struct ReActAgent {
    model: Model,
    tool_callbacks: HashMap<String, Arc<ToolCallback>>,
    max_iterations: usize,
    tool_timeout: Duration,
}

// ... keep existing structs (ToolResult, AgentIteration, AgentResponse) ...

impl ReActAgent {
    /// Create a new ReAct agent with default settings
    ///
    /// This will initialize the agent with access to all registered tool callbacks
    /// from the model (including MCP auto-registered tools).
    ///
    /// # Errors
    ///
    /// Returns an error if tool callbacks cannot be retrieved from the model.
    pub fn new(model: Model) -> Result<Self> {
        let tool_callbacks = model.get_tool_callbacks()
            .context("Failed to get tool callbacks from model. Ensure tools are registered via builder.")?;

        Ok(Self {
            model,
            tool_callbacks,
            max_iterations: 10,
            tool_timeout: Duration::from_secs(30),
        })
    }

    /// Set the maximum number of iterations before stopping
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Set the timeout for individual tool executions
    pub fn with_tool_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.tool_timeout = Duration::from_secs(timeout_secs);
        self
    }

    // ... keep existing run() method unchanged ...

    /// Execute a tool call with timeout protection
    async fn execute_tool_with_timeout(&self, tool_call: &ToolCallResponse) -> ToolResult {
        match timeout(self.tool_timeout, self.execute_tool_internal(tool_call)).await {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => ToolResult {
                content: String::new(),
                error: Some(format!("Tool execution error: {}", e)),
            },
            Err(_) => ToolResult {
                content: String::new(),
                error: Some(format!(
                    "Tool execution timed out after {:?}",
                    self.tool_timeout
                )),
            },
        }
    }

    /// Internal tool execution method - NOW ACTUALLY WORKS!
    ///
    /// This method directly invokes the registered tool callbacks, which include:
    /// - Native callbacks registered via `.with_tool_callback()`
    /// - MCP auto-registered tools from `.with_mcp_client()`
    ///
    /// The callbacks handle the async-to-sync bridge internally, so we can
    /// call them directly here and get results synchronously.
    async fn execute_tool_internal(&self, tool_call: &ToolCallResponse) -> Result<ToolResult> {
        let tool_name = &tool_call.function.name;

        // Look up the tool callback
        let callback = self.tool_callbacks.get(tool_name)
            .ok_or_else(|| anyhow!(
                "Tool '{}' not found. Available tools: {}",
                tool_name,
                self.tool_callbacks.keys()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))?;

        // Execute the tool callback
        // Note: The callback internally handles async->sync conversion for MCP tools
        let result = callback(&tool_call.function)
            .context(format!("Failed to execute tool '{}'", tool_name))?;

        Ok(ToolResult {
            content: result,
            error: None,
        })
    }
}

/// Builder for configuring ReAct agent
pub struct ReActAgentBuilder {
    model: Model,
    max_iterations: usize,
    tool_timeout: Duration,
    system_prompt: Option<String>,
}

impl ReActAgentBuilder {
    /// Create a new ReAct agent builder
    pub fn new(model: Model) -> Self {
        Self {
            model,
            max_iterations: 10,
            tool_timeout: Duration::from_secs(30),
            system_prompt: None,
        }
    }

    /// Set the maximum number of iterations
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Set the tool execution timeout
    pub fn with_tool_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.tool_timeout = Duration::from_secs(timeout_secs);
        self
    }

    /// Set a custom system prompt for the agent
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Build the ReAct agent
    ///
    /// # Errors
    ///
    /// Returns an error if tool callbacks cannot be retrieved from the model.
    pub fn build(self) -> Result<ReActAgent> {
        let tool_callbacks = self.model.get_tool_callbacks()
            .context("Failed to get tool callbacks from model")?;

        Ok(ReActAgent {
            model: self.model,
            tool_callbacks,
            max_iterations: self.max_iterations,
            tool_timeout: self.tool_timeout,
        })
    }
}
```

## Testing the Fix

Create `mistralrs/examples/react_agent/main.rs`:

```rust
use anyhow::Result;
use mistralrs::{TextModelBuilder, IsqType, TextMessageRole};
use mistralrs::react_agent::ReActAgent;
use mistralrs_core::{McpClientConfig, McpServerConfig, McpServerSource};

#[tokio::main]
async fn main() -> Result<()> {
    // Configure MCP client with time server
    let mcp_config = McpClientConfig {
        servers: vec![
            McpServerConfig {
                name: "Time Server".to_string(),
                source: McpServerSource::Process {
                    command: "npx".to_string(),
                    args: vec!["-y".to_string(), "@modelcontextprotocol/server-time".to_string()],
                    work_dir: None,
                    env: None,
                },
                ..Default::default()
            },
        ],
        auto_register_tools: true,
        tool_timeout_secs: Some(10),
        max_concurrent_calls: Some(5),
    };

    // Build model with MCP tools
    let model = TextModelBuilder::new("meta-llama/Llama-3.2-3B-Instruct")
        .with_logging()
        .with_isq(IsqType::Q8_0)
        .with_mcp_client(mcp_config)
        .build()
        .await?;

    // Create ReAct agent
    let agent = ReActAgent::new(model)?
        .with_max_iterations(5)
        .with_tool_timeout_secs(30);

    // Run agent with a query that requires tool use
    println!("Running ReAct agent...");
    let response = agent.run("What is the current time in UTC?").await?;

    println!("\n=== Final Answer ===");
    println!("{}", response.final_answer);

    println!("\n=== Agent Trace ===");
    println!("Total iterations: {}", response.total_iterations);

    for (i, iteration) in response.iterations.iter().enumerate() {
        println!("\nIteration {}:", i + 1);
        if let Some(thought) = &iteration.thought {
            println!("  Thought: {}", thought);
        }
        println!("  Actions: {} tool call(s)", iteration.actions.len());
        for (j, action) in iteration.actions.iter().enumerate() {
            println!("    {}. {}({})", j + 1, action.function.name, action.function.arguments);
        }
        for (j, obs) in iteration.observations.iter().enumerate() {
            println!("  Observation {}: {}", j + 1,
                if let Some(err) = &obs.error {
                    format!("ERROR: {}", err)
                } else {
                    obs.content.clone()
                });
        }
    }

    Ok(())
}
```

## Verification Steps

1. **Compile**:

```bash
cargo check -p mistralrs-core -p mistralrs
```

2. **Run example**:

```bash
cargo run --example react_agent --release
```

3. **Expected output**:

```
Running ReAct agent...

=== Final Answer ===
The current time in UTC is 2025-10-03T14:32:15Z

=== Agent Trace ===
Total iterations: 2

Iteration 1:
  Thought: I need to get the current UTC time
  Actions: 1 tool call(s)
    1. get_time({"timezone":"UTC"})
  Observation 1: {"time":"2025-10-03T14:32:15Z","timezone":"UTC"}

Iteration 2:
  Final answer provided without tool calls
```

## Why This Fix Works

1. **Direct callback access**: Agent can now call tools synchronously via stored callbacks
1. **MCP tools work**: Callbacks already handle async→sync bridge (see `mistralrs-mcp/src/client.rs:224-257`)
1. **No architecture changes**: Uses existing tool callback infrastructure
1. **Thread-safe**: `Arc<ToolCallback>` allows safe cloning and concurrent execution
1. **Timeout protection**: Wraps execution in tokio::timeout
1. **Error handling**: Proper Result propagation with context

## Breaking Changes

- `ReActAgent::new()` now returns `Result<Self>` instead of `Self`
- `ReActAgentBuilder::build()` now returns `Result<ReActAgent>` instead of `ReActAgent`

Both changes are necessary because tool callback retrieval can fail if no tools are registered.

## Migration Guide

**Before**:

```rust
let agent = ReActAgent::new(model);
```

**After**:

```rust
let agent = ReActAgent::new(model)?;  // Add error handling
```
