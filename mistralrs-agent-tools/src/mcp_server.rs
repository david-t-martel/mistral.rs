// MCP Server implementation for agent-tools
//
// This module exposes all agent-tools utilities via the Model Context Protocol (MCP),
// making them accessible to any MCP client including Claude Desktop, IDEs, and other AI assistants.
//
// The server supports:
// - Tool discovery via tools/list endpoint
// - Tool execution via tools/call endpoint
// - JSON-RPC 2.0 protocol
// - Stdio transport for local communication
// - HTTP/WebSocket transports (optional)

use crate::types::{
    CatOptions, CommandOptions, GrepOptions, HeadOptions, LsOptions, ShellType, SortOptions,
    TailOptions, UniqOptions, WcOptions,
};
use crate::AgentToolkit;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map as JsonMap, Value};
use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// MCP JSON-RPC 2.0 request structure
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

/// MCP JSON-RPC 2.0 response structure
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// MCP JSON-RPC error structure
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// MCP tool schema for tool discovery
#[derive(Debug, Serialize)]
struct McpToolSchema {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

impl McpToolSchema {
    fn new(name: String, description: String, input_schema: Value) -> Self {
        Self {
            name,
            description,
            input_schema,
        }
    }
}

/// MCP Server that exposes agent-tools via Model Context Protocol
pub struct McpServer {
    toolkit: Arc<AgentToolkit>,
    tool_prefix: Option<String>,
}

impl McpServer {
    /// Create a new MCP server with the given toolkit
    pub fn new(toolkit: Arc<AgentToolkit>, tool_prefix: Option<String>) -> Self {
        Self {
            toolkit,
            tool_prefix,
        }
    }

    /// Get the prefixed tool name
    fn get_tool_name(&self, base_name: &str) -> String {
        match &self.tool_prefix {
            Some(prefix) => format!("{}_{}", prefix, base_name),
            None => base_name.to_string(),
        }
    }

    /// List all available tools
    fn list_tools(&self) -> Value {
        let tools = self.tool_schemas();
        json!({
            "tools": tools
        })
    }

    fn tool_schemas(&self) -> Vec<McpToolSchema> {
        vec![
            McpToolSchema::new(
                self.get_tool_name("cat"),
                "Display file contents with syntax highlighting and line numbers".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "File paths to display"
                        },
                        "number_lines": {
                            "type": "boolean",
                            "description": "Show line numbers"
                        },
                        "show_ends": {
                            "type": "boolean",
                            "description": "Display $ at end of each line"
                        },
                        "squeeze_blank": {
                            "type": "boolean",
                            "description": "Suppress repeated empty output lines"
                        }
                    },
                    "required": ["paths"]
                }),
            ),
            McpToolSchema::new(
                self.get_tool_name("ls"),
                "List directory contents with detailed information".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Directory path to list"
                        },
                        "all": {
                            "type": "boolean",
                            "description": "Include hidden files"
                        },
                        "long": {
                            "type": "boolean",
                            "description": "Use long listing format"
                        },
                        "human_readable": {
                            "type": "boolean",
                            "description": "Human-readable file sizes"
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "List subdirectories recursively"
                        },
                        "sort_by_time": {
                            "type": "boolean",
                            "description": "Sort entries by modification time"
                        },
                        "reverse": {
                            "type": "boolean",
                            "description": "Reverse the sort order"
                        }
                    },
                    "required": ["path"]
                }),
            ),
            McpToolSchema::new(
                self.get_tool_name("grep"),
                "Search for patterns in files using regex".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Search pattern (regex)"
                        },
                        "paths": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "File or directory paths to search"
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "Search recursively"
                        },
                        "ignore_case": {
                            "type": "boolean",
                            "description": "Case-insensitive search"
                        },
                        "line_number": {
                            "type": "boolean",
                            "description": "Show line numbers"
                        },
                        "count": {
                            "type": "boolean",
                            "description": "Only print the count of matching lines"
                        },
                        "before_context": {
                            "type": "integer",
                            "description": "Number of leading context lines"
                        },
                        "after_context": {
                            "type": "integer",
                            "description": "Number of trailing context lines"
                        }
                    },
                    "required": ["pattern", "paths"]
                }),
            ),
            McpToolSchema::new(
                self.get_tool_name("head"),
                "Display first lines of a file".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "File paths"
                        },
                        "lines": {
                            "type": "integer",
                            "description": "Number of lines to display (default: 10)"
                        },
                        "bytes": {
                            "type": "integer",
                            "description": "Number of bytes to display"
                        },
                        "verbose": {
                            "type": "boolean",
                            "description": "Print headers with file names"
                        },
                        "quiet": {
                            "type": "boolean",
                            "description": "Suppress headers"
                        }
                    },
                    "required": ["paths"]
                }),
            ),
            McpToolSchema::new(
                self.get_tool_name("tail"),
                "Display last lines of a file".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "File paths"
                        },
                        "lines": {
                            "type": "integer",
                            "description": "Number of lines to display (default: 10)"
                        },
                        "bytes": {
                            "type": "integer",
                            "description": "Number of bytes to display"
                        },
                        "verbose": {
                            "type": "boolean",
                            "description": "Print headers with file names"
                        },
                        "quiet": {
                            "type": "boolean",
                            "description": "Suppress headers"
                        }
                    },
                    "required": ["paths"]
                }),
            ),
            McpToolSchema::new(
                self.get_tool_name("wc"),
                "Count lines, words, and characters in files".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "File paths to analyze"
                        },
                        "lines": {
                            "type": "boolean",
                            "description": "Count lines only"
                        },
                        "words": {
                            "type": "boolean",
                            "description": "Count words only"
                        },
                        "chars": {
                            "type": "boolean",
                            "description": "Count characters only"
                        }
                    },
                    "required": ["paths"]
                }),
            ),
            McpToolSchema::new(
                self.get_tool_name("sort"),
                "Sort lines of text files".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "File paths to sort"
                        },
                        "reverse": {
                            "type": "boolean",
                            "description": "Sort in reverse order"
                        },
                        "numeric": {
                            "type": "boolean",
                            "description": "Numeric sort"
                        },
                        "unique": {
                            "type": "boolean",
                            "description": "Output only unique lines"
                        }
                    },
                    "required": ["paths"]
                }),
            ),
            McpToolSchema::new(
                self.get_tool_name("uniq"),
                "Report or filter out repeated lines".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "File paths"
                        },
                        "count": {
                            "type": "boolean",
                            "description": "Prefix lines with occurrence count"
                        },
                        "repeated": {
                            "type": "boolean",
                            "description": "Only show repeated lines"
                        },
                        "unique": {
                            "type": "boolean",
                            "description": "Only show unique lines"
                        },
                        "ignore_case": {
                            "type": "boolean",
                            "description": "Compare lines case-insensitively"
                        },
                        "skip_fields": {
                            "type": "integer",
                            "description": "Skip initial fields when comparing"
                        },
                        "skip_chars": {
                            "type": "integer",
                            "description": "Skip initial characters when comparing"
                        }
                    },
                    "required": ["paths"]
                }),
            ),
            McpToolSchema::new(
                self.get_tool_name("shell"),
                "Execute a shell command inside the sandbox".to_string(),
                json!({
                    "type": "object",
                    "properties": {
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
                            "description": "Environment variables to set"
                        }
                    },
                    "required": ["command"]
                }),
            ),
            McpToolSchema::new(
                self.get_tool_name("execute"),
                "Execute a shell command inside the sandbox".to_string(),
                json!({
                    "type": "object",
                    "properties": {
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
                            "description": "Environment variables to set"
                        }
                    },
                    "required": ["command"]
                }),
            ),
        ]
    }

    /// Execute a tool call
    fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        let base_name = match &self.tool_prefix {
            Some(prefix) => tool_name
                .strip_prefix(&format!("{}_", prefix))
                .unwrap_or(tool_name),
            None => tool_name,
        };

        let args_map = match arguments {
            Value::Null => JsonMap::new(),
            Value::Object(map) => map,
            other => {
                return Err(anyhow!(
                    "Expected JSON object for tool arguments, received {}",
                    other
                ))
            }
        };

        let response = match base_name {
            "cat" => {
                let paths = Self::parse_path_array(&args_map, "paths")?;
                let options = CatOptions {
                    number_lines: Self::bool_arg(&args_map, "number_lines", false),
                    show_ends: Self::bool_arg(&args_map, "show_ends", false),
                    squeeze_blank: Self::bool_arg(&args_map, "squeeze_blank", false),
                };
                let path_refs = Self::path_refs(&paths);
                let output = self.toolkit.cat(&path_refs, &options)?;
                Self::text_response(output)
            }
            "ls" => {
                let path = Self::string_arg(&args_map, "path").unwrap_or_else(|| ".".to_string());
                let options = LsOptions {
                    all: Self::bool_arg(&args_map, "all", false),
                    long: Self::bool_arg(&args_map, "long", false),
                    human_readable: Self::bool_arg(&args_map, "human_readable", false),
                    recursive: Self::bool_arg(&args_map, "recursive", false),
                    sort_by_time: Self::bool_arg(&args_map, "sort_by_time", false),
                    reverse: Self::bool_arg(&args_map, "reverse", false),
                };
                let listing = self.toolkit.ls(Path::new(&path), &options)?;
                let listing_json = json!({
                    "total": listing.total,
                    "total_size": listing.total_size,
                    "entries": listing.entries.iter().map(|entry| {
                        json!({
                            "name": entry.name,
                            "is_dir": entry.is_dir,
                            "size": entry.size,
                            "modified": entry.modified,
                        })
                    }).collect::<Vec<_>>()
                });
                let text = serde_json::to_string_pretty(&listing_json)?;
                Self::text_response(text)
            }
            "grep" => {
                let pattern = Self::string_required(&args_map, "pattern")?;
                let paths = Self::parse_path_array(&args_map, "paths")?;
                let options = GrepOptions {
                    ignore_case: Self::bool_arg(&args_map, "ignore_case", true),
                    line_number: Self::bool_arg(&args_map, "line_number", true),
                    count: Self::bool_arg(&args_map, "count", false),
                    before_context: Self::usize_arg(&args_map, "before_context", 0),
                    after_context: Self::usize_arg(&args_map, "after_context", 0),
                    recursive: Self::bool_arg(&args_map, "recursive", false),
                    invert_match: false,
                    files_with_matches: false,
                    files_without_match: false,
                    extended_regexp: true,
                    fixed_strings: false,
                };
                let path_refs = Self::path_refs(&paths);
                let matches = self.toolkit.grep(&pattern, &path_refs, &options)?;
                let text = if options.count {
                    let mut counts = BTreeMap::new();
                    for m in &matches {
                        *counts.entry(&m.path).or_insert(0usize) += 1;
                    }
                    counts
                        .into_iter()
                        .map(|(path, count)| format!("{}: {} matches", path, count))
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    matches
                        .into_iter()
                        .map(|m| {
                            if options.line_number {
                                format!("{}:{}: {}", m.path, m.line_number, m.line)
                            } else {
                                format!("{}: {}", m.path, m.line)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                Self::text_response(text)
            }
            "head" => {
                let paths = Self::parse_path_array(&args_map, "paths")?;
                let options = HeadOptions {
                    lines: Self::usize_arg(&args_map, "lines", 10),
                    bytes: args_map
                        .get("bytes")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as usize),
                    verbose: Self::bool_arg(&args_map, "verbose", false),
                    quiet: Self::bool_arg(&args_map, "quiet", false),
                };
                let path_refs = Self::path_refs(&paths);
                let output = self.toolkit.head(&path_refs, &options)?;
                Self::text_response(output)
            }
            "tail" => {
                let paths = Self::parse_path_array(&args_map, "paths")?;
                let options = TailOptions {
                    lines: Self::usize_arg(&args_map, "lines", 10),
                    bytes: args_map
                        .get("bytes")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as usize),
                    verbose: Self::bool_arg(&args_map, "verbose", false),
                    quiet: Self::bool_arg(&args_map, "quiet", false),
                };
                let path_refs = Self::path_refs(&paths);
                let output = self.toolkit.tail(&path_refs, &options)?;
                Self::text_response(output)
            }
            "wc" => {
                let paths = Self::parse_path_array(&args_map, "paths")?;
                let options = WcOptions {
                    lines: Self::bool_arg(&args_map, "lines", true),
                    words: Self::bool_arg(&args_map, "words", true),
                    bytes: Self::bool_arg(&args_map, "bytes", false),
                    chars: Self::bool_arg(&args_map, "chars", false),
                };
                let path_refs = Self::path_refs(&paths);
                let results = self.toolkit.wc(&path_refs, &options)?;
                let text = results
                    .into_iter()
                    .map(|(path, res)| {
                        format!(
                            "{}: {} lines, {} words, {} bytes, {} chars",
                            path, res.lines, res.words, res.bytes, res.chars
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Self::text_response(text)
            }
            "sort" => {
                let paths = Self::parse_path_array(&args_map, "paths")?;
                let options = SortOptions {
                    reverse: Self::bool_arg(&args_map, "reverse", false),
                    numeric: Self::bool_arg(&args_map, "numeric", false),
                    unique: Self::bool_arg(&args_map, "unique", false),
                    ..Default::default()
                };
                let path_refs = Self::path_refs(&paths);
                let output = self.toolkit.sort(&path_refs, &options)?;
                Self::text_response(output)
            }
            "uniq" => {
                let paths = Self::parse_path_array(&args_map, "paths")?;
                let options = UniqOptions {
                    count: Self::bool_arg(&args_map, "count", false),
                    repeated: Self::bool_arg(&args_map, "repeated", false),
                    unique: Self::bool_arg(&args_map, "unique", false),
                    ignore_case: Self::bool_arg(&args_map, "ignore_case", false),
                    skip_fields: Self::usize_arg(&args_map, "skip_fields", 0),
                    skip_chars: Self::usize_arg(&args_map, "skip_chars", 0),
                };
                let path_refs = Self::path_refs(&paths);
                let output = self.toolkit.uniq(&path_refs, &options)?;
                Self::text_response(output)
            }
            "shell" | "execute" => {
                let command = Self::string_required(&args_map, "command")?;
                let mut options = CommandOptions::default();
                if let Some(shell) = Self::string_arg(&args_map, "shell") {
                    options.shell = match shell.to_ascii_lowercase().as_str() {
                        "bash" => ShellType::Bash,
                        "pwsh" | "powershell" => ShellType::PowerShell,
                        "cmd" => ShellType::Cmd,
                        other => return Err(anyhow!("Unsupported shell '{other}'")),
                    };
                }
                if let Some(dir) = Self::string_arg(&args_map, "working_dir") {
                    options.working_dir = Some(PathBuf::from(dir));
                }
                options.timeout = args_map.get("timeout").and_then(|v| v.as_u64());
                options.capture_stdout = Self::bool_arg(&args_map, "capture_stdout", true);
                options.capture_stderr = Self::bool_arg(&args_map, "capture_stderr", true);
                if let Some(env) = args_map.get("env").and_then(|v| v.as_object()) {
                    let mut env_pairs = Vec::with_capacity(env.len());
                    for (key, value) in env {
                        let val = value.as_str().ok_or_else(|| {
                            anyhow!("Environment variable '{key}' must be a string")
                        })?;
                        env_pairs.push((key.clone(), val.to_string()));
                    }
                    options.env = env_pairs;
                }

                let result = self.toolkit.execute(&command, &options)?;
                let mut text = String::new();
                if !result.stdout.is_empty() {
                    text.push_str(&result.stdout);
                }
                if !result.stderr.is_empty() {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str("[stderr]:\n");
                    text.push_str(&result.stderr);
                }
                text.push_str(&format!("\n[exit code: {}]\n", result.status));
                Self::text_response(text)
            }
            _ => return Err(anyhow!("Unknown tool: {}", base_name)),
        };

        Ok(response)
    }

    fn bool_arg(args: &JsonMap<String, Value>, key: &str, default: bool) -> bool {
        args.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
    }

    fn usize_arg(args: &JsonMap<String, Value>, key: &str, default: usize) -> usize {
        args.get(key)
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(default)
    }

    fn string_arg(args: &JsonMap<String, Value>, key: &str) -> Option<String> {
        args.get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn string_required(args: &JsonMap<String, Value>, key: &str) -> Result<String> {
        Self::string_arg(args, key).ok_or_else(|| anyhow!("Missing '{key}' argument"))
    }

    fn parse_path_array(args: &JsonMap<String, Value>, key: &str) -> Result<Vec<PathBuf>> {
        let value = args
            .get(key)
            .ok_or_else(|| anyhow!("Missing '{key}' argument"))?;
        match value {
            Value::Array(items) => {
                let mut paths = Vec::with_capacity(items.len());
                for entry in items {
                    let path = entry
                        .as_str()
                        .ok_or_else(|| anyhow!("Expected string in '{key}' array"))?;
                    paths.push(PathBuf::from(path));
                }
                if paths.is_empty() {
                    Err(anyhow!("At least one path must be provided for '{key}'"))
                } else {
                    Ok(paths)
                }
            }
            Value::String(path) => Ok(vec![PathBuf::from(path)]),
            _ => Err(anyhow!(
                "Expected string or string array for '{key}', received {}",
                value
            )),
        }
    }

    fn path_refs(paths: &[PathBuf]) -> Vec<&Path> {
        paths.iter().map(|p| p.as_path()).collect()
    }

    fn text_response(text: String) -> Value {
        json!({
            "content": [{
                "type": "text",
                "text": text
            }]
        })
    }

    /// Handle an MCP request
    fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let JsonRpcRequest {
            jsonrpc,
            id,
            method,
            params,
        } = request;

        if jsonrpc.trim() != "2.0" {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32600,
                    message: format!("Unsupported JSON-RPC version '{jsonrpc}'"),
                    data: Some(json!({
                        "supportedVersion": "2.0",
                        "receivedVersion": jsonrpc,
                    })),
                }),
            };
        }

        let result: Result<serde_json::Value> = match method.as_str() {
            "initialize" => {
                // MCP initialization handshake
                Ok(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "agent-tools-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }))
            }
            "tools/list" => {
                // List available tools
                Ok(self.list_tools())
            }
            "tools/call" => {
                // Execute a tool
                let empty_params = json!({});
                let params = params.as_ref().unwrap_or(&empty_params);
                match params["name"].as_str() {
                    Some(tool_name) => {
                        let arguments = params["arguments"].clone();
                        self.call_tool(tool_name, arguments)
                    }
                    None => Err(anyhow::anyhow!("Missing tool name")),
                }
            }
            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        };

        match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(value),
                error: None,
            },
            Err(err) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: err.to_string(),
                    data: None,
                }),
            },
        }
    }

    /// Run the MCP server on stdio (for process transport)
    pub fn run_stdio(&self) -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            let line = line.context("Failed to read from stdin")?;

            // Parse JSON-RPC request
            let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
                Ok(request) => self.handle_request(request),
                Err(err) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Failed to parse JSON-RPC request: {err}"),
                        data: None,
                    }),
                },
            };

            // Write response
            let response_json = serde_json::to_string(&response)?;
            writeln!(stdout, "{}", response_json)?;
            stdout.flush()?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name_prefix() {
        let toolkit = Arc::new(AgentToolkit::with_defaults());
        let server = McpServer::new(toolkit, Some("agent".to_string()));

        assert_eq!(server.get_tool_name("cat"), "agent_cat");
        assert_eq!(server.get_tool_name("ls"), "agent_ls");

        let server_no_prefix = McpServer::new(Arc::new(AgentToolkit::with_defaults()), None);
        assert_eq!(server_no_prefix.get_tool_name("cat"), "cat");
    }

    #[test]
    fn test_list_tools() {
        let toolkit = Arc::new(AgentToolkit::with_defaults());
        let server = McpServer::new(toolkit, None);

        let tools = server.list_tools();
        let tools_array = tools["tools"].as_array().unwrap();

        assert_eq!(tools_array.len(), 10);
        assert!(tools_array
            .iter()
            .any(|t| t["name"].as_str() == Some("cat")));
        assert!(tools_array
            .iter()
            .any(|t| t["name"].as_str() == Some("grep")));
    }

    #[test]
    fn test_mcp_ls() {
        let toolkit = Arc::new(AgentToolkit::with_defaults());
        let server = McpServer::new(toolkit, None);

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "ls",
                "arguments": {
                    "path": "."
                }
            }
        });

        let response = server.handle_request(serde_json::from_value(request).unwrap());

        assert!(response.error.is_none());
        let result = response.result.unwrap();
        let content = result["content"][0]["text"].as_str().unwrap();
        assert!(content.contains("Cargo.toml"));
    }

    #[test]
    fn test_mcp_cat() {
        let toolkit = Arc::new(AgentToolkit::with_defaults());
        let server = McpServer::new(toolkit, None);

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "cat",
                "arguments": {
                    "paths": ["Cargo.toml"]
                }
            }
        });

        let response = server.handle_request(serde_json::from_value(request).unwrap());

        assert!(response.error.is_none());
        let result = response.result.unwrap();
        let content = result["content"][0]["text"].as_str().unwrap();
        assert!(content.contains("[package]"));
    }
}
