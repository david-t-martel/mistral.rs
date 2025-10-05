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

use crate::AgentToolkit;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
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
        json!({
            "tools": [
                {
                    "name": self.get_tool_name("cat"),
                    "description": "Display file contents with syntax highlighting and line numbers",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the file to display"
                            },
                            "number_lines": {
                                "type": "boolean",
                                "description": "Show line numbers"
                            },
                            "show_ends": {
                                "type": "boolean",
                                "description": "Display $ at end of each line"
                            }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": self.get_tool_name("ls"),
                    "description": "List directory contents with detailed information",
                    "inputSchema": {
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
                            }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": self.get_tool_name("grep"),
                    "description": "Search for patterns in files using regex",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "pattern": {
                                "type": "string",
                                "description": "Search pattern (regex)"
                            },
                            "path": {
                                "type": "string",
                                "description": "File or directory to search"
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
                            }
                        },
                        "required": ["pattern", "path"]
                    }
                },
                {
                    "name": self.get_tool_name("head"),
                    "description": "Display first lines of a file",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "File path"
                            },
                            "lines": {
                                "type": "integer",
                                "description": "Number of lines to display (default: 10)"
                            }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": self.get_tool_name("tail"),
                    "description": "Display last lines of a file",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "File path"
                            },
                            "lines": {
                                "type": "integer",
                                "description": "Number of lines to display (default: 10)"
                            }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": self.get_tool_name("wc"),
                    "description": "Count lines, words, and characters in files",
                    "inputSchema": {
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
                    }
                },
                {
                    "name": self.get_tool_name("sort"),
                    "description": "Sort lines of text files",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "File path to sort"
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
                        "required": ["path"]
                    }
                },
                {
                    "name": self.get_tool_name("uniq"),
                    "description": "Report or filter out repeated lines",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "File path"
                            },
                            "count": {
                                "type": "boolean",
                                "description": "Prefix lines with occurrence count"
                            },
                            "repeated": {
                                "type": "boolean",
                                "description": "Only show repeated lines"
                            }
                        },
                        "required": ["path"]
                    }
                }
            ]
        })
    }

    /// Execute a tool call
    fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        // Strip prefix if present
        let base_name = match &self.tool_prefix {
            Some(prefix) => tool_name
                .strip_prefix(&format!("{}_", prefix))
                .unwrap_or(tool_name),
            None => tool_name,
        };

        // Convert arguments to HashMap for tool execution
        let args: HashMap<String, Value> =
            serde_json::from_value(arguments).context("Failed to parse tool arguments")?;

        // Execute the tool
        let result = match base_name {
            "cat" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("Missing 'path' argument")?;
                let number_lines = args
                    .get("number_lines")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let show_ends = args
                    .get("show_ends")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let opts = crate::CatOptions {
                    number_lines,
                    show_ends,
                    ..Default::default()
                };

                let path_obj = std::path::Path::new(path);
                self.toolkit.cat(&[&path_obj], &opts)?
            }
            "ls" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("Missing 'path' argument")?;
                let all = args.get("all").and_then(|v| v.as_bool()).unwrap_or(false);
                let long = args.get("long").and_then(|v| v.as_bool()).unwrap_or(false);
                let human_readable = args
                    .get("human_readable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let opts = crate::LsOptions {
                    all,
                    long,
                    human_readable,
                    ..Default::default()
                };

                let path_obj = std::path::Path::new(path);
                let result = self.toolkit.ls(path_obj, &opts)?;

                // Format LsResult as a string
                result
                    .entries
                    .iter()
                    .map(|e| {
                        if long {
                            format!("{} {} {}", if e.is_dir { "d" } else { "-" }, e.size, e.name)
                        } else {
                            e.name.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            "grep" => {
                let pattern = args
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .context("Missing 'pattern' argument")?;
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("Missing 'path' argument")?;
                let recursive = args
                    .get("recursive")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let ignore_case = args
                    .get("ignore_case")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let line_number = args
                    .get("line_number")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let opts = crate::GrepOptions {
                    recursive,
                    ignore_case,
                    line_number,
                    ..Default::default()
                };

                let path_obj = std::path::Path::new(path);
                let matches = self.toolkit.grep(pattern, &[&path_obj], &opts)?;

                // Format grep matches as text
                matches
                    .iter()
                    .map(|m| {
                        if m.line_number > 0 {
                            format!("{}:{}: {}", m.path, m.line_number, m.line)
                        } else {
                            format!("{}: {}", m.path, m.line)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            "head" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("Missing 'path' argument")?;
                let lines = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

                let opts = crate::HeadOptions {
                    lines,
                    ..Default::default()
                };

                let path_obj = std::path::Path::new(path);
                self.toolkit.head(&[&path_obj], &opts)?
            }
            "tail" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("Missing 'path' argument")?;
                let lines = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

                let opts = crate::TailOptions {
                    lines,
                    ..Default::default()
                };

                let path_obj = std::path::Path::new(path);
                self.toolkit.tail(&[&path_obj], &opts)?
            }
            "wc" => {
                let paths: Vec<String> = args
                    .get("paths")
                    .and_then(|v| v.as_array())
                    .context("Missing 'paths' argument")?
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();

                if paths.is_empty() {
                    anyhow::bail!("At least one path required for wc");
                }

                let lines = args.get("lines").and_then(|v| v.as_bool()).unwrap_or(false);
                let words = args.get("words").and_then(|v| v.as_bool()).unwrap_or(false);
                let chars = args.get("chars").and_then(|v| v.as_bool()).unwrap_or(false);

                let opts = crate::WcOptions {
                    lines,
                    words,
                    chars,
                    ..Default::default()
                };

                // Convert String paths to Path references
                let path_objs: Vec<std::path::PathBuf> =
                    paths.iter().map(std::path::PathBuf::from).collect();
                let path_refs: Vec<&std::path::Path> =
                    path_objs.iter().map(|p| p.as_path()).collect();

                // wc returns Vec<(String, WcResult)> - format as single string
                let results = self.toolkit.wc(&path_refs, &opts)?;
                results
                    .iter()
                    .map(|(path, res)| {
                        format!(
                            "{}: {} lines, {} words, {} bytes, {} chars",
                            path, res.lines, res.words, res.bytes, res.chars
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            "sort" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("Missing 'path' argument")?;
                let reverse = args
                    .get("reverse")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let numeric = args
                    .get("numeric")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let unique = args
                    .get("unique")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let opts = crate::SortOptions {
                    reverse,
                    numeric,
                    unique,
                    ..Default::default()
                };

                let path_obj = std::path::Path::new(path);
                self.toolkit.sort(&[&path_obj], &opts)?
            }
            "uniq" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("Missing 'path' argument")?;
                let count = args.get("count").and_then(|v| v.as_bool()).unwrap_or(false);
                let repeated = args
                    .get("repeated")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let opts = crate::UniqOptions {
                    count,
                    repeated,
                    ..Default::default()
                };

                let path_obj = std::path::Path::new(path);
                self.toolkit.uniq(&[&path_obj], &opts)?
            }
            _ => anyhow::bail!("Unknown tool: {}", base_name),
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": result
            }]
        }))
    }

    /// Handle an MCP request
    fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result: Result<serde_json::Value> = match request.method.as_str() {
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
                let params = request.params.as_ref().unwrap_or(&empty_params);
                match params["name"].as_str() {
                    Some(tool_name) => {
                        let arguments = params["arguments"].clone();
                        self.call_tool(tool_name, arguments)
                    }
                    None => Err(anyhow::anyhow!("Missing tool name")),
                }
            }
            _ => Err(anyhow::anyhow!("Unknown method: {}", request.method)),
        };

        match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(value),
                error: None,
            },
            Err(err) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
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
            let request: JsonRpcRequest =
                serde_json::from_str(&line).context("Failed to parse JSON-RPC request")?;

            // Handle request
            let response = self.handle_request(request);

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
        let toolkit = Arc::new(AgentToolkit::default());
        let server = McpServer::new(toolkit, Some("agent".to_string()));

        assert_eq!(server.get_tool_name("cat"), "agent_cat");
        assert_eq!(server.get_tool_name("ls"), "agent_ls");

        let server_no_prefix = McpServer::new(Arc::new(AgentToolkit::default()), None);
        assert_eq!(server_no_prefix.get_tool_name("cat"), "cat");
    }

    #[test]
    fn test_list_tools() {
        let toolkit = Arc::new(AgentToolkit::default());
        let server = McpServer::new(toolkit, None);

        let tools = server.list_tools();
        let tools_array = tools["tools"].as_array().unwrap();

        assert_eq!(tools_array.len(), 8);
        assert!(tools_array
            .iter()
            .any(|t| t["name"].as_str() == Some("cat")));
        assert!(tools_array
            .iter()
            .any(|t| t["name"].as_str() == Some("grep")));
    }
}
