# Agent Mode Examples

This directory contains practical examples demonstrating autonomous ReAct agent capabilities in mistral.rs.

## Available Examples

### 1. File Analysis Agent (`file-analysis-agent.ps1`)

**Purpose**: Automatically analyze code files in a directory

**Usage**:

```powershell
# Analyze all Rust files in current directory
.\file-analysis-agent.ps1

# Analyze specific pattern in a directory
.\file-analysis-agent.ps1 -TargetDirectory "src" -FilePattern "*.rs"

# Use different model
.\file-analysis-agent.ps1 -Model "meta-llama/Llama-3.2-3B-Instruct"
```

**What It Does**:

1. Lists all matching files
1. Counts lines of code in each file
1. Identifies file purposes
1. Generates structured summary report

**Tools Used**: Filesystem (list_directory, read_file)

______________________________________________________________________

### 2. Code Documentation Generator (`code-doc-generator.ps1`)

**Purpose**: Generate comprehensive documentation for source code files

**Usage**:

```powershell
# Generate docs for a specific file
.\code-doc-generator.ps1 -SourceFile "src\main.rs"

# Specify output file
.\code-doc-generator.ps1 -SourceFile "lib.rs" -OutputFile "library-docs.md"

# Use larger model for better quality
.\code-doc-generator.ps1 -SourceFile "complex.rs" -Model "Qwen/Qwen2.5-7B-Instruct"
```

**What It Does**:

1. Reads source code
1. Analyzes structure (functions, classes, patterns)
1. Identifies dependencies and API surface
1. Creates detailed markdown documentation
1. Writes output to file

**Tools Used**: Filesystem (read_file, write_file)

______________________________________________________________________

### 3. TODO Tracker Agent (`todo-tracker-agent.ps1`)

**Purpose**: Extract and organize TODO comments from codebase

**Usage**:

```powershell
# Track TODOs in current directory
.\todo-tracker-agent.ps1

# Specific project and pattern
.\todo-tracker-agent.ps1 -ProjectDirectory "mistralrs-core" -FilePattern "*.rs"

# Custom output file
.\todo-tracker-agent.ps1 -OutputFile "PROJECT-TODOS.md"
```

**What It Does**:

1. Searches for all matching source files
1. Extracts TODO/FIXME/HACK comments
1. Categorizes by priority and component
1. Adds timestamp
1. Creates organized markdown report

**Tools Used**: Filesystem (list_directory, read_file, write_file), Time (get_current_time)

______________________________________________________________________

## How to Run

### Prerequisites

1. **Built mistralrs-server**:

   ```bash
   make build-cuda-full  # or appropriate build for your platform
   ```

1. **Node.js/npx** (for MCP servers):

   ```bash
   node --version  # Should be v18+
   npx --version
   ```

1. **Model** (either):

   - HuggingFace model ID (e.g., `Qwen/Qwen2.5-1.5B-Instruct`)
   - Local GGUF file (e.g., `models/Qwen2.5-1.5B-Q4_K_M.gguf`)

### Running Examples

**Method 1: Direct Execution**

```powershell
.\file-analysis-agent.ps1
```

**Method 2: Via Makefile**

```bash
# Create custom target in Makefile
make run-file-analysis
```

**Method 3: Modify for Your Needs**
Copy an example and customize:

```powershell
cp file-analysis-agent.ps1 my-custom-agent.ps1
# Edit my-custom-agent.ps1 to change prompt, tools, etc.
.\my-custom-agent.ps1
```

## Creating Your Own Examples

### Template Structure

```powershell
#!/usr/bin/env pwsh
# your-agent.ps1
# Description of what this agent does

[CmdletBinding()]
param(
    [Parameter()]
    [string]$Model = "Qwen/Qwen2.5-1.5B-Instruct",
    # Add your parameters
)

# 1. Source utilities
. "$PSScriptRoot\..\..\..\scripts\utils\Get-ProjectPaths.ps1"

# 2. Create MCP configuration
$mcpConfig = @{
    servers = @(
        @{
            id = "your-server"
            name = "Your MCP Server"
            source = @{
                type = "Process"
                command = "npx"
                args = @("-y", "@modelcontextprotocol/server-xxx")
            }
        }
    )
    auto_register_tools = $true
} | ConvertTo-Json -Depth 10

$mcpConfigPath = Join-Path $env:TEMP "your-agent-mcp-config.json"
$mcpConfig | Set-Content $mcpConfigPath

# 3. Define agent prompt
$prompt = @"
Your task description here:
1. Step one
2. Step two
3. Step three
"@

# 4. Get binary and launch
$binaryPath = Get-MistralRSBinary
$cmdArgs = @(
    '--agent-mode',
    '--mcp-config', $mcpConfigPath,
    'plain', '-m', $Model
)

Write-Host "Paste this prompt when agent starts:"
Write-Host $prompt

& $binaryPath @cmdArgs

# 5. Cleanup
Remove-Item $mcpConfigPath -ErrorAction SilentlyContinue
```

### Available MCP Servers

Reference these in your examples:

| Server     | ID                                        | Tools                                       | Usage               |
| ---------- | ----------------------------------------- | ------------------------------------------- | ------------------- |
| Filesystem | `@modelcontextprotocol/server-filesystem` | read_file, write_file, list_directory, etc. | File operations     |
| GitHub     | `@modelcontextprotocol/server-github`     | create_issue, create_pr, search_repos       | GitHub automation   |
| Time       | `@modelcontextprotocol/server-time`       | get_current_time, convert_timezone          | Time operations     |
| Memory     | `@modelcontextprotocol/server-memory`     | store, recall                               | Context persistence |
| Fetch      | `@modelcontextprotocol/server-fetch`      | fetch_url                                   | HTTP requests       |

See [MCP Servers](https://github.com/modelcontextprotocol/servers) for full list.

## Tips for Effective Agents

### 1. Prompt Design

**Good Prompts**:

- ✅ Clear, numbered steps
- ✅ Explicit tool usage hints
- ✅ Specify output format
- ✅ Include error handling guidance

**Example**:

```
1. List all .py files in the src directory
2. For each file, count lines and identify imports
3. Create a dependency graph
4. Write results to dependencies.json

If a file cannot be read, skip it and note in output.
```

### 2. Model Selection

- **Quick Tasks**: Qwen2.5-1.5B (940MB, fast)
- **Analysis**: Qwen2.5-3B or Gemma 2 2B (1.5-2GB, good quality)
- **Complex Tasks**: Qwen2.5-7B or Llama 3.1 8B (4-8GB, high quality)

### 3. MCP Configuration

```json
{
  "servers": [...],
  "auto_register_tools": true,          // Always true for agent mode
  "tool_timeout_secs": 60,              // Adjust based on task
  "max_concurrent_calls": 5             // Limit for parallel execution
}
```

### 4. Error Handling

Agents should gracefully handle:

- Missing files → Skip with note
- Permission errors → Inform user
- Tool failures → Retry or suggest alternative

Include error handling hints in your prompts.

## Testing Your Examples

### Quick Test

```powershell
# Run with verbose output
$env:RUST_LOG="debug"
.\your-agent.ps1
```

### Automated Testing

Create a companion test script:

```powershell
# test-your-agent.ps1
$result = .\your-agent.ps1 -SomeParam "test"
if (Test-Path "expected-output.md") {
    Write-Host "✓ Test passed"
} else {
    Write-Error "✗ Test failed"
}
```

### Integration with CI

```yaml
# .github/workflows/test-agents.yml
- name: Test Agent Examples
  run: |
    pwsh tests/agent/examples/file-analysis-agent.ps1
    pwsh tests/agent/examples/todo-tracker-agent.ps1
```

## Contributing

To add a new example:

1. **Create script**: `tests/agent/examples/your-example.ps1`
1. **Add documentation**: Update this README
1. **Test thoroughly**: Verify with multiple models
1. **Follow patterns**: Use existing examples as templates

## Support

For questions or issues:

- See [AGENT_MODE_GUIDE.md](../../../docs/AGENT_MODE_GUIDE.md)
- Check [mistral.rs docs](../../../docs/)
- Open an issue on GitHub

______________________________________________________________________

**Happy Agent Building!** 🤖
