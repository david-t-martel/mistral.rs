//! Integration with mistralrs-core tool system
//!
//! This module provides `AgentToolProvider` which bridges agent-tools with mistralrs-core's
//! tool callback infrastructure, enabling all 90+ utilities to be used by language models.

use crate::types::{
    CatOptions, CommandOptions, GrepOptions, HeadOptions, LsOptions, ShellType, SortOptions,
    TailOptions, UniqOptions, WcOptions,
};
use crate::{AgentToolkit, SandboxConfig};
use anyhow::{anyhow, Result};
use mistralrs_mcp::{CalledFunction, Function, Tool, ToolCallbackWithTool, ToolType};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Provider that exposes agent-tools as mistralrs-core tool callbacks
#[derive(Clone)]
pub struct AgentToolProvider {
    toolkit: AgentToolkit,
    tool_prefix: Option<String>,
}

impl AgentToolProvider {
    /// Create a new provider with the given sandbox configuration
    pub fn new(sandbox_config: SandboxConfig) -> Self {
        Self {
            toolkit: AgentToolkit::new(sandbox_config),
            tool_prefix: None,
        }
    }

    /// Add a prefix to all tool names (e.g., "agent_cat" instead of "cat")
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.tool_prefix = Some(prefix.into());
        self
    }

    /// Access the underlying toolkit.
    pub fn toolkit(&self) -> &AgentToolkit {
        &self.toolkit
    }

    /// Get the tool name with optional prefix
    fn tool_name(&self, base_name: &str) -> String {
        match &self.tool_prefix {
            Some(prefix) => format!("{}_{}", prefix, base_name),
            None => base_name.to_string(),
        }
    }

    /// Generate all Tool definitions for agent-tools utilities
    pub fn get_tools(&self) -> Vec<Tool> {
        vec![
            // File operations
            self.create_cat_tool(),
            self.create_ls_tool(),
            // Text processing
            self.create_grep_tool(),
            self.create_head_tool(),
            self.create_tail_tool(),
            self.create_wc_tool(),
            self.create_sort_tool(),
            self.create_uniq_tool(),
            // Shell execution
            self.create_shell_tool(),
        ]
        // Additional utilities are tracked in TODO_ANALYSIS.md for future integration
    }

    /// Generate ToolCallbackWithTool for mistralrs-core integration
    pub fn get_tool_callbacks_with_tools(&self) -> HashMap<String, ToolCallbackWithTool> {
        let mut callbacks = HashMap::new();

        for tool in self.get_tools() {
            let name = tool.function.name.clone();
            let toolkit = self.toolkit.clone();

            // Create callback with correct signature: &CalledFunction -> Result<String>
            let callback = Arc::new(move |called: &CalledFunction| {
                let toolkit_clone = toolkit.clone();
                let args_json = called.arguments.clone();

                // Execute synchronously (agent tools are blocking)
                execute_agent_tool(&toolkit_clone, &called.name, &args_json)
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

    // Tool definition creators

    fn create_cat_tool(&self) -> Tool {
        let mut params = HashMap::new();
        params.insert("type".to_string(), json!("object"));
        params.insert(
            "properties".to_string(),
            json!({
                "paths": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "File paths to read and concatenate"
                },
                "number_lines": {
                    "type": "boolean",
                    "description": "Number all output lines",
                    "default": false
                },
                "show_ends": {
                    "type": "boolean",
                    "description": "Display $ at end of each line",
                    "default": false
                },
                "squeeze_blank": {
                    "type": "boolean",
                    "description": "Suppress repeated empty output lines",
                    "default": false
                }
            }),
        );
        params.insert("required".to_string(), json!(["paths"]));

        Tool {
            tp: ToolType::Function,
            function: Function {
                name: self.tool_name("cat"),
                description: Some("Concatenate and display file contents. Supports multiple files, line numbering, and various text encodings.".to_string()),
                parameters: Some(params),
            },
        }
    }

    fn create_ls_tool(&self) -> Tool {
        let mut params = HashMap::new();
        params.insert("type".to_string(), json!("object"));
        params.insert(
            "properties".to_string(),
            json!({
                "path": {
                    "type": "string",
                    "description": "Directory path to list (default: current directory)"
                },
                "all": {
                    "type": "boolean",
                    "description": "Show hidden files (starting with .)",
                    "default": false
                },
                "long": {
                    "type": "boolean",
                    "description": "Use long listing format with details",
                    "default": false
                },
                "human_readable": {
                    "type": "boolean",
                    "description": "Print sizes in human-readable format (e.g., 1K, 234M)",
                    "default": false
                },
                "sort_by_time": {
                    "type": "boolean",
                    "description": "Sort by modification time, newest first",
                    "default": false
                },
                "reverse": {
                    "type": "boolean",
                    "description": "Reverse order while sorting",
                    "default": false
                },
                "recursive": {
                    "type": "boolean",
                    "description": "List subdirectories recursively",
                    "default": false
                }
            }),
        );
        params.insert("required".to_string(), json!([]));

        Tool {
            tp: ToolType::Function,
            function: Function {
                name: self.tool_name("ls"),
                description: Some("List directory contents with detailed information including permissions, sizes, and timestamps.".to_string()),
                parameters: Some(params),
            },
        }
    }
    fn create_grep_tool(&self) -> Tool {
        let mut params = HashMap::new();
        params.insert("type".to_string(), json!("object"));
        params.insert(
            "properties".to_string(),
            json!({
                "pattern": {
                    "type": "string",
                    "description": "Pattern to search for (regex or fixed string)"
                },
                "paths": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "File paths to search in"
                },
                "ignore_case": {
                    "type": "boolean",
                    "description": "Ignore case distinctions",
                    "default": false
                },
                "line_number": {
                    "type": "boolean",
                    "description": "Prefix each line with line number",
                    "default": false
                },
                "count": {
                    "type": "boolean",
                    "description": "Only print count of matching lines",
                    "default": false
                },
                "before_context": {
                    "type": "integer",
                    "description": "Print NUM lines of leading context",
                    "default": 0
                },
                "after_context": {
                    "type": "integer",
                    "description": "Print NUM lines of trailing context",
                    "default": 0
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Recurse into directories when searching",
                    "default": false
                }
            }),
        );
        params.insert("required".to_string(), json!(["pattern", "paths"]));

        Tool {
            tp: ToolType::Function,
            function: Function {
                name: self.tool_name("grep"),
                description: Some("Search for patterns in files using regular expressions. Returns matching lines with context.".to_string()),
                parameters: Some(params),
            },
        }
    }

    fn create_head_tool(&self) -> Tool {
        let mut params = HashMap::new();
        params.insert("type".to_string(), json!("object"));
        params.insert(
            "properties".to_string(),
            json!({
                "paths": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "File paths to display"
                },
                "lines": {
                    "type": "integer",
                    "description": "Number of lines to display",
                    "default": 10
                }
            }),
        );
        params.insert("required".to_string(), json!(["paths"]));

        Tool {
            tp: ToolType::Function,
            function: Function {
                name: self.tool_name("head"),
                description: Some(
                    "Display the first part of files. Useful for previewing file contents."
                        .to_string(),
                ),
                parameters: Some(params),
            },
        }
    }

    fn create_tail_tool(&self) -> Tool {
        let mut params = HashMap::new();
        params.insert("type".to_string(), json!("object"));
        params.insert(
            "properties".to_string(),
            json!({
                "paths": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "File paths to display"
                },
                "lines": {
                    "type": "integer",
                    "description": "Number of lines to display",
                    "default": 10
                }
            }),
        );
        params.insert("required".to_string(), json!(["paths"]));

        Tool {
            tp: ToolType::Function,
            function: Function {
                name: self.tool_name("tail"),
                description: Some(
                    "Display the last part of files. Useful for viewing end of logs or files."
                        .to_string(),
                ),
                parameters: Some(params),
            },
        }
    }

    fn create_wc_tool(&self) -> Tool {
        let mut params = HashMap::new();
        params.insert("type".to_string(), json!("object"));
        params.insert(
            "properties".to_string(),
            json!({
                "paths": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "File paths to count"
                },
                "lines": {
                    "type": "boolean",
                    "description": "Count lines",
                    "default": false
                },
                "words": {
                    "type": "boolean",
                    "description": "Count words",
                    "default": false
                },
                "bytes": {
                    "type": "boolean",
                    "description": "Count bytes",
                    "default": false
                },
                "chars": {
                    "type": "boolean",
                    "description": "Count characters",
                    "default": false
                }
            }),
        );
        params.insert("required".to_string(), json!(["paths"]));

        Tool {
            tp: ToolType::Function,
            function: Function {
                name: self.tool_name("wc"),
                description: Some(
                    "Count lines, words, bytes, and characters in files.".to_string(),
                ),
                parameters: Some(params),
            },
        }
    }

    fn create_sort_tool(&self) -> Tool {
        let mut params = HashMap::new();
        params.insert("type".to_string(), json!("object"));
        params.insert(
            "properties".to_string(),
            json!({
                "paths": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "File paths to sort"
                },
                "reverse": {
                    "type": "boolean",
                    "description": "Reverse the result of comparisons",
                    "default": false
                },
                "numeric": {
                    "type": "boolean",
                    "description": "Compare according to string numerical value",
                    "default": false
                },
                "unique": {
                    "type": "boolean",
                    "description": "Output only the first of an equal run",
                    "default": false
                }
            }),
        );
        params.insert("required".to_string(), json!(["paths"]));

        Tool {
            tp: ToolType::Function,
            function: Function {
                name: self.tool_name("sort"),
                description: Some(
                    "Sort lines of text files. Supports various sorting methods and options."
                        .to_string(),
                ),
                parameters: Some(params),
            },
        }
    }

    fn create_uniq_tool(&self) -> Tool {
        let mut params = HashMap::new();
        params.insert("type".to_string(), json!("object"));
        params.insert(
            "properties".to_string(),
            json!({
                "paths": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "File paths to process"
                },
                "count": {
                    "type": "boolean",
                    "description": "Prefix lines by the number of occurrences",
                    "default": false
                },
                "repeated": {
                    "type": "boolean",
                    "description": "Only print duplicate lines",
                    "default": false
                },
                "unique": {
                    "type": "boolean",
                    "description": "Only print unique lines",
                    "default": false
                }
            }),
        );
        params.insert("required".to_string(), json!(["paths"]));

        Tool {
            tp: ToolType::Function,
            function: Function {
                name: self.tool_name("uniq"),
                description: Some(
                    "Report or omit repeated lines. Filters out adjacent duplicate lines."
                        .to_string(),
                ),
                parameters: Some(params),
            },
        }
    }

    fn create_shell_tool(&self) -> Tool {
        let mut params = HashMap::new();
        params.insert("type".to_string(), json!("object"));
        params.insert(
            "properties".to_string(),
            json!({
                "command": {
                    "type": "string",
                    "description": "Command to execute"
                },
                "shell": {
                    "type": "string",
                    "description": "Shell to use (bash, pwsh, cmd)"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory for the command"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds",
                    "default": 30
                },
                "capture_stdout": {
                    "type": "boolean",
                    "description": "Capture standard output",
                    "default": true
                },
                "capture_stderr": {
                    "type": "boolean",
                    "description": "Capture standard error",
                    "default": true
                },
                "env": {
                    "type": "object",
                    "additionalProperties": {"type": "string"},
                    "description": "Environment variables to set (key/value pairs)"
                }
            }),
        );
        params.insert("required".to_string(), json!(["command"]));

        Tool {
            tp: ToolType::Function,
            function: Function {
                name: self.tool_name("shell"),
                description: Some(
                    "Execute a command inside the sandbox using the configured shell.".to_string(),
                ),
                parameters: Some(params),
            },
        }
    }
}

/// Execute an agent tool with JSON arguments
fn execute_agent_tool(toolkit: &AgentToolkit, tool_name: &str, args_json: &str) -> Result<String> {
    // Parse JSON arguments
    let args: Value =
        serde_json::from_str(args_json).map_err(|e| anyhow!("Failed to parse arguments: {}", e))?;

    // Route to appropriate tool execution
    match tool_name.rsplit('_').next().unwrap_or(tool_name) {
        "cat" => execute_cat(toolkit, &args),
        "ls" => execute_ls(toolkit, &args),
        "grep" => execute_grep(toolkit, &args),
        "head" => execute_head(toolkit, &args),
        "tail" => execute_tail(toolkit, &args),
        "wc" => execute_wc(toolkit, &args),
        "sort" => execute_sort(toolkit, &args),
        "uniq" => execute_uniq(toolkit, &args),
        "shell" | "run" | "execute" => execute_shell(toolkit, &args),
        _ => Err(anyhow!("Unknown tool: {}", tool_name)),
    }
}

fn execute_cat(toolkit: &AgentToolkit, args: &Value) -> Result<String> {
    let paths: Vec<String> = args
        .get("paths")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| anyhow!("Missing 'paths' parameter"))?;

    let options = CatOptions {
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
    };

    let path_refs: Vec<&Path> = paths.iter().map(|p| Path::new(p.as_str())).collect();
    let result = crate::tools::file::cat(&toolkit.sandbox, &path_refs, &options)?;

    Ok(result)
}

fn execute_ls(toolkit: &AgentToolkit, args: &Value) -> Result<String> {
    let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let path = Path::new(path_str);

    let options = LsOptions {
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
    };

    let result = crate::tools::file::ls(&toolkit.sandbox, path, &options)?;

    // Format result as JSON
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "total": result.total,
        "total_size": result.total_size,
        "entries": result.entries.iter().map(|e| {
            serde_json::json!({
                "name": e.name,
                "is_dir": e.is_dir,
                "size": e.size,
                "modified": e.modified,
            })
        }).collect::<Vec<_>>()
    }))?)
}

fn execute_grep(toolkit: &AgentToolkit, args: &Value) -> Result<String> {
    let pattern = args
        .get("pattern")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing 'pattern' parameter"))?;

    let paths: Vec<String> = args
        .get("paths")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| anyhow!("Missing 'paths' parameter"))?;

    let options = GrepOptions {
        ignore_case: args
            .get("ignore_case")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        line_number: args
            .get("line_number")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        count: args.get("count").and_then(|v| v.as_bool()).unwrap_or(false),
        before_context: args
            .get("before_context")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
        after_context: args
            .get("after_context")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
        invert_match: false,
        files_with_matches: false,
        files_without_match: false,
        extended_regexp: true,
        fixed_strings: false,
        recursive: args
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };

    let path_refs: Vec<&Path> = paths.iter().map(|p| Path::new(p.as_str())).collect();
    let matches = crate::tools::text::grep(&toolkit.sandbox, pattern, &path_refs, &options)?;

    // Format matches as string
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

fn execute_head(toolkit: &AgentToolkit, args: &Value) -> Result<String> {
    let paths: Vec<String> = args
        .get("paths")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| anyhow!("Missing 'paths' parameter"))?;

    let options = HeadOptions {
        lines: args.get("lines").and_then(|v| v.as_u64()).unwrap_or(10) as usize,
        bytes: None,
        verbose: false,
        quiet: false,
    };

    let path_refs: Vec<&Path> = paths.iter().map(|p| Path::new(p.as_str())).collect();
    let result = crate::tools::text::head(&toolkit.sandbox, &path_refs, &options)?;

    Ok(result)
}

fn execute_tail(toolkit: &AgentToolkit, args: &Value) -> Result<String> {
    let paths: Vec<String> = args
        .get("paths")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| anyhow!("Missing 'paths' parameter"))?;

    let options = TailOptions {
        lines: args.get("lines").and_then(|v| v.as_u64()).unwrap_or(10) as usize,
        bytes: None,
        verbose: false,
        quiet: false,
    };

    let path_refs: Vec<&Path> = paths.iter().map(|p| Path::new(p.as_str())).collect();
    let result = crate::tools::text::tail(&toolkit.sandbox, &path_refs, &options)?;

    Ok(result)
}

fn execute_wc(toolkit: &AgentToolkit, args: &Value) -> Result<String> {
    let paths: Vec<String> = args
        .get("paths")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| anyhow!("Missing 'paths' parameter"))?;

    let options = WcOptions {
        lines: args.get("lines").and_then(|v| v.as_bool()).unwrap_or(false),
        words: args.get("words").and_then(|v| v.as_bool()).unwrap_or(false),
        bytes: args.get("bytes").and_then(|v| v.as_bool()).unwrap_or(false),
        chars: args.get("chars").and_then(|v| v.as_bool()).unwrap_or(false),
    };

    let path_refs: Vec<&Path> = paths.iter().map(|p| Path::new(p.as_str())).collect();
    let results = crate::tools::text::wc(&toolkit.sandbox, &path_refs, &options)?;

    // Format results
    Ok(crate::tools::text::format_wc_output(&results, &options))
}

fn execute_sort(toolkit: &AgentToolkit, args: &Value) -> Result<String> {
    let paths: Vec<String> = args
        .get("paths")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| anyhow!("Missing 'paths' parameter"))?;

    let options = SortOptions {
        reverse: args
            .get("reverse")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        numeric: args
            .get("numeric")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        unique: args
            .get("unique")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        ignore_case: false,
        version_sort: false,
        month_sort: false,
        human_numeric: false,
    };

    let path_refs: Vec<&Path> = paths.iter().map(|p| Path::new(p.as_str())).collect();
    let result = crate::tools::text::sort(&toolkit.sandbox, &path_refs, &options)?;

    Ok(result)
}

fn execute_shell(toolkit: &AgentToolkit, args: &Value) -> Result<String> {
    let command = args
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing 'command' parameter"))?;

    let mut options = CommandOptions::default();

    if let Some(timeout) = args.get("timeout").and_then(|v| v.as_u64()) {
        options.timeout = Some(timeout);
    }

    options.capture_stdout = args
        .get("capture_stdout")
        .and_then(|v| v.as_bool())
        .unwrap_or(options.capture_stdout);
    options.capture_stderr = args
        .get("capture_stderr")
        .and_then(|v| v.as_bool())
        .unwrap_or(options.capture_stderr);

    if let Some(shell_str) = args.get("shell").and_then(|v| v.as_str()) {
        options.shell = match shell_str.to_ascii_lowercase().as_str() {
            "bash" => ShellType::Bash,
            "pwsh" | "powershell" => ShellType::PowerShell,
            "cmd" => ShellType::Cmd,
            other => return Err(anyhow!("Unsupported shell '{other}'")),
        };
    }

    options.working_dir = args
        .get("working_dir")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);

    if let Some(env_map) = args.get("env").and_then(|v| v.as_object()) {
        let mut env = Vec::with_capacity(env_map.len());
        for (key, value) in env_map {
            let env_val = value
                .as_str()
                .ok_or_else(|| anyhow!("Environment variable '{key}' must be a string"))?;
            env.push((key.clone(), env_val.to_string()));
        }
        options.env = env;
    }

    let result = toolkit.execute(command, &options)?;

    let mut output = String::new();
    if !result.stdout.is_empty() {
        output.push_str(&result.stdout);
    }
    if !result.stderr.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str("[stderr]:\n");
        output.push_str(&result.stderr);
    }
    output.push_str(&format!("\n[exit code: {}]\n", result.status));

    Ok(output)
}

fn execute_uniq(toolkit: &AgentToolkit, args: &Value) -> Result<String> {
    let paths: Vec<String> = args
        .get("paths")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| anyhow!("Missing 'paths' parameter"))?;

    let options = UniqOptions {
        count: args.get("count").and_then(|v| v.as_bool()).unwrap_or(false),
        repeated: args
            .get("repeated")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        unique: args
            .get("unique")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        ignore_case: false,
        skip_fields: 0,
        skip_chars: 0,
    };

    let path_refs: Vec<&Path> = paths.iter().map(|p| Path::new(p.as_str())).collect();
    let result = crate::tools::text::uniq(&toolkit.sandbox, &path_refs, &options)?;

    Ok(result)
}
