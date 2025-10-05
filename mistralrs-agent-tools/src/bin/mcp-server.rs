// Standalone MCP server binary for agent-tools
//
// This binary exposes all agent-tools utilities via the Model Context Protocol (MCP),
// allowing any MCP client (Claude Desktop, IDEs, etc.) to discover and use them.
//
// Usage:
//   mcp-server [--prefix PREFIX] [--root ROOT_DIR]
//
// The server communicates via stdin/stdout using JSON-RPC 2.0 protocol.

use anyhow::Result;
use mistralrs_agent_tools::{AgentToolkit, McpServer, SandboxConfig};
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    let mut tool_prefix: Option<String> = None;
    let mut root_dir: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--prefix" => {
                if i + 1 < args.len() {
                    tool_prefix = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --prefix requires a value");
                    std::process::exit(1);
                }
            }
            "--root" => {
                if i + 1 < args.len() {
                    root_dir = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("Error: --root requires a value");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Error: Unknown argument: {}", args[i]);
                print_help();
                std::process::exit(1);
            }
        }
    }

    // Create sandbox configuration
    let config = match root_dir {
        Some(root) => SandboxConfig::new(root),
        None => SandboxConfig::default(),
    };

    // Create toolkit
    let toolkit = Arc::new(AgentToolkit::new(config));

    // Create and run MCP server
    let server = McpServer::new(toolkit, tool_prefix);

    eprintln!("Starting MCP server for agent-tools...");
    eprintln!("Listening on stdin/stdout for JSON-RPC messages");

    server.run_stdio()?;

    Ok(())
}

fn print_help() {
    eprintln!(
        r#"MCP Server for Agent Tools

Exposes agent-tools utilities via Model Context Protocol (MCP).

USAGE:
    mcp-server [OPTIONS]

OPTIONS:
    --prefix PREFIX    Prefix to add to all tool names (e.g., "agent")
    --root ROOT_DIR    Root directory for sandbox (default: current directory)
    -h, --help         Print help information

EXAMPLES:
    # Run server with default settings
    mcp-server

    # Run with tool name prefix "agent"
    mcp-server --prefix agent

    # Run with custom sandbox root
    mcp-server --root /workspace

    # Combine options
    mcp-server --prefix fs --root /tmp

PROTOCOL:
    The server communicates via stdin/stdout using JSON-RPC 2.0.

    Supported methods:
    - initialize: Handshake and capability negotiation
    - tools/list: List all available tools with schemas
    - tools/call: Execute a tool with given arguments

AVAILABLE TOOLS:
    - cat: Display file contents
    - ls: List directory contents
    - grep: Search for patterns in files
    - head: Display first lines of files
    - tail: Display last lines of files
    - wc: Count lines, words, and characters
    - sort: Sort lines of text files
    - uniq: Report or filter out repeated lines

For more information, visit:
    https://github.com/EricLBuehler/mistral.rs
"#
    );
}
