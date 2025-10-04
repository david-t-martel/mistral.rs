# ReAct Agent Mode - User Guide

## Overview

mistral.rs now includes **Agent Mode** - an autonomous ReAct-style (Reasoning + Acting) orchestration system that enables models to independently reason about tasks, decide which tools to use, execute them automatically, and iterate until the task is complete.

### What is ReAct?

ReAct is a prompting paradigm that combines:

- **Reasoning**: The model thinks through the problem step-by-step
- **Acting**: The model decides to use tools when needed
- **Observing**: The model processes tool results and continues reasoning

Unlike traditional chat interfaces where tool execution requires manual intervention, Agent Mode **automatically executes tools** and feeds results back to the model until the task is solved.

## Key Features

✅ **Autonomous Tool Execution** - Tools are called and executed automatically
✅ **Multi-Step Reasoning** - Continues iterating until task complete
✅ **MCP Integration** - Works seamlessly with MCP auto-registered tools
✅ **Native Tool Support** - Compatible with registered tool callbacks
✅ **Streaming Display** - Shows model responses in real-time
✅ **REPL Interface** - Interactive command-line experience

## Quick Start

### 1. Basic Usage (HuggingFace Model)

```bash
# Launch agent mode with a HuggingFace model
./mistralrs-server --agent-mode plain -m Qwen/Qwen2.5-1.5B-Instruct
```

### 2. With MCP Tools

```bash
# Create mcp-config.json
{
  "servers": [{
    "id": "filesystem",
    "name": "Filesystem Tools",
    "source": {
      "type": "Process",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
    }
  }],
  "auto_register_tools": true
}

# Launch with MCP integration
./mistralrs-server --agent-mode --mcp-config mcp-config.json \
  plain -m Qwen/Qwen2.5-1.5B-Instruct
```

### 3. Using Makefile

```bash
# Interactive demo with filesystem and time tools
make demo-agent

# With GGUF model
make demo-agent-gguf

# Run automated tests
make test-agent
```

## CLI Options

### Agent Mode Flag

```bash
--agent-mode
```

Enables autonomous ReAct agent mode instead of standard interactive/server mode.

### MCP Configuration

```bash
--mcp-config <path>
```

Path to MCP configuration file. The agent will auto-discover and register all tools from configured MCP servers.

### Model Selection

Works with all model loading modes:

```bash
# HuggingFace plain model
plain -m <model_id>

# GGUF quantized model
gguf -m <path> -f <filename>

# Vision model (with agent capabilities)
vision -m <model_id>
```

## REPL Commands

Once in agent mode, use these commands:

| Command            | Description                        | Example            |
| ------------------ | ---------------------------------- | ------------------ |
| `\help`            | Show help message                  | `\help`            |
| `\exit`            | Quit agent mode                    | `\exit`            |
| `\clear`           | Clear conversation history         | `\clear`           |
| `\temperature <N>` | Set sampling temperature (0.0-2.0) | `\temperature 0.7` |
| `\topk <N>`        | Set top-k sampling                 | `\topk 50`         |
| `\topp <N>`        | Set top-p sampling (0.0-1.0)       | `\topp 0.9`        |

## Example Workflows

### Filesystem Operations

**Prompt:**

```
List all markdown files in the docs directory and summarize the main topics
```

**What Happens:**

1. Model reasons: "I need to list markdown files"
1. Model uses `list_directory` or `read_directory` tool
1. Model observes file list
1. Model uses `read_file` for each markdown file
1. Model synthesizes summary from all files

### Multi-Tool Workflow

**Prompt:**

```
Check the current time in Tokyo and write it to a file named tokyo-time.txt
```

**What Happens:**

1. Model uses `get_current_time` with timezone parameter
1. Model observes the time result
1. Model uses `write_file` to create tokyo-time.txt
1. Model confirms task completion

### Web Search + File Operations

**Prompt:**

```
Search for the latest Rust news and save the findings to rust-news.md
```

**What Happens (with `--enable-search`):**

1. Model uses web search tool to find Rust news
1. Model processes search results
1. Model uses `write_file` to create rust-news.md
1. Model confirms file creation

## MCP Server Configuration

### Filesystem Server

```json
{
  "servers": [{
    "id": "filesystem",
    "name": "Filesystem Tools",
    "source": {
      "type": "Process",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
    }
  }],
  "auto_register_tools": true,
  "tool_timeout_secs": 30
}
```

**Available Tools:**

- `read_file` - Read file contents
- `write_file` - Write/create files
- `list_directory` - List directory contents
- `create_directory` - Create directories
- `move_file` - Move/rename files
- `search_files` - Search for files by pattern

### Time Server

```json
{
  "servers": [{
    "id": "time",
    "name": "Time Tools",
    "source": {
      "type": "Process",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-time"]
    }
  }],
  "auto_register_tools": true
}
```

**Available Tools:**

- `get_current_time` - Get current time with timezone support
- `convert_timezone` - Convert between timezones
- `get_timezone` - Get timezone information

### GitHub Server

```json
{
  "servers": [{
    "id": "github",
    "name": "GitHub Tools",
    "source": {
      "type": "Process",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_TOKEN": "ghp_your_token_here"
      }
    }
  }],
  "auto_register_tools": true
}
```

**Available Tools:**

- `create_issue` - Create GitHub issues
- `create_pull_request` - Create pull requests
- `search_repositories` - Search for repositories
- `get_file_contents` - Read repository files

## Architecture

### How It Works

```
┌─────────────┐
│  User Query │
└──────┬──────┘
       │
       ▼
┌─────────────────┐
│ Model Reasoning │ ◄─────────┐
└──────┬──────────┘           │
       │                      │
       │ (Tool Call Detected) │
       ▼                      │
┌──────────────────┐          │
│ Auto Tool Exec   │          │
│ (via Engine)     │          │
└──────┬───────────┘          │
       │                      │
       │ (Tool Result)        │
       ▼                      │
┌──────────────────┐          │
│ Feed Result Back │──────────┘
│ to Model         │
└──────────────────┘
       │
       │ (No More Tools)
       ▼
┌─────────────────┐
│  Final Answer   │
└─────────────────┘
```

### Engine-Powered Execution

Agent Mode leverages mistralrs-core's built-in tool execution system:

- **Automatic Detection**: The engine detects `tool_calls` in model responses
- **Callback Execution**: Registered callbacks (native or MCP) are invoked automatically
- **Result Integration**: Tool results are formatted and fed back to the model
- **Multi-Turn Support**: Process continues until no more tools are needed

This design means Agent Mode works with **any** tool system that integrates with mistralrs-core.

## Testing

### Automated Test Suite

```bash
# Run pre-flight checks
make test-agent

# Generate JSON report
make test-agent-json

# Generate Markdown report
make test-agent-markdown
```

### Interactive Demo

```bash
# Launch interactive demo
./tests/agent/demo-agent-mode.ps1 -Interactive

# With specific model
./tests/agent/demo-agent-mode.ps1 -Model "meta-llama/Llama-3.2-3B-Instruct"

# With GGUF
./tests/agent/demo-agent-mode.ps1 -ModelType gguf \
  -ModelPath "T:/models" \
  -ModelFile "Qwen2.5-1.5B-Instruct-Q4_K_M.gguf"
```

### Manual Test Scenarios

Try these prompts to verify agent behavior:

1. **File Listing**:
   `"List all .rs files in the mistralrs-core/src directory"`

1. **File Reading**:
   `"Read the README.md and tell me what this project does"`

1. **File Creation**:
   `"Create a file named test.txt with 'Hello from Agent Mode'"`

1. **Time Query**:
   `"What time is it in London right now?"`

1. **Multi-Step**:
   `"Find all TODO comments in .rs files and write them to todos.txt"`

## Troubleshooting

### Agent Mode Not Activating

**Problem**: Server starts but agent mode doesn't activate

**Solution**: Ensure `--agent-mode` flag comes BEFORE model selector:

```bash
# ✅ Correct
./mistralrs-server --agent-mode plain -m Qwen/Qwen2.5-1.5B-Instruct

# ❌ Wrong
./mistralrs-server plain -m Qwen/Qwen2.5-1.5B-Instruct --agent-mode
```

### MCP Tools Not Available

**Problem**: Agent doesn't have access to MCP tools

**Solution**: Check MCP configuration:

1. Verify `--mcp-config` path is correct
1. Ensure `auto_register_tools: true` in config
1. Check MCP server process is available (`npx` installed)
1. Review logs for MCP server startup errors

### Tools Executing But No Response

**Problem**: Tools execute but model doesn't synthesize answer

**Solution**:

- Increase `tool_timeout_secs` in MCP config
- Try simpler prompts first
- Check model supports function calling (most modern models do)
- Verify conversation history isn't too long (clear with `\clear`)

### Performance Issues

**Problem**: Agent mode is slow

**Solutions**:

- Use quantized models (GGUF Q4_K_M or lower)
- Reduce `max_concurrent_calls` in MCP config
- Use GPU acceleration (CUDA/Metal builds)
- Limit context window with simpler queries

## Best Practices

### 1. Model Selection

**Recommended Models for Agent Mode:**

- **Small/Fast**: Qwen2.5-1.5B-Instruct (940MB GGUF)
- **Balanced**: Qwen2.5-3B-Instruct or Gemma 2 2B (1.5-2GB)
- **Capable**: Llama 3.2 3B, Qwen2.5-7B (4-5GB)
- **Advanced**: Llama 3.1 8B or larger (8GB+)

### 2. Tool Configuration

- **Start Simple**: Begin with filesystem or time tools
- **Add Gradually**: Introduce more tools as needed
- **Set Timeouts**: Configure appropriate `tool_timeout_secs`
- **Limit Concurrency**: Use `max_concurrent_calls` to prevent overload

### 3. Prompt Engineering

**Good Prompts:**

- ✅ "List all Python files and count the total lines of code"
- ✅ "Read config.json and explain what settings are configured"
- ✅ "Find all TODO comments and create a summary report"

**Avoid:**

- ❌ Overly complex multi-step tasks (break them down)
- ❌ Ambiguous requests without clear actions
- ❌ Tasks requiring external state not accessible via tools

### 4. Debugging

Enable verbose logging:

```bash
RUST_LOG=debug ./mistralrs-server --agent-mode \
  --mcp-config mcp-config.json plain -m <model>
```

Check tool execution:

```bash
# Monitor tool calls in logs
RUST_LOG=mistralrs_core=debug ./mistralrs-server --agent-mode ...
```

## Comparison with Interactive Mode

| Feature             | Interactive Mode                           | Agent Mode                 |
| ------------------- | ------------------------------------------ | -------------------------- |
| **Tool Execution**  | Manual (user sees tool_calls, must handle) | Automatic                  |
| **Multi-Step**      | Single request/response                    | Iterative until complete   |
| **Use Case**        | Chat, Q&A, generation                      | Task automation, workflows |
| **Tool Support**    | Via API response                           | Built-in orchestration     |
| **User Experience** | Traditional chat                           | Autonomous agent           |

## Extending Agent Mode

### Adding Custom Tools

To add custom tools, implement them as MCP servers or native callbacks:

**Native Callback** (in Rust):

```rust
let model = TextModelBuilder::new("model")
    .with_tool_callback_and_tool("custom_tool", callback, tool_def)
    .build()
    .await?;
```

**MCP Server** (Node.js/Python):
Create an MCP server that exposes tools via stdio JSON-RPC, then add to config:

```json
{
  "servers": [{
    "id": "custom",
    "source": {
      "type": "Process",
      "command": "node",
      "args": ["custom-mcp-server.js"]
    }
  }]
}
```

### Integration with CI/CD

```yaml
# .github/workflows/agent-tests.yml
- name: Run Agent Mode Tests
  run: make test-agent-json

- name: Upload Results
  uses: actions/upload-artifact@v3
  with:
    name: agent-test-results
    path: tests/results/agent-test-results.json
```

## FAQ

**Q: Does agent mode work with all models?**
A: Yes, but models trained on function calling (Llama 3.x, Qwen2.5, Gemma, etc.) work best.

**Q: Can I use agent mode over HTTP API?**
A: Currently, agent mode is CLI-only. HTTP API supports tool calling but requires manual orchestration.

**Q: How many iterations can the agent perform?**
A: The engine handles multi-turn automatically. No hard limit, but conversation length is constrained by model context window.

**Q: Can I see individual tool execution steps?**
A: Enable debug logging (`RUST_LOG=debug`) to see detailed tool execution in logs.

**Q: Is agent mode compatible with web search?**
A: Yes! Use `--enable-search` flag and the agent can automatically perform web searches.

## Next Steps

1. **Try the Demo**: `make demo-agent`
1. **Run Tests**: `make test-agent`
1. **Create Custom MCP Server**: See [MCP documentation](https://modelcontextprotocol.io)
1. **Integrate with Workflow**: Use agent mode in automation scripts

## Resources

- **mistral.rs Documentation**: [docs/](../docs/)
- **MCP Protocol**: https://modelcontextprotocol.io
- **MCP Servers**: https://github.com/modelcontextprotocol/servers
- **Tool Calling Guide**: [docs/TOOLS.md](./TOOLS.md) *(if exists)*
- **Example Scripts**: [tests/agent/](../tests/agent/)

______________________________________________________________________

**Agent Mode** transforms mistral.rs from a chat interface into an autonomous task executor. Experiment with different tools and models to discover powerful automation workflows!
