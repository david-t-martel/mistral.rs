//! Tool discovery and schema generation for LLM function calling
//!
//! Provides tool metadata and JSON schemas for LLM integration:
//! - Tool definitions with parameters
//! - JSON schema generation
//! - Tool catalog management

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

/// Tool definition for LLM function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name (used in function calls)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Parameter schema (JSON Schema format)
    pub parameters: JsonValue,
    /// Example usage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<ToolExample>>,
}

/// Example tool usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub description: String,
    pub arguments: JsonValue,
}

/// Tool catalog containing all available tools
#[derive(Debug, Clone)]
pub struct ToolCatalog {
    tools: Vec<ToolDefinition>,
}

impl ToolCatalog {
    /// Create a new tool catalog with default tools
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
                Self::define_execute(),
            ],
        }
    }

    /// Get all tool definitions
    pub fn tools(&self) -> &[ToolDefinition] {
        &self.tools
    }

    /// Get a tool definition by name
    pub fn get_tool(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.iter().find(|t| t.name == name)
    }

    /// Convert to OpenAI function calling format
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

    /// Convert to anthropic tool format
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

    // Tool definitions

    fn define_ls() -> ToolDefinition {
        ToolDefinition {
            name: "ls".to_string(),
            description: "List directory contents. Shows files and directories in the specified path.".to_string(),
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
                        "description": "Regular expression pattern to search for"
                    },
                    "paths": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Files to search in"
                    },
                    "ignore_case": {
                        "type": "boolean",
                        "description": "Case-insensitive search"
                    },
                    "line_number": {
                        "type": "boolean",
                        "description": "Show line numbers in output"
                    },
                    "context": {
                        "type": "integer",
                        "description": "Show N lines before and after matches"
                    }
                },
                "required": ["pattern", "paths"]
            }),
            examples: Some(vec![ToolExample {
                description: "Search for TODO comments".to_string(),
                arguments: json!({"pattern": "TODO", "paths": ["src/"], "ignore_case": true}),
            }]),
        }
    }

    fn define_wc() -> ToolDefinition {
        ToolDefinition {
            name: "wc".to_string(),
            description: "Count lines, words, and characters in files.".to_string(),
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
            examples: None,
        }
    }

    fn define_sort() -> ToolDefinition {
        ToolDefinition {
            name: "sort".to_string(),
            description: "Sort lines of text files.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Files to sort"
                    },
                    "numeric": {
                        "type": "boolean",
                        "description": "Sort numerically"
                    },
                    "reverse": {
                        "type": "boolean",
                        "description": "Reverse sort order"
                    },
                    "unique": {
                        "type": "boolean",
                        "description": "Remove duplicate lines"
                    }
                },
                "required": ["paths"]
            }),
            examples: None,
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
                    }
                },
                "required": ["paths"]
            }),
            examples: None,
        }
    }

    fn define_execute() -> ToolDefinition {
        ToolDefinition {
            name: "execute".to_string(),
            description: "Execute a shell command. Use with caution.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute"
                    },
                    "args": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Command arguments"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in seconds"
                    }
                },
                "required": ["command"]
            }),
            examples: Some(vec![ToolExample {
                description: "List processes".to_string(),
                arguments: json!({"command": "ps", "args": ["aux"]}),
            }]),
        }
    }
}

impl Default for ToolCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_creation() {
        let catalog = ToolCatalog::new();
        assert_eq!(catalog.tools().len(), 9);
    }

    #[test]
    fn test_get_tool() {
        let catalog = ToolCatalog::new();
        let ls_tool = catalog.get_tool("ls");
        assert!(ls_tool.is_some());
        assert_eq!(ls_tool.unwrap().name, "ls");
    }

    #[test]
    fn test_openai_format() {
        let catalog = ToolCatalog::new();
        let functions = catalog.to_openai_functions();
        assert_eq!(functions.len(), 9);
        assert!(functions[0].get("name").is_some());
        assert!(functions[0].get("description").is_some());
        assert!(functions[0].get("parameters").is_some());
    }

    #[test]
    fn test_anthropic_format() {
        let catalog = ToolCatalog::new();
        let tools = catalog.to_anthropic_tools();
        assert_eq!(tools.len(), 9);
        assert!(tools[0].get("name").is_some());
        assert!(tools[0].get("input_schema").is_some());
    }
}
