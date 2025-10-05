# derive-utils - Windows-Enhanced Utilities

This directory contains Windows-specific utilities and shell wrappers that extend the functionality of the core coreutils with Windows-native features and enhanced shell integration.

## Components

### Core Windows Utilities

1. **where** - Enhanced Windows `where` command with path normalization
1. **which** - Cross-platform command locator with environment detection
1. **tree** - Enhanced directory tree visualization with Windows attributes

### Shell Wrappers with winpath Integration

4. **cmd-wrapper** - Enhanced cmd.exe wrapper with universal path normalization
1. **pwsh-wrapper** - PowerShell wrapper supporting both Core and Windows PowerShell
1. **bash-wrapper** - Bash wrapper with multi-environment support (Git Bash, WSL, Cygwin, MSYS2)

## Build Requirements

⚠️ **CRITICAL**: All utilities in this project MUST be built using the Makefile system. Direct cargo commands are prohibited.

```bash
# Correct build procedure (from winutils root)
make clean
make release
make validate-all-77
make install
```

## Shell Wrapper Features

### Universal Path Normalization

All shell wrappers provide automatic path conversion between different Windows environments:

- **DOS paths**: `C:\Windows\System32`
- **WSL paths**: `/mnt/c/Windows/System32`
- **Git Bash paths**: `/c/Windows/System32`
- **Cygwin paths**: `/cygdrive/c/Windows/System32`
- **UNC paths**: `\\?\C:\Windows\System32`

### Transparent Operation

The wrappers work as drop-in replacements for their respective shells:

```bash
# These work exactly like the original commands
cmd /c "dir C:\Windows"
pwsh -Command "Get-ChildItem C:\Windows"
bash -c "ls /c/Windows"

# But also handle cross-environment paths automatically
cmd /c "dir /mnt/c/Windows"        # WSL path -> Windows path
pwsh -Command "Get-ChildItem /c/Windows"  # Git Bash path -> Windows path
bash -c "ls C:\Windows"            # Windows path -> appropriate bash format
```

### Performance Characteristics

| Component    | Startup Overhead | Memory Usage | Binary Size |
| ------------ | ---------------- | ------------ | ----------- |
| cmd-wrapper  | < 5ms            | ~2MB         | ~1.5MB      |
| pwsh-wrapper | < 10ms           | ~3MB         | ~2MB        |
| bash-wrapper | < 8ms            | ~2.5MB       | ~1.8MB      |

## Installation Structure

After `make install`, the utilities are installed to `C:\users\david\.local\bin\`:

```
C:\users\david\.local\bin\
├── wu-where.exe           # Enhanced where command
├── wu-which.exe           # Cross-platform which command
├── wu-tree.exe            # Enhanced tree command
├── cmd.exe                # CMD wrapper with path normalization
├── pwsh.exe               # PowerShell Core wrapper
├── powershell.exe         # Windows PowerShell wrapper
└── bash.exe               # Bash wrapper (Git Bash/WSL/Cygwin)
```

## Usage Examples

### Cross-Environment File Operations

```bash
# Git Bash environment
bash -c "cp '/c/source/file.txt' '/d/backup/'"

# WSL environment
bash -c "cp '/mnt/c/source/file.txt' '/mnt/d/backup/'"

# PowerShell with mixed paths
pwsh -Command "Copy-Item '/c/source/*' 'D:\backup\'"

# CMD with WSL paths
cmd /c "copy /mnt/c/source\file.txt D:\backup\"
```

### Script Execution with Path Normalization

```powershell
# PowerShell script with automatic path conversion
pwsh -File script.ps1
# Script content:
# Get-ChildItem "/mnt/c/data" | Export-Csv "/c/results/output.csv"
```

```bash
# Bash script with environment detection
bash myscript.sh
# Script content:
#!/bin/bash
cd "C:\workspace"           # Converted to appropriate format
find "D:\logs" -name "*.log" | sort > "C:\reports\logfiles.txt"
```

### Build System Integration

```batch
REM Windows batch file using enhanced shells
cmd /c "bash -c 'make clean && make release'"
pwsh -Command "& bash -c 'npm test && npm run build'"
```

## Debugging and Troubleshooting

Enable debug output for any wrapper:

```bash
# Via environment variable
set WINPATH_DEBUG=1
cmd /c "dir /mnt/c/Windows"

# Via command-line flag
pwsh --debug -Command "Get-ChildItem /c/Windows"
bash --debug -c "ls C:\Windows"
```

Debug output includes:

- Detected environment (Git Bash, WSL, etc.)
- Original and converted paths
- Path conversion reasoning
- Actual command executed

## Error Handling

All wrappers provide comprehensive error handling:

1. **Path conversion failures**: Fallback to original path
1. **Shell detection failures**: Graceful fallback to system PATH
1. **Process creation failures**: Detailed error messages with suggested fixes
1. **Invalid arguments**: Helpful usage information

## Integration with winpath Library

All shell wrappers use the shared `winpath` library for consistent path normalization:

- **Sub-millisecond performance** with LRU caching
- **Comprehensive format support** for all Windows path variants
- **Intelligent detection** of path types and environments
- **Error recovery** with fallback strategies

## Contributing

When modifying or extending these utilities:

1. **Use Makefile build system**: Never use cargo directly
1. **Maintain backward compatibility**: All wrappers must work as drop-in replacements
1. **Add comprehensive tests**: Test all supported environments
1. **Update documentation**: Keep README files current
1. **Follow winpath patterns**: Use the shared library for all path operations

## Testing

Run comprehensive tests for all components:

```bash
# From winutils root directory
make test                    # Run all tests
make validate-all-77         # Validate all utilities
./derive-utils/test_all_git_bash_paths.sh  # Git Bash specific tests
```

Individual component testing:

```bash
# Test specific wrapper (after build)
C:\users\david\.local\bin\cmd.exe --debug /c "echo Test"
C:\users\david\.local\bin\pwsh.exe --debug -Command "Write-Host Test"
C:\users\david\.local\bin\bash.exe --debug -c "echo Test"
```

## License

MIT OR Apache-2.0

______________________________________________________________________

*Part of the winutils project - Windows-optimized GNU coreutils with universal path normalization*
