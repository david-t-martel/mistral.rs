# bash-wrapper - Enhanced Bash Wrapper

A Rust-based wrapper for bash that provides universal path normalization and environment-specific path conversion for optimal compatibility across Git Bash, WSL, Cygwin, and MSYS2.

## Features

- **Multi-Environment Support**: Detects and adapts to Git Bash, WSL, Cygwin, and MSYS2
- **Intelligent Path Conversion**: Converts paths to the appropriate format for each bash environment
- **Script-Aware Processing**: Parses shell scripts to normalize paths within commands
- **Interactive & Non-Interactive**: Supports both interactive shells and script execution
- **Shell Feature Preservation**: Maintains full bash functionality including pipes, redirections, and job control
- **Environment Detection**: Automatically detects the bash environment and adjusts behavior

## Installation

```bash
# Build using the mandatory Makefile (NEVER use cargo directly)
make clean
make release
make install
```

The wrapper will be installed as `bash.exe` in `C:\users\david\.local\bin\`.

## Supported Bash Environments

### 1. Git Bash (Most Common)

- **Path Format**: `/c/Windows/System32`
- **Location**: `C:\Program Files\Git\bin\bash.exe`
- **Features**: Login shell support, Windows path integration

### 2. Windows Subsystem for Linux (WSL)

- **Path Format**: `/mnt/c/Windows/System32`
- **Location**: `C:\Windows\System32\bash.exe` or `bash.exe` in PATH
- **Features**: Full Linux compatibility, Windows interop

### 3. Cygwin

- **Path Format**: `/cygdrive/c/Windows/System32`
- **Location**: `C:\cygwin64\bin\bash.exe`
- **Features**: POSIX environment emulation

### 4. MSYS2

- **Path Format**: `/c/Windows/System32` (similar to Git Bash)
- **Location**: `C:\msys64\usr\bin\bash.exe`
- **Features**: Package management, development tools

## Usage

### Basic Usage

```bash
# Use exactly like regular bash
bash -c "ls /c/Windows"

# Paths are automatically converted to the appropriate format
bash -c "ls C:\Windows"  # Windows -> Git Bash format
bash -c "cd /mnt/c/data && pwd"  # WSL format preserved
```

### Environment-Specific Path Conversion

```bash
# Git Bash environment - converts to /c/ format
bash -c "ls 'C:\Program Files'"
# Result: ls '/c/Program Files'

# WSL environment - converts to /mnt/c/ format
bash -c "ls 'C:\Program Files'"
# Result: ls '/mnt/c/Program Files'

# Cygwin environment - converts to /cygdrive/c/ format
bash -c "ls 'C:\Program Files'"
# Result: ls '/cygdrive/c/Program Files'
```

### Script File Execution

```bash
# Execute script with path normalization
bash myscript.sh

# Example script content that gets normalized:
#!/bin/bash
cd "C:\data"                    # -> cd "/c/data" (Git Bash)
ls "D:\projects"                # -> ls "/d/projects"
cp "/mnt/c/source/*" "/c/dest"  # Paths converted as needed
```

### Interactive Mode

```bash
# Start interactive bash session
bash -i

# Login shell
bash -l

# Combined interactive login shell
bash -i -l
```

### Advanced Options

```bash
# Preserve original arguments (disable path normalization)
bash --preserve-args -c "echo 'C:\Windows'"

# Enable debug output
bash --debug -c "ls /c/Windows"

# Force interactive mode
bash --interactive -c "pwd"

# Environment variable control
export WINPATH_DEBUG=1
bash -c "ls C:\Windows"
```

## Path Normalization in Scripts

The wrapper intelligently parses shell scripts and normalizes paths in common commands:

### File Operations

```bash
bash -c "cp 'C:\source\file.txt' 'D:\destination\'"
bash -c "mv '/mnt/c/old/path' '/c/new/path'"
bash -c "rm 'C:\temporary\file.log'"
```

### Directory Operations

```bash
bash -c "cd 'C:\workspace' && ls -la"
bash -c "mkdir -p 'D:\new\directory\structure'"
bash -c "find 'C:\data' -name '*.txt'"
```

### Text Processing

```bash
bash -c "grep 'pattern' 'C:\logs\application.log'"
bash -c "cat 'D:\config\settings.conf' | grep 'value'"
bash -c "sort 'C:\data\input.txt' > 'D:\output\sorted.txt'"
```

### I/O Redirection

```bash
bash -c "echo 'Hello' > 'C:\output\message.txt'"
bash -c "command 2> 'D:\logs\errors.log'"
bash -c "program < 'C:\input\data.txt' > 'D:\results\output.txt'"
```

## Environment Variable Support

- `WINPATH_DEBUG`: Enable debug output for path normalization
- `WINPATH_CACHE_SIZE`: Set LRU cache size for path normalization
- `WINPATH_NO_CACHE`: Disable path caching entirely
- `BASH_ENV`: Startup file for non-interactive shells (preserved)
- `PS1`, `PS2`: Prompt variables (preserved)

## Complex Script Examples

### Build Script

```bash
#!/bin/bash
# Cross-platform build script with automatic path normalization

SOURCE_DIR="C:\workspace\project"        # -> /c/workspace/project
BUILD_DIR="D:\builds\release"            # -> /d/builds/release
OUTPUT_DIR="/mnt/c/releases"             # Preserved in WSL

cd "$SOURCE_DIR"
make clean
make release
cp build/output/* "$BUILD_DIR/"
tar -czf "$OUTPUT_DIR/release.tar.gz" -C "$BUILD_DIR" .
```

### Data Processing Pipeline

```bash
bash -c "cat 'C:\data\input.csv' | \
         cut -d',' -f1,3 | \
         sort | \
         uniq > 'D:\output\processed.csv'"
```

### Cross-Environment File Sync

```bash
#!/bin/bash
# Sync files between Windows and WSL paths

WINDOWS_PATH="C:\Users\David\Documents"    # -> /c/Users/David/Documents (Git Bash)
WSL_PATH="/mnt/c/Users/David/Documents"    # Preserved (WSL)
BACKUP_PATH="D:\backups"                   # -> /d/backups

rsync -av "$WINDOWS_PATH/" "$BACKUP_PATH/windows-backup/"
rsync -av "$WSL_PATH/" "$BACKUP_PATH/wsl-backup/"
```

## Error Handling

Comprehensive error handling with detailed logging:

```bash
# Enable debug to see path conversion and environment detection
export WINPATH_DEBUG=1
bash -c "ls /invalid/path/that/does/not/exist"

# Error output includes:
# - Detected bash environment (Git Bash, WSL, etc.)
# - Original and converted paths
# - Conversion reasoning
# - Actual bash command executed
```

## Performance

- **Startup overhead**: < 8ms additional latency
- **Script parsing**: < 3ms for typical scripts
- **Path normalization**: < 1ms per path (cached)
- **Memory usage**: ~2.5MB additional RAM
- **Binary size**: ~1.8MB executable

## Integration Examples

### Windows Batch File

```batch
REM Call bash from Windows batch file
bash.exe -c "find 'C:\logs' -name '*.log' -mtime +7 -delete"
```

### PowerShell Integration

```powershell
# Use bash from PowerShell
& bash.exe -c "grep 'ERROR' 'C:\application\logs\*.log'"
```

### Git Hooks

```bash
#!/bin/bash
# Git pre-commit hook with Windows path support
bash -c "python 'C:\tools\check-syntax.py' $(git diff --cached --name-only)"
```

## Limitations

- Bash version 4.0 or later required
- Some advanced bash features may have minor compatibility differences
- Complex shell expansions in scripts may need manual review
- Performance overhead for simple commands (< 8ms)
- WSL1 vs WSL2 differences may affect some operations

## Technical Details

### Environment Detection Algorithm

1. Analyzes bash executable path
1. Checks for environment-specific indicators
1. Tests path access patterns
1. Sets appropriate path conversion strategy

### Path Conversion Logic

- **Git Bash**: `C:\path` → `/c/path`
- **WSL**: `C:\path` → `/mnt/c/path`
- **Cygwin**: `C:\path` → `/cygdrive/c/path`
- **MSYS2**: `C:\path` → `/c/path`

### Script Parsing

- Uses regex patterns to identify common commands with paths
- Handles quoted paths with spaces
- Preserves shell variables and expansions
- Maintains script structure and logic

### Windows API Integration

- Uses `CreateProcessW` for proper process creation
- Handles wide character encoding
- Inherits standard handles for seamless I/O
- Supports job objects for proper cleanup

## Troubleshooting

### Common Issues

1. **Path not found errors**

   ```bash
   # Enable debug to see path conversion
   export WINPATH_DEBUG=1
   bash -c "ls 'problematic/path'"
   ```

1. **Wrong bash environment detected**

   ```bash
   # Check detected environment
   bash --debug -c "echo 'Environment test'"
   ```

1. **Script parsing issues**

   ```bash
   # Use preserve-args for complex scripts
   bash --preserve-args myscript.sh
   ```

## Contributing

This is part of the larger winutils project. All changes must:

1. Use the Makefile build system (NEVER cargo directly)
1. Maintain backward compatibility with bash
1. Support all four bash environments (Git Bash, WSL, Cygwin, MSYS2)
1. Include comprehensive tests for each environment
1. Follow Rust best practices

## License

MIT OR Apache-2.0
