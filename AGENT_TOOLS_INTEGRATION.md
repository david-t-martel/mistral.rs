# Agent Tools Integration

_Reference: Review the [Repository Guidelines](AGENTS.md) for shared contribution standards before expanding this integration._

## Overview

The `mistralrs-agent-tools` crate has been successfully integrated into the `mistralrs-server` to enable sandboxed filesystem operations in agent mode.

## Architecture

### Components

1. **mistralrs-agent-tools** (Pure Rust crate)

   - Location: `mistralrs-agent-tools/`
   - Provides sandboxed filesystem operations:
     - `read()` - Read file contents
     - `write()` - Write/create files
     - `append()` - Append to files
     - `delete()` - Remove files
     - `find()` - Search for files by pattern
     - `tree()` - List directory trees
     - `exists()` - Check file existence

1. **mistralrs-server/agent_mode.rs**

   - Integrates agent tools for tool execution
   - Parses tool calls from model responses
   - Executes filesystem operations via AgentTools API
   - Returns formatted results to the model

### Sandbox Configuration

The agent tools enforce sandbox restrictions:

- **Root directory**: Set via `MISTRALRS_AGENT_SANDBOX_ROOT` env var or defaults to current directory
- **Read-only paths**: Default protections for `.git/`, `target/`, `node_modules/`
- **Path traversal prevention**: Blocks attempts to escape sandbox
- **File size limits**: Maximum 5 MiB for read operations
- **Result limits**: Maximum 1000 items for find/tree operations

## Usage

### Starting Agent Mode

```bash
# Set sandbox root (optional)
export MISTRALRS_AGENT_SANDBOX_ROOT=/path/to/workspace

# Start agent mode
./mistralrs-server --agent-mode <model-config>
```

### Available Tool Functions

When the model makes tool calls, the following functions are available:

1. **read_file / read**

   ```json
   {"path": "src/main.rs"}
   ```

1. **write_file / write**

   ```json
   {
     "path": "output.txt",
     "content": "Hello, world!",
     "create": true,
     "overwrite": false
   }
   ```

1. **append_file / append**

   ```json
   {"path": "log.txt", "content": "New log entry\n"}
   ```

1. **delete_file / delete**

   ```json
   {"path": "temp.txt"}
   ```

1. **find_files / find**

   ```json
   {"pattern": "*.rs", "max_depth": 3}
   ```

1. **list_tree / tree**

   ```json
   {"root": "src", "max_depth": 2}
   ```

1. **exists**

   ```json
   {"path": "Cargo.toml"}
   ```

## Implementation Details

### Tool Call Execution Flow

1. Model generates text response with tool calls
1. `agent_mode.rs` detects tool calls in `Delta::tool_calls`
1. `execute_tool_calls()` function:
   - Parses tool name and JSON arguments
   - Routes to appropriate AgentTools method
   - Handles errors gracefully
   - Returns formatted results
1. Results are displayed to the user
1. Assistant message with results is added to history

### Code Structure

```rust
// Initialize agent tools with sandbox
let agent_tools = AgentTools::with_defaults();

// When tool calls detected in response
if let Some(tool_calls) = delta.tool_calls {
    let results = execute_tool_calls(&agent_tools, &tool_calls);
    // Display results
    for result in results {
        println!("{}", result);
    }
}
```

### Error Handling

- Sandbox violations return descriptive errors
- Read-only path protection prevents accidental modifications
- File size limits prevent memory exhaustion
- JSON parsing errors are caught and reported
- IO errors are propagated with context

## Security Features

1. **Sandbox Enforcement**

   - All paths validated against sandbox root
   - Symlinks checked for sandbox escape
   - Canonical path resolution

1. **Read-Only Protection**

   - Configurable read-only paths
   - Prevents modification of critical directories

1. **Resource Limits**

   - File size limits (5 MiB)
   - Result count limits (1000 items)
   - Prevents DoS via large operations

1. **Audit Logging**

   - All operations logged via `tracing`
   - Path validation attempts logged
   - Failed operations logged with context

## Testing

The agent tools have comprehensive unit tests:

```bash
cd mistralrs-agent-tools
cargo test
```

Tests cover:

- Sandbox enforcement
- Read-only path protection
- Basic CRUD operations
- Path traversal prevention
- Error handling

## Future Enhancements

1. **Additional Operations**

   - Copy/move files
   - Change permissions
   - Create directories
   - File metadata queries

1. **Enhanced Security**

   - File type validation
   - Content scanning
   - Rate limiting
   - Audit log persistence

1. **Integration Improvements**

   - Streaming large file reads
   - Async filesystem operations
   - Progress reporting
   - Batch operations

## Maintenance Notes

### Dependencies

- `mistralrs-agent-tools` is now part of the workspace
- No external PyO3 dependency - pure Rust implementation
- Uses `camino` for UTF-8 path handling
- Uses `walkdir` for directory traversal

### Configuration

- Sandbox root can be configured via environment variable
- Read-only paths defined in `SandboxConfig::default()`
- Limits defined as constants in `lib.rs`

### Building

```bash
# Build just the agent tools crate
cargo build -p mistralrs-agent-tools

# Build server with agent mode
cargo build -p mistralrs-server --no-default-features

# Build with full features
cargo build -p mistralrs-server
```

## Conclusion

The integration provides a secure, efficient, and maintainable way to enable filesystem operations in agent mode. The pure Rust implementation eliminates PyO3 complexity while maintaining safety through comprehensive sandboxing.
