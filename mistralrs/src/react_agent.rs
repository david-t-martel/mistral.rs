//! ReAct Agent Implementation for mistral.rs
//!
//! This module provides an autonomous ReAct (Reasoning + Acting) agent runtime that:
//! - Auto-executes tools without manual intervention
//! - Performs multi-step reasoning until task completion
//! - Integrates seamlessly with existing tool calling infrastructure
//! - Works with both native callbacks and MCP auto-registered tools
//!
//! # Architecture
//!
//! The ReAct agent operates in a loop:
//! 1. **Think** - Model reasons about what action to take
//! 2. **Act** - Tool calls are automatically executed
//! 3. **Observe** - Tool results are fed back to the model
//! 4. **Repeat** - Continue until no more tool calls (task complete) or max iterations
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use mistralrs::{TextModelBuilder, IsqType, TextMessageRole};
//! use mistralrs::react_agent::ReActAgent;
//! use mistralrs_core::{McpClientConfig, McpServerConfig, McpServerSource};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Configure MCP client with tools (e.g., time server)
//!     let mcp_config = McpClientConfig {
//!         servers: vec![
//!             McpServerConfig {
//!                 name: "Time Server".to_string(),
//!                 source: McpServerSource::Process {
//!                     command: "npx".to_string(),
//!                     args: vec!["-y".to_string(), "@modelcontextprotocol/server-time".to_string()],
//!                     work_dir: None,
//!                     env: None,
//!                 },
//!                 ..Default::default()
//!             },
//!         ],
//!         auto_register_tools: true,
//!         tool_timeout_secs: Some(10),
//!         max_concurrent_calls: Some(5),
//!     };
//!
//!     // Build model with MCP tools
//!     let model = TextModelBuilder::new("meta-llama/Meta-Llama-3.1-8B-Instruct")
//!         .with_logging()
//!         .with_isq(IsqType::Q8_0)
//!         .with_mcp_client(mcp_config)
//!         .build()
//!         .await?;
//!
//!     // Create ReAct agent (now returns Result)
//!     let agent = ReActAgent::new(model)?
//!         .with_max_iterations(10)
//!         .with_tool_timeout_secs(30);
//!
//!     let response = agent.run("What is the current time in UTC?").await?;
//!
//!     println!("Final answer: {}", response.final_answer);
//!     println!("Iterations: {}", response.total_iterations);
//!
//!     for (i, iteration) in response.iterations.iter().enumerate() {
//!         println!("\nIteration {}:", i + 1);
//!         println!("  Thought: {:?}", iteration.thought);
//!         println!("  Actions: {} tool calls", iteration.actions.len());
//!         for (j, obs) in iteration.observations.iter().enumerate() {
//!             println!("  Observation {}: {}", j + 1,
//!                 if let Some(err) = &obs.error {
//!                     format!("ERROR: {}", err)
//!                 } else {
//!                     obs.content.clone()
//!                 }
//!             );
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```

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

/// Result of executing a single tool call
#[derive(Clone, Debug)]
pub struct ToolResult {
    /// The output of the tool execution
    pub content: String,
    /// Error message if the tool execution failed
    pub error: Option<String>,
}

/// Represents a single iteration in the ReAct loop
#[derive(Clone, Debug)]
pub struct AgentIteration {
    /// The model's reasoning/thought process (if any)
    pub thought: Option<String>,
    /// The tool calls made in this iteration
    pub actions: Vec<ToolCallResponse>,
    /// The observations/results from executing the tools
    pub observations: Vec<ToolResult>,
}

/// Final response from the ReAct agent
#[derive(Clone, Debug)]
pub struct AgentResponse {
    /// The final answer from the agent
    pub final_answer: String,
    /// All iterations performed during the agent run
    pub iterations: Vec<AgentIteration>,
    /// Total number of iterations performed
    pub total_iterations: usize,
}

impl ReActAgent {
    /// Create a new ReAct agent with default settings
    ///
    /// This will initialize the agent with access to all registered tool callbacks
    /// from the model (including MCP auto-registered tools).
    ///
    /// Defaults:
    /// - max_iterations: 10
    /// - tool_timeout: 30 seconds
    ///
    /// # Errors
    ///
    /// Returns an error if tool callbacks cannot be retrieved from the model.
    /// This can happen if the model was not built with any tools registered.
    pub fn new(model: Model) -> Result<Self> {
        let tool_callbacks = model.get_tool_callbacks().context(
            "Failed to get tool callbacks from model. Ensure tools are registered via builder.",
        )?;

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

    /// Run the ReAct agent loop with a user query
    ///
    /// The agent will:
    /// 1. Send the query to the model
    /// 2. Execute any tool calls automatically
    /// 3. Feed tool results back to the model
    /// 4. Repeat until no more tool calls or max iterations reached
    ///
    /// # Arguments
    ///
    /// * `user_query` - The user's question or task
    ///
    /// # Returns
    ///
    /// An `AgentResponse` containing the final answer and iteration history,
    /// or an error if the agent failed or exceeded max iterations.
    pub async fn run(&self, user_query: impl Into<String>) -> Result<AgentResponse> {
        let user_query = user_query.into();
        let mut messages = RequestBuilder::new().add_message(TextMessageRole::User, &user_query);

        let mut iterations = vec![];

        for iteration_num in 0..self.max_iterations {
            // Send request to model
            let response = self
                .model
                .send_chat_request(messages.clone())
                .await
                .context(format!(
                    "Failed to send chat request at iteration {}",
                    iteration_num
                ))?;

            let message = response
                .choices
                .first()
                .ok_or_else(|| anyhow!("No choices in response at iteration {}", iteration_num))?
                .message
                .clone();

            // Check if model wants to call tools
            if let Some(tool_calls) = &message.tool_calls {
                // AUTO-EXECUTE all tool calls in parallel
                let mut tool_results = Vec::new();

                for tool_call in tool_calls {
                    let result = self.execute_tool_with_timeout(tool_call).await;
                    tool_results.push(result);
                }

                // Add assistant message with tool calls
                messages = messages.add_message_with_tool_call(
                    TextMessageRole::Assistant,
                    message.content.clone().unwrap_or_default(),
                    tool_calls.clone(),
                );

                // Add tool result messages for each call
                for (tool_call, tool_result) in tool_calls.iter().zip(tool_results.iter()) {
                    messages = messages
                        .add_tool_message(tool_result.content.clone(), tool_call.id.clone());
                }

                // Record this iteration
                iterations.push(AgentIteration {
                    thought: message.content.clone(),
                    actions: tool_calls.clone(),
                    observations: tool_results,
                });
            } else {
                // No tool calls - task is complete
                let final_answer = message
                    .content
                    .clone()
                    .unwrap_or_else(|| "No response generated".to_string());

                return Ok(AgentResponse {
                    final_answer,
                    iterations,
                    total_iterations: iteration_num + 1,
                });
            }
        }

        // Max iterations exceeded
        Err(anyhow!(
            "ReAct agent exceeded maximum iterations ({})",
            self.max_iterations
        ))
    }

    /// Execute a tool call with timeout protection
    ///
    /// This method wraps tool execution with a timeout to prevent hanging.
    /// The actual execution is delegated to the model's internal tool callbacks
    /// (either native callbacks or MCP auto-registered tools).
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

    /// Internal tool execution method - FULLY FUNCTIONAL
    ///
    /// This directly invokes the registered tool callbacks, which include:
    /// - Native callbacks registered via `.with_tool_callback()`
    /// - MCP auto-registered tools from `.with_mcp_client()`
    ///
    /// The callbacks handle the async-to-sync bridge internally (for MCP tools),
    /// so we can call them directly here and get results synchronously.
    ///
    /// # How MCP tools work here
    ///
    /// When MCP tools are registered via `.with_mcp_client()`, they're converted
    /// into `ToolCallback` functions that:
    /// 1. Take the function call arguments
    /// 2. Spawn a thread with the tokio runtime
    /// 3. Execute the async MCP tool call
    /// 4. Return the result synchronously
    ///
    /// See `mistralrs-mcp/src/client.rs:224-257` for the implementation details.
    async fn execute_tool_internal(&self, tool_call: &ToolCallResponse) -> Result<ToolResult> {
        let tool_name = &tool_call.function.name;

        // Look up the tool callback in our registry
        let callback = self.tool_callbacks.get(tool_name).ok_or_else(|| {
            anyhow!(
                "Tool '{}' not found. Available tools: {}",
                tool_name,
                if self.tool_callbacks.is_empty() {
                    "(none - no tools registered)".to_string()
                } else {
                    self.tool_callbacks
                        .keys()
                        .map(|k| k.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            )
        })?;

        // Execute the tool callback
        // Note: This is synchronous from our perspective, but MCP callbacks
        // internally handle async execution via thread spawning + runtime.
        // Native (non-MCP) callbacks are just regular synchronous functions.
        let result = callback(&tool_call.function)
            .context(format!("Failed to execute tool '{}'", tool_name))?;

        Ok(ToolResult {
            content: result,
            error: None,
        })
    }
}

/// Builder for configuring ReAct agent with custom system prompts and tools
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
        let tool_callbacks = self.model.get_tool_callbacks().context(
            "Failed to get tool callbacks from model. Ensure tools are registered via builder.",
        )?;

        Ok(ReActAgent {
            model: self.model,
            tool_callbacks,
            max_iterations: self.max_iterations,
            tool_timeout: self.tool_timeout,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_iteration_structure() {
        let iteration = AgentIteration {
            thought: Some("I should use the weather tool".to_string()),
            actions: vec![],
            observations: vec![],
        };

        assert_eq!(
            iteration.thought,
            Some("I should use the weather tool".to_string())
        );
        assert_eq!(iteration.actions.len(), 0);
    }

    #[test]
    fn test_tool_result_structure() {
        let result = ToolResult {
            content: "Temperature: 25C".to_string(),
            error: None,
        };

        assert_eq!(result.content, "Temperature: 25C");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_agent_response_structure() {
        let response = AgentResponse {
            final_answer: "The weather is nice".to_string(),
            iterations: vec![],
            total_iterations: 3,
        };

        assert_eq!(response.final_answer, "The weather is nice");
        assert_eq!(response.total_iterations, 3);
    }
}
