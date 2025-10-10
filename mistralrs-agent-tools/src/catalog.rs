//! Shared tool catalog and metadata definitions.
//!
//! This module provides a single source of truth for tool definitions that can
//! be consumed by the server, TUI, and MCP integrations. Definitions here are
//! expressed in JSON-Schema compatible structures so they can be surfaced to
//! LLM tool-calling APIs directly.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

/// Tool definition for LLM function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name (used in function calls).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Parameter schema (JSON Schema format).
    pub parameters: JsonValue,
    /// Example usage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<ToolExample>>,
}

/// Example tool usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub description: String,
    pub arguments: JsonValue,
}

/// Tool catalog containing all available tools.
#[derive(Debug, Clone, Default)]
pub struct ToolCatalog {
    tools: Vec<ToolDefinition>,
}

impl ToolCatalog {
    /// Create a new tool catalog with default tools.
    pub fn new() -> Self {
        Self {
            tools: vec![
                Self::define_ls(),
                Self::define_cat(),
                Self::define_head(),
                Self::define_tail(),
                Self::define_grep(),
                Self::define_wc(),
                Self::define_sort(),
                Self::define_uniq(),
                Self::define_shell(),
                Self::define_shell_alias(),
            ],
        }
    }

    fn define_shell_alias() -> ToolDefinition {
        let mut shell = Self::define_shell();
        shell.name = "execute".to_string();
        shell
    }

    /// Return the underlying tool definitions.
    pub fn tools(&self) -> &[ToolDefinition] {
        &self.tools
    }

    /// Get a tool definition by name.
    pub fn get_tool(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.iter().find(|t| t.name == name)
    }

    /// Convert to OpenAI function calling format.
    pub fn to_openai_functions(&self) -> Vec<JsonValue> {
        self.tools
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters
                })
            })
            .collect()
    }

    /// Convert to Anthropic tool format.
    pub fn to_anthropic_tools(&self) -> Vec<JsonValue> {
        self.tools
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.parameters
                })
            })
            .collect()
    }

    // Tool definitions -----------------------------------------------------

    fn define_ls() -> ToolDefinition {
        ToolDefinition {
            name: "ls".to_string(),
            description:
                "List directory contents. Shows files and directories in the specified path."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path to list (default: current directory)"
                    },
                    "all": {
                        "type": "boolean",
                        "description": "Show hidden files (starting with .)"
                    },
                    "long": {
                        "type": "boolean",
                        "description": "Show detailed information (permissions, size, date)"
                    },
                    "human_readable": {
                        "type": "boolean",
                        "description": "Show human-readable file sizes (KB, MB, GB)"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "List subdirectories recursively"
                    }
                }
            }),
            examples: Some(vec![
                ToolExample {
                    description: "List current directory".to_string(),
                    arguments: json!({"path": "."}),
                },
                ToolExample {
                    description: "List with details and hidden files".to_string(),
                    arguments: json!({"path": ".", "all": true, "long": true}),
                },
            ]),
        }
    }

    fn define_cat() -> ToolDefinition {
        ToolDefinition {
            name: "cat".to_string(),
            description: "Display file contents. Can concatenate multiple files.".to_string(),
            parameters: json!({
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
            examples: Some(vec![ToolExample {
                description: "Display a file".to_string(),
                arguments: json!({"paths": ["README.md"]}),
            }]),
        }
    }

    fn define_head() -> ToolDefinition {
        ToolDefinition {
            name: "head".to_string(),
            description: "Display the first lines of files (default: 10 lines).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "File paths to read"
                    },
                    "lines": {
                        "type": "integer",
                        "description": "Number of lines to show"
                    }
                },
                "required": ["paths"]
            }),
            examples: None,
        }
    }

    fn define_tail() -> ToolDefinition {
        ToolDefinition {
            name: "tail".to_string(),
            description: "Display the last lines of files (default: 10 lines).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "File paths to read"
                    },
                    "lines": {
                        "type": "integer",
                        "description": "Number of lines to show"
                    }
                },
                "required": ["paths"]
            }),
            examples: None,
        }
    }

    fn define_grep() -> ToolDefinition {
        ToolDefinition {
            name: "grep".to_string(),
            description: "Search for patterns in files using regular expressions.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for"
                    },
                    "paths": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Files to search"
                    },
                    "ignore_case": {
                        "type": "boolean",
                        "description": "Case-insensitive search"
                    },
                    "line_number": {
                        "type": "boolean",
                        "description": "Show line numbers"
                    },
                    "invert_match": {
                        "type": "boolean",
                        "description": "Select non-matching lines"
                    }
                },
                "required": ["pattern", "paths"]
            }),
            examples: Some(vec![ToolExample {
                description: "Search for TODO comments".to_string(),
                arguments: json!({"pattern": "TODO", "paths": ["src/lib.rs"]}),
            }]),
        }
    }

    fn define_wc() -> ToolDefinition {
        ToolDefinition {
            name: "wc".to_string(),
            description: "Count lines, words, bytes, and characters.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Files to count"
                    },
                    "lines": {
                        "type": "boolean",
                        "description": "Count lines"
                    },
                    "words": {
                        "type": "boolean",
                        "description": "Count words"
                    },
                    "bytes": {
                        "type": "boolean",
                        "description": "Count bytes"
                    },
                    "chars": {
                        "type": "boolean",
                        "description": "Count characters"
                    }
                },
                "required": ["paths"]
            }),
            examples: None,
        }
    }

    fn define_sort() -> ToolDefinition {
        ToolDefinition {
            name: "sort".to_string(),
            description: "Sort lines within files.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Files to sort"
                    },
                    "reverse": {
                        "type": "boolean",
                        "description": "Sort in reverse order"
                    },
                    "numeric": {
                        "type": "boolean",
                        "description": "Sort numerically"
                    },
                    "unique": {
                        "type": "boolean",
                        "description": "Suppress duplicate lines"
                    }
                },
                "required": ["paths"]
            }),
            examples: Some(vec![ToolExample {
                description: "Sort log file chronologically".to_string(),
                arguments: json!({"paths": ["app.log"]}),
            }]),
        }
    }

    fn define_uniq() -> ToolDefinition {
        ToolDefinition {
            name: "uniq".to_string(),
            description: "Remove duplicate adjacent lines from files.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Files to process"
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
                    }
                },
                "required": ["paths"]
            }),
            examples: None,
        }
    }

    fn define_shell() -> ToolDefinition {
        ToolDefinition {
            name: "shell".to_string(),
            description: "Execute a shell command inside the sandbox.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute"
                    },
                    "shell": {
                        "type": "string",
                        "description": "Shell to use (bash, pwsh, cmd)"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in seconds",
                        "default": 30
                    },
                    "working_dir": {
                        "type": "string",
                        "description": "Working directory for the command"
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
            examples: Some(vec![ToolExample {
                description: "List Python processes".to_string(),
                arguments: json!({
                    "command": "ps aux | grep python",
                    "shell": "bash",
                    "capture_stdout": true,
                    "capture_stderr": false
                }),
            }]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_expected_tools() {
        let catalog = ToolCatalog::new();
        assert_eq!(catalog.tools().len(), 10);
        assert!(catalog.get_tool("ls").is_some());
        assert!(catalog.get_tool("execute").is_some());
    }

    #[test]
    fn openai_format_contains_required_fields() {
        let catalog = ToolCatalog::new();
        let functions = catalog.to_openai_functions();
        assert_eq!(functions.len(), catalog.tools().len());
        let first = &functions[0];
        assert!(first.get("name").is_some());
        assert!(first.get("description").is_some());
        assert!(first.get("parameters").is_some());
    }
}
