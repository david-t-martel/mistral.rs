# cmd-wrapper - Enhanced Windows CMD.exe Wrapper

A Rust-based wrapper for Windows cmd.exe that provides universal path normalization across all Windows environments (DOS, WSL, Git Bash, Cygwin).

## Features

- **Universal Path Normalization**: Automatically converts paths between different Windows environments
- **Transparent Operation**: Works as a drop-in replacement for cmd.exe
- **Intelligent Path Detection**: Recognizes and converts various path formats
- **Environment Variable Expansion**: Properly handles Windows environment variables
- **Batch File Support**: Full compatibility with Windows batch files
- **Error Handling**: Robust error handling with detailed logging

## Installation

```bash
# Build using the mandatory Makefile (NEVER use cargo directly)
make clean
make release
make install
```

The wrapper will be installed as `cmd.exe` in `C:\users\david\.local\bin\`.

## Usage

### Basic Usage

```cmd
# Use exactly like regular cmd.exe
cmd /c "dir C:\Windows"

# Paths are automatically normalized
cmd /c "dir /mnt/c/Windows"  # WSL path -> Windows path
cmd /c "dir /c/Windows"      # Git Bash path -> Windows path
```

### Path Normalization Examples

```cmd
# All these paths are normalized to C:\Windows\System32
cmd /c "dir C:\Windows\System32"        # DOS format (no change)
cmd /c "dir C:/Windows/System32"        # Mixed separators -> backslashes
cmd /c "dir /mnt/c/Windows/System32"    # WSL format -> Windows format
cmd /c "dir /c/Windows/System32"        # Git Bash format -> Windows format
cmd /c "dir /cygdrive/c/Windows/System32" # Cygwin format -> Windows format
```

### Advanced Options

```cmd
# Preserve original arguments (disable path normalization)
cmd --preserve-args /c "echo /mnt/c/Windows"

# Enable debug output
cmd --debug /c "dir C:\Windows"

# Set debug via environment variable
set WINPATH_DEBUG=1
cmd /c "dir /mnt/c/Windows"
```

## Environment Variables

- `WINPATH_DEBUG`: Enable debug output for path normalization
- `WINPATH_CACHE_SIZE`: Set LRU cache size for path normalization (default: 1024)
- `WINPATH_NO_CACHE`: Disable path caching entirely

## Integration with winpath

The wrapper uses the `winpath` library for path normalization:

- **Performance**: Sub-millisecond path normalization with LRU caching
- **Accuracy**: Handles edge cases like long paths, UNC paths, and special characters
- **Compatibility**: Works across all Windows environments

## Examples

### Batch File Execution

```cmd
# Execute batch file with normalized paths
cmd /c "mybatch.bat /mnt/c/data input.txt"
```

### Pipe and Redirection Support

```cmd
# Full support for CMD.exe features
cmd /c "dir C:\Windows > output.txt"
cmd /c "type input.txt | findstr pattern"
```

### Cross-Environment Compatibility

```cmd
# Works seamlessly in different environments
# Git Bash:
cmd /c "dir /c/Program Files"

# WSL:
cmd /c "dir /mnt/c/Program Files"

# Standard Windows:
cmd /c "dir C:\Program Files"
```

## Error Handling

The wrapper provides detailed error messages and logging:

```cmd
# Enable debug to see path conversion details
set WINPATH_DEBUG=1
cmd /c "dir /invalid/path"
```

## Performance

- **Startup overhead**: < 5ms additional latency
- **Path normalization**: < 1ms per path (cached)
- **Memory usage**: ~2MB additional RAM
- **Binary size**: ~1.5MB executable

## Limitations

- Requires Windows 10 or later
- Some advanced CMD.exe features may have minor compatibility differences
- Performance overhead for simple commands (< 5ms)

## Technical Details

### Path Detection Algorithm

The wrapper uses heuristics to detect paths:

- Contains `/` or `\` characters
- Starts with drive letter pattern (`C:`)
- Matches WSL patterns (`/mnt/c/`)
- Matches Git Bash patterns (`/c/`)
- Matches Cygwin patterns (`/cygdrive/c/`)

### Windows API Integration

- Uses `CreateProcessW` for proper process creation
- Inherits standard handles for seamless I/O
- Handles wide character encoding correctly
- Supports long path names (>260 characters)

## Contributing

This is part of the larger winutils project. All changes must:

1. Use the Makefile build system (NEVER cargo directly)
1. Maintain backward compatibility with cmd.exe
1. Include comprehensive tests
1. Follow Rust best practices

## License

MIT OR Apache-2.0
