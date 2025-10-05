//! Tool execution engine for agent mode
//!
//! This module provides the execution infrastructure for running agent tools:
//! - Async tool invocation with timeout support
//! - Result capture and serialization
//! - Session state management
//! - Event notifications for UI updates

use anyhow::{Context, Result};
use chrono::Utc;
use mistralrs_agent_tools::{
    AgentToolkit, CatOptions, CommandOptions, GrepOptions, HeadOptions, LsOptions, SortOptions,
    TailOptions, UniqOptions, WcOptions,
};
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use uuid::Uuid;

use super::events::{EventBus, ExecutionEvent};
use super::toolkit::{ToolCall, ToolCallResult};

/// Tool execution engine
///
/// Manages the lifecycle of tool executions including:
/// - Argument parsing and validation
/// - Async execution with timeout
/// - Result capture and formatting
/// - Error handling
#[derive(Debug, Clone)]
pub struct ToolExecutor {
    /// Reference to the agent toolkit
    toolkit: AgentToolkit,
    /// Default timeout for tool execution (in seconds)
    default_timeout: u64,
    /// Event bus for broadcasting execution events
    event_bus: Option<EventBus>,
}

impl ToolExecutor {
    /// Create a new tool executor with the given toolkit
    pub fn new(toolkit: AgentToolkit) -> Self {
        Self {
            toolkit,
            default_timeout: 30, // 30 seconds default
            event_bus: None,
        }
    }

    /// Create a new tool executor with event bus support
    pub fn with_events(toolkit: AgentToolkit, event_bus: EventBus) -> Self {
        Self {
            toolkit,
            default_timeout: 30,
            event_bus: Some(event_bus),
        }
    }

    /// Set the default timeout for tool executions
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.default_timeout = timeout_secs;
        self
    }

    /// Execute a tool by name with the given arguments
    ///
    /// This is the main entry point for tool execution. It:
    /// 1. Parses and validates arguments
    /// 2. Executes the tool with timeout
    /// 3. Captures results and timing
    /// 4. Returns a complete ToolCallResult
    pub async fn execute(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
        timeout_override: Option<u64>,
    ) -> Result<ToolCallResult> {
        let call_id = Uuid::new_v4();
        let start = Instant::now();

        // Emit started event
        if let Some(ref bus) = self.event_bus {
            bus.emit(ExecutionEvent::started(call_id, tool_name));
        }

        // Determine timeout
        let timeout_secs = timeout_override.unwrap_or(self.default_timeout);
        let timeout_duration = Duration::from_secs(timeout_secs);

        // Execute with timeout
        let result = timeout(timeout_duration, self.execute_tool(tool_name, arguments))
            .await
            .context("Tool execution timeout")?;

        let duration = start.elapsed();

        // Convert result to ToolCallResult and emit appropriate event
        let tool_result = match result {
            Ok(output) => {
                let tool_result = ToolCallResult {
                    success: true,
                    output: serde_json::Value::String(output.clone()),
                    error: None,
                    duration,
                };

                // Emit completed event
                if let Some(ref bus) = self.event_bus {
                    bus.emit(ExecutionEvent::completed(
                        call_id,
                        tool_name,
                        tool_result.clone(),
                    ));
                }

                tool_result
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Emit failed event
                if let Some(ref bus) = self.event_bus {
                    bus.emit(ExecutionEvent::failed(call_id, tool_name, &error_msg));
                }

                ToolCallResult {
                    success: false,
                    output: serde_json::Value::Null,
                    error: Some(error_msg),
                    duration,
                }
            }
        };

        Ok(tool_result)
    }

    /// Execute a specific tool (internal implementation)
    async fn execute_tool(&self, tool_name: &str, arguments: serde_json::Value) -> Result<String> {
        // Spawn blocking task since agent tools are synchronous
        let toolkit = self.toolkit.clone();
        let tool_name = tool_name.to_string();
        let args = arguments.clone();

        tokio::task::spawn_blocking(move || execute_tool_blocking(&toolkit, &tool_name, &args))
            .await
            .context("Tool execution task panicked")?
    }

    /// Create a complete ToolCall record from execution
    pub fn create_tool_call(
        &self,
        tool_name: String,
        arguments: serde_json::Value,
        session_id: Option<Uuid>,
    ) -> ToolCall {
        ToolCall {
            id: Uuid::new_v4(),
            tool_name,
            arguments,
            result: None,
            timestamp: Utc::now(),
            session_id,
        }
    }
    /// Update a ToolCall with execution result
    pub fn complete_tool_call(&self, mut tool_call: ToolCall, result: ToolCallResult) -> ToolCall {
        tool_call.result = Some(result);
        tool_call
    }
}

/// Execute a tool in blocking mode (called from spawn_blocking)
fn execute_tool_blocking(
    toolkit: &AgentToolkit,
    tool_name: &str,
    arguments: &serde_json::Value,
) -> Result<String> {
    match tool_name {
        // File operations
        "ls" => execute_ls(toolkit, arguments),
        "cat" => execute_cat(toolkit, arguments),

        // Text operations
        "head" => execute_head(toolkit, arguments),
        "tail" => execute_tail(toolkit, arguments),
        "grep" => execute_grep(toolkit, arguments),
        "wc" => execute_wc(toolkit, arguments),
        "sort" => execute_sort(toolkit, arguments),
        "uniq" => execute_uniq(toolkit, arguments),

        // Shell execution
        "execute" | "shell" | "run" => execute_shell(toolkit, arguments),

        // Unknown tool
        _ => anyhow::bail!("Unknown tool: {}", tool_name),
    }
}

// Tool-specific execution functions

fn execute_ls(toolkit: &AgentToolkit, args: &serde_json::Value) -> Result<String> {
    let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let path = Path::new(path_str);

    let options = parse_ls_options(args)?;
    let result = toolkit
        .ls(path, &options)
        .map_err(|e| anyhow::anyhow!("ls failed: {}", e))?;

    // Format output
    let mut output = String::new();
    for entry in &result.entries {
        output.push_str(&entry.name);
        output.push('\n');
    }

    Ok(output)
}

fn execute_cat(toolkit: &AgentToolkit, args: &serde_json::Value) -> Result<String> {
    let paths_arr = args
        .get("paths")
        .and_then(|v| v.as_array())
        .context("Missing or invalid 'paths' argument")?;

    let paths: Vec<_> = paths_arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(Path::new)
        .collect();

    let paths_refs: Vec<&Path> = paths.iter().map(|p| p.as_ref()).collect();
    let options = parse_cat_options(args)?;

    toolkit
        .cat(&paths_refs, &options)
        .map_err(|e| anyhow::anyhow!("cat failed: {}", e))
}

fn execute_head(toolkit: &AgentToolkit, args: &serde_json::Value) -> Result<String> {
    let paths_arr = args
        .get("paths")
        .and_then(|v| v.as_array())
        .context("Missing or invalid 'paths' argument")?;

    let paths: Vec<_> = paths_arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(Path::new)
        .collect();

    let paths_refs: Vec<&Path> = paths.iter().map(|p| p.as_ref()).collect();
    let options = parse_head_options(args)?;

    toolkit
        .head(&paths_refs, &options)
        .map_err(|e| anyhow::anyhow!("head failed: {}", e))
}

fn execute_tail(toolkit: &AgentToolkit, args: &serde_json::Value) -> Result<String> {
    let paths_arr = args
        .get("paths")
        .and_then(|v| v.as_array())
        .context("Missing or invalid 'paths' argument")?;

    let paths: Vec<_> = paths_arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(Path::new)
        .collect();

    let paths_refs: Vec<&Path> = paths.iter().map(|p| p.as_ref()).collect();
    let options = parse_tail_options(args)?;

    toolkit
        .tail(&paths_refs, &options)
        .map_err(|e| anyhow::anyhow!("tail failed: {}", e))
}

fn execute_grep(toolkit: &AgentToolkit, args: &serde_json::Value) -> Result<String> {
    let pattern = args
        .get("pattern")
        .and_then(|v| v.as_str())
        .context("Missing or invalid 'pattern' argument")?;

    let paths_arr = args
        .get("paths")
        .and_then(|v| v.as_array())
        .context("Missing or invalid 'paths' argument")?;

    let paths: Vec<_> = paths_arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(Path::new)
        .collect();

    let paths_refs: Vec<&Path> = paths.iter().map(|p| p.as_ref()).collect();
    let options = parse_grep_options(args)?;

    let matches = toolkit
        .grep(pattern, &paths_refs, &options)
        .map_err(|e| anyhow::anyhow!("grep failed: {}", e))?;

    // Format matches
    let mut output = String::new();
    for m in matches {
        if options.line_number {
            output.push_str(&format!("{}:{}:{}", m.path, m.line_number, m.line));
        } else {
            output.push_str(&format!("{}:{}", m.path, m.line));
        }
        output.push('\n');
    }

    Ok(output)
}

fn execute_wc(toolkit: &AgentToolkit, args: &serde_json::Value) -> Result<String> {
    let paths_arr = args
        .get("paths")
        .and_then(|v| v.as_array())
        .context("Missing or invalid 'paths' argument")?;

    let paths: Vec<_> = paths_arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(Path::new)
        .collect();

    let paths_refs: Vec<&Path> = paths.iter().map(|p| p.as_ref()).collect();
    let options = parse_wc_options(args)?;

    let results = toolkit
        .wc(&paths_refs, &options)
        .map_err(|e| anyhow::anyhow!("wc failed: {}", e))?;

    // Format output
    let mut output = String::new();
    for (path, result) in results {
        output.push_str(&format!(
            "{} {} {} {}\n",
            result.lines, result.words, result.bytes, path
        ));
    }

    Ok(output)
}

fn execute_sort(toolkit: &AgentToolkit, args: &serde_json::Value) -> Result<String> {
    let paths_arr = args
        .get("paths")
        .and_then(|v| v.as_array())
        .context("Missing or invalid 'paths' argument")?;

    let paths: Vec<_> = paths_arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(Path::new)
        .collect();

    let paths_refs: Vec<&Path> = paths.iter().map(|p| p.as_ref()).collect();
    let options = parse_sort_options(args)?;

    toolkit
        .sort(&paths_refs, &options)
        .map_err(|e| anyhow::anyhow!("sort failed: {}", e))
}

fn execute_uniq(toolkit: &AgentToolkit, args: &serde_json::Value) -> Result<String> {
    let paths_arr = args
        .get("paths")
        .and_then(|v| v.as_array())
        .context("Missing or invalid 'paths' argument")?;

    let paths: Vec<_> = paths_arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(Path::new)
        .collect();

    let paths_refs: Vec<&Path> = paths.iter().map(|p| p.as_ref()).collect();
    let options = parse_uniq_options(args)?;

    toolkit
        .uniq(&paths_refs, &options)
        .map_err(|e| anyhow::anyhow!("uniq failed: {}", e))
}

fn execute_shell(toolkit: &AgentToolkit, args: &serde_json::Value) -> Result<String> {
    let command = args
        .get("command")
        .and_then(|v| v.as_str())
        .context("Missing or invalid 'command' argument")?;

    let options = parse_command_options(args)?;

    let result = toolkit
        .execute(command, &options)
        .map_err(|e| anyhow::anyhow!("Shell execution failed: {}", e))?;

    // Format output
    let mut output = String::new();
    output.push_str(&result.stdout);
    if !result.stderr.is_empty() {
        output.push_str("\n[stderr]:\n");
        output.push_str(&result.stderr);
    }
    output.push_str(&format!("\n[exit code: {}]\n", result.status));

    Ok(output)
}

// Option parsing helper functions

fn parse_ls_options(args: &serde_json::Value) -> Result<LsOptions> {
    Ok(LsOptions {
        all: args.get("all").and_then(|v| v.as_bool()).unwrap_or(false),
        long: args.get("long").and_then(|v| v.as_bool()).unwrap_or(false),
        human_readable: args
            .get("human_readable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        recursive: args
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        sort_by_time: args
            .get("sort_by_time")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        reverse: args
            .get("reverse")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

fn parse_cat_options(args: &serde_json::Value) -> Result<CatOptions> {
    Ok(CatOptions {
        number_lines: args
            .get("number_lines")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        show_ends: args
            .get("show_ends")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        squeeze_blank: args
            .get("squeeze_blank")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

fn parse_head_options(args: &serde_json::Value) -> Result<HeadOptions> {
    Ok(HeadOptions {
        lines: args.get("lines").and_then(|v| v.as_u64()).unwrap_or(10) as usize,
        bytes: args
            .get("bytes")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize),
        verbose: args
            .get("verbose")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        quiet: args.get("quiet").and_then(|v| v.as_bool()).unwrap_or(false),
    })
}

fn parse_tail_options(args: &serde_json::Value) -> Result<TailOptions> {
    Ok(TailOptions {
        lines: args.get("lines").and_then(|v| v.as_u64()).unwrap_or(10) as usize,
        bytes: args
            .get("bytes")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize),
        verbose: args
            .get("verbose")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        quiet: args.get("quiet").and_then(|v| v.as_bool()).unwrap_or(false),
    })
}

fn parse_grep_options(args: &serde_json::Value) -> Result<GrepOptions> {
    Ok(GrepOptions {
        ignore_case: args
            .get("ignore_case")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        invert_match: args
            .get("invert_match")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        line_number: args
            .get("line_number")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        count: args.get("count").and_then(|v| v.as_bool()).unwrap_or(false),
        files_with_matches: args
            .get("files_with_matches")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        files_without_match: args
            .get("files_without_match")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        before_context: args
            .get("before_context")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
        after_context: args
            .get("after_context")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
        extended_regexp: args
            .get("extended_regexp")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        ..Default::default()
    })
}

fn parse_wc_options(args: &serde_json::Value) -> Result<WcOptions> {
    Ok(WcOptions {
        lines: args.get("lines").and_then(|v| v.as_bool()).unwrap_or(false),
        words: args.get("words").and_then(|v| v.as_bool()).unwrap_or(false),
        bytes: args.get("bytes").and_then(|v| v.as_bool()).unwrap_or(false),
        chars: args.get("chars").and_then(|v| v.as_bool()).unwrap_or(false),
    })
}

fn parse_sort_options(args: &serde_json::Value) -> Result<SortOptions> {
    Ok(SortOptions {
        numeric: args
            .get("numeric")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        reverse: args
            .get("reverse")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        unique: args
            .get("unique")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        ..Default::default()
    })
}

fn parse_uniq_options(args: &serde_json::Value) -> Result<UniqOptions> {
    Ok(UniqOptions {
        count: args.get("count").and_then(|v| v.as_bool()).unwrap_or(false),
        repeated: args
            .get("repeated")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        unique: args
            .get("unique")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        ignore_case: args
            .get("ignore_case")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        skip_fields: args
            .get("skip_fields")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
        skip_chars: args.get("skip_chars").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
    })
}

fn parse_command_options(args: &serde_json::Value) -> Result<CommandOptions> {
    let timeout = args.get("timeout").and_then(|v| v.as_u64());

    Ok(CommandOptions {
        timeout,
        capture_stdout: args
            .get("capture_stdout")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        capture_stderr: args
            .get("capture_stderr")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_execute_ls() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

        let toolkit = AgentToolkit::with_root(temp_dir.path().to_path_buf());
        let executor = ToolExecutor::new(toolkit);

        let args = serde_json::json!({
            "path": temp_dir.path().to_str().unwrap(),
            "all": false
        });

        let result = executor.execute("ls", args, None).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("test.txt"));
    }

    #[tokio::test]
    async fn test_execute_cat() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, World!").unwrap();

        let toolkit = AgentToolkit::with_root(temp_dir.path().to_path_buf());
        let executor = ToolExecutor::new(toolkit);

        let args = serde_json::json!({
            "paths": [file_path.to_str().unwrap()]
        });

        let result = executor.execute("cat", args, None).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_timeout() {
        let toolkit = AgentToolkit::with_defaults();
        let executor = ToolExecutor::new(toolkit).with_timeout(1); // 1 second

        // This should timeout
        let args = serde_json::json!({
            "command": "sleep 10"
        });

        let result = executor.execute("shell", args, None).await;
        assert!(result.is_err());
    }
}
