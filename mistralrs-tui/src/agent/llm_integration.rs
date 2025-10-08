//! LLM integration for tool calling
//!
//! Provides utilities for integrating agent tools with LLM frameworks:
//! - Parsing tool calls from LLM responses
//! - Formatting tool results for LLM context
//! - Multi-turn conversation handling

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use super::discovery::ToolCatalog;
use super::toolkit::{ToolCall, ToolCallResult};

/// LLM tool call request (from model response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMToolCall {
    /// Tool function name
    pub name: String,
    /// Tool arguments as JSON
    pub arguments: JsonValue,
    /// Optional call ID for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// LLM tool call response (to send back to model)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMToolResponse {
    /// Tool call ID (if provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Tool name
    pub name: String,
    /// Tool execution result
    pub result: JsonValue,
    /// Whether execution was successful
    pub success: bool,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// OpenAI function calling format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunctionCall {
    pub name: String,
    pub arguments: String, // JSON string
}

/// OpenAI tool choice
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpenAIToolChoice {
    Auto,
    None,
    Required,
    Specific { function: OpenAIFunctionCall },
}

/// Anthropic tool use block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicToolUse {
    pub id: String,
    pub name: String,
    pub input: JsonValue,
}

/// LLM integration helper
pub struct LLMIntegration {
    catalog: ToolCatalog,
}

impl LLMIntegration {
    /// Create a new LLM integration with the tool catalog
    pub fn new(catalog: ToolCatalog) -> Self {
        Self { catalog }
    }

    /// Create with default tool catalog
    pub fn with_defaults() -> Self {
        Self::new(ToolCatalog::new())
    }

    /// Parse OpenAI function call format
    pub fn parse_openai_function(&self, function: &OpenAIFunctionCall) -> Result<LLMToolCall> {
        let arguments: JsonValue = serde_json::from_str(&function.arguments)
            .context("Failed to parse function arguments")?;

        Ok(LLMToolCall {
            name: function.name.clone(),
            arguments,
            id: None,
        })
    }

    /// Parse OpenAI tool calls from response
    pub fn parse_openai_tools(&self, tools: &[JsonValue]) -> Result<Vec<LLMToolCall>> {
        let mut calls = Vec::new();

        for tool in tools {
            if let Some(function) = tool.get("function") {
                let name = function
                    .get("name")
                    .and_then(|v| v.as_str())
                    .context("Missing function name")?
                    .to_string();

                let arguments_str = function
                    .get("arguments")
                    .and_then(|v| v.as_str())
                    .context("Missing function arguments")?;

                let arguments: JsonValue = serde_json::from_str(arguments_str)
                    .context("Failed to parse function arguments")?;

                let id = tool.get("id").and_then(|v| v.as_str()).map(String::from);

                calls.push(LLMToolCall {
                    name,
                    arguments,
                    id,
                });
            }
        }

        Ok(calls)
    }

    /// Parse Anthropic tool use blocks
    pub fn parse_anthropic_tools(&self, tool_uses: &[AnthropicToolUse]) -> Vec<LLMToolCall> {
        tool_uses
            .iter()
            .map(|tool_use| LLMToolCall {
                name: tool_use.name.clone(),
                arguments: tool_use.input.clone(),
                id: Some(tool_use.id.clone()),
            })
            .collect()
    }

    /// Format tool result for OpenAI
    pub fn format_openai_result(&self, call: &ToolCall) -> JsonValue {
        let result = call.result.as_ref();

        json!({
            "role": "tool",
            "tool_call_id": call.id.to_string(),
            "name": call.tool_name,
            "content": self.format_result_content(result)
        })
    }

    /// Format tool result for Anthropic
    pub fn format_anthropic_result(&self, call: &ToolCall) -> JsonValue {
        let result = call.result.as_ref();

        json!({
            "type": "tool_result",
            "tool_use_id": call.id.to_string(),
            "content": self.format_result_content(result),
            "is_error": result.map(|r| !r.success).unwrap_or(false)
        })
    }

    /// Format result content as string for LLM
    fn format_result_content(&self, result: Option<&ToolCallResult>) -> String {
        match result {
            Some(r) if r.success => {
                // Format successful result
                match &r.output {
                    JsonValue::String(s) => s.clone(),
                    JsonValue::Null => "No output".to_string(),
                    other => {
                        serde_json::to_string_pretty(other).unwrap_or_else(|_| "{}".to_string())
                    }
                }
            }
            Some(r) => {
                // Format error result
                format!("Error: {}", r.error.as_deref().unwrap_or("Unknown error"))
            }
            None => "Tool call pending".to_string(),
        }
    }

    /// Create system prompt with tool information
    pub fn create_system_prompt(&self) -> String {
        let tool_list: Vec<String> = self
            .catalog
            .tools()
            .iter()
            .map(|t| format!("- {}: {}", t.name, t.description))
            .collect();

        format!(
            "You are an AI assistant with access to the following tools:\n\n{}\n\n\
             Use these tools when appropriate to help the user. Call tools by their exact names \
             with the required parameters.",
            tool_list.join("\n")
        )
    }

    /// Build conversation messages with tool results
    pub fn build_conversation(
        &self,
        user_message: &str,
        tool_calls: &[ToolCall],
    ) -> Vec<JsonValue> {
        let mut messages = vec![json!({
            "role": "user",
            "content": user_message
        })];

        // Add tool results as assistant messages
        if !tool_calls.is_empty() {
            // Add assistant message with tool calls
            let tool_call_refs: Vec<JsonValue> = tool_calls
                .iter()
                .map(|call| {
                    json!({
                        "id": call.id.to_string(),
                        "type": "function",
                        "function": {
                            "name": call.tool_name,
                            "arguments": serde_json::to_string(&call.arguments).unwrap_or_default()
                        }
                    })
                })
                .collect();

            messages.push(json!({
                "role": "assistant",
                "content": null,
                "tool_calls": tool_call_refs
            }));

            // Add tool results
            for call in tool_calls {
                messages.push(self.format_openai_result(call));
            }
        }

        messages
    }

    /// Get tool catalog
    pub fn catalog(&self) -> &ToolCatalog {
        &self.catalog
    }
}

/// Helper to create OpenAI chat completion request with tools
pub fn create_openai_request(
    model: &str,
    messages: Vec<JsonValue>,
    tools: Vec<JsonValue>,
    temperature: Option<f32>,
) -> JsonValue {
    let mut request = json!({
        "model": model,
        "messages": messages,
        "tools": tools,
    });

    if let Some(temp) = temperature {
        request["temperature"] = json!(temp);
    }

    request
}

/// Helper to create Anthropic messages request with tools
pub fn create_anthropic_request(
    model: &str,
    messages: Vec<JsonValue>,
    tools: Vec<JsonValue>,
    max_tokens: u32,
) -> JsonValue {
    json!({
        "model": model,
        "messages": messages,
        "tools": tools,
        "max_tokens": max_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_integration_creation() {
        let integration = LLMIntegration::with_defaults();
        assert!(integration.catalog().tools().len() > 0);
    }

    #[test]
    fn test_parse_openai_function() {
        let integration = LLMIntegration::with_defaults();
        let function = OpenAIFunctionCall {
            name: "ls".to_string(),
            arguments: r#"{"path": "."}"#.to_string(),
        };

        let parsed = integration.parse_openai_function(&function).unwrap();
        assert_eq!(parsed.name, "ls");
        assert_eq!(parsed.arguments.get("path").unwrap(), ".");
    }

    #[test]
    fn test_system_prompt_generation() {
        let integration = LLMIntegration::with_defaults();
        let prompt = integration.create_system_prompt();
        assert!(prompt.contains("tools"));
        assert!(prompt.contains("ls"));
    }

    #[test]
    fn test_openai_request_creation() {
        let messages = vec![json!({"role": "user", "content": "Hello"})];
        let tools = vec![json!({"name": "ls", "description": "List files"})];

        let request = create_openai_request("gpt-4", messages, tools, Some(0.7));

        assert_eq!(request["model"], "gpt-4");
        // Use approximate comparison for floating point
        let temp = request["temperature"].as_f64().unwrap();
        assert!((temp - 0.7).abs() < 0.001);
        assert!(request["tools"].is_array());
    }
}
