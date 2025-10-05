# Getting Started with WinUtils

Welcome to WinUtils! This guide will help you get up and running with the Windows-optimized GNU coreutils implementation.

## Table of Contents

1. [Quick Start](#quick-start)
1. [Installation](#installation)
1. [Basic Usage](#basic-usage)
1. [Common Use Cases](#common-use-cases)
1. [Path Handling](#path-handling)
1. [Performance Tips](#performance-tips)
1. [Troubleshooting](#troubleshooting)
1. [Next Steps](#next-steps)

## Quick Start

### 1-Minute Setup

```bash
# Clone the repository
git clone https://github.com/david-t-martel/uutils-windows.git
cd winutils

# Build (MUST use Make, not cargo)
make clean
make release

# Install to local bin
make install

# Verify installation
make validate-all-77
```

### Add to PATH

```powershell
# PowerShell - Add to user PATH
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$newPath = "C:\users\david\.local\bin"
[Environment]::SetEnvironmentVariable("Path", "$userPath;$newPath", "User")

# Restart shell to apply changes
```

## Installation

### System Requirements

- **OS**: Windows 10/11 (version 1903+)
- **Architecture**: x86_64
- **Memory**: 4GB RAM minimum
- **Disk**: 500MB free space
- **Build Tools**: Visual Studio 2019+ or Build Tools

### Pre-built Binaries

Download pre-built binaries from the releases page:

```powershell
# PowerShell download script
$version = "v1.0.0"
$url = "https://github.com/david-t-martel/uutils-windows/releases/download/$version/winutils-$version-windows-x64.zip"
Invoke-WebRequest -Uri $url -OutFile "winutils.zip"
Expand-Archive -Path "winutils.zip" -DestinationPath "C:\users\david\.local\bin"
```

### Building from Source

#### Prerequisites

1. **Rust Toolchain**

   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Add Windows target
   rustup target add x86_64-pc-windows-msvc
   ```

1. **GNU Make** (via Git Bash or MSYS2)

   ```bash
   pacman -S make
   ```

1. **Visual Studio or Build Tools**

   - Download from: https://visualstudio.microsoft.com/downloads/
   - Select "Desktop development with C++"

#### Build Process

```bash
# CRITICAL: Always use Make, never cargo directly
make clean      # Clean previous builds
make release    # Build optimized binaries
make test       # Run test suite
make install    # Install to ~/.local/bin
```

## Basic Usage

### Command Structure

All utilities follow the pattern: `wu-<utility> [options] [arguments]`

```bash
# List files
wu-ls -la

# Concatenate files
wu-cat file1.txt file2.txt

# Count lines
wu-wc -l document.txt

# Sort file
wu-sort input.txt -o output.txt

# Search text
wu-grep "pattern" file.txt
```

### Getting Help

Every utility has comprehensive help:

```bash
# Show help for any utility
wu-ls --help
wu-cat --help
wu-sort --help

# Show version information
wu-ls --version
```

## Common Use Cases

### File Operations

```bash
# List files with details
wu-ls -la C:\Windows

# Copy files
wu-cp source.txt destination.txt
wu-cp -r source_dir/ dest_dir/

# Move/rename files
wu-mv old_name.txt new_name.txt

# Remove files
wu-rm unwanted.txt
wu-rm -rf temp_directory/

# Create directories
wu-mkdir -p path/to/new/directory
```

### Text Processing

```bash
# View file contents
wu-cat file.txt
wu-head -n 20 file.txt
wu-tail -f log.txt

# Search in files
wu-grep "error" *.log
wu-grep -r "TODO" src/

# Count lines/words/bytes
wu-wc -l *.txt
wu-wc -w document.txt

# Sort and unique
wu-sort input.txt | wu-uniq > output.txt
```

### System Information

```bash
# Current directory
wu-pwd

# User information
wu-whoami

# System architecture
wu-arch

# Disk usage
wu-df -h
wu-du -sh *

# Process information
wu-nproc  # Number of processors
```

### Data Processing

```bash
# Sort data
wu-sort -n numbers.txt     # Numeric sort
wu-sort -r file.txt        # Reverse sort
wu-sort -k2 data.csv       # Sort by column

# Filter duplicates
wu-uniq input.txt
wu-uniq -c input.txt       # Count occurrences

# Extract columns
wu-cut -d',' -f1,3 data.csv
wu-cut -c1-10 file.txt

# Transform text
wu-tr 'a-z' 'A-Z' < input.txt
wu-tr -d '\r' < dos.txt > unix.txt
```

## Path Handling

### Universal Path Support

WinUtils automatically handles all Windows path formats:

```bash
# DOS/Windows paths
wu-ls C:\Windows\System32
wu-ls "C:\Program Files"

# WSL paths
wu-ls /mnt/c/Windows
wu-ls /mnt/d/Projects

# Git Bash paths
wu-ls /c/Windows
wu-ls /d/Projects

# Cygwin paths
wu-ls /cygdrive/c/Windows

# UNC paths
wu-ls "\\?\C:\Very\Long\Path\That\Exceeds\260\Characters"
```

### Path Normalization

The built-in winpath library normalizes all paths automatically:

```bash
# All these refer to the same location
wu-ls /mnt/c/Windows
wu-ls /c/Windows
wu-ls C:\Windows
wu-ls C:/Windows

# Output is always consistent
```

### Working with Spaces

```bash
# Quote paths with spaces
wu-ls "C:\Program Files"
wu-ls "/c/Program Files"

# Escape spaces
wu-ls C:\Program\ Files
wu-ls /c/Program\ Files
```

## Performance Tips

### 1. Use Native Operations

```bash
# Fast file copying (uses Windows CopyFileEx)
wu-cp --native large_file.iso backup.iso

# Fast directory listing (uses FindFirstFileEx)
wu-ls --fast C:\Windows
```

### 2. Enable Caching

```bash
# Path caching for repeated operations
export WINUTILS_CACHE=1
wu-find . -name "*.txt"  # First run builds cache
wu-find . -name "*.log"  # Subsequent runs are faster
```

### 3. Parallel Processing

```bash
# Use parallel processing where available
wu-sort --parallel=8 huge_file.txt
wu-grep --threads=4 "pattern" *.log
```

### 4. Memory-Mapped I/O

```bash
# Automatically used for large files (>100MB)
wu-cat large_file.bin  # Uses mmap internally
wu-wc -l huge_log.txt  # Efficient line counting
```

## Troubleshooting

### Common Issues

#### Issue: Command not found

```bash
# Solution: Add to PATH
export PATH="$PATH:C:/users/david/.local/bin"

# Or use full path
C:/users/david/.local/bin/wu-ls
```

#### Issue: Path not normalized

```bash
# Solution: Ensure winpath is working
winpath.exe "/mnt/c/Windows"
# Should output: C:\Windows

# Rebuild if needed
make clean
make release
```

#### Issue: Permission denied

```powershell
# Solution: Run as administrator
Start-Process powershell -Verb RunAs
```

#### Issue: Slow performance

```bash
# Enable optimizations
export WINUTILS_OPTIMIZE=1
export WINUTILS_CACHE=1

# Use release build
make clean
make release  # Not 'make debug'
```

### Getting Help

```bash
# Check utility help
wu-ls --help

# Validate installation
make validate-all-77

# Run diagnostics
winpath.exe --diagnose

# Check version
wu-ls --version --verbose
```

## Environment Variables

### Configuration

```bash
# Enable colored output
export WINUTILS_COLOR=always

# Set cache directory
export WINUTILS_CACHE_DIR="C:/temp/winutils-cache"

# Enable debug output
export WINUTILS_DEBUG=1

# Set performance mode
export WINUTILS_PERF=aggressive
```

### Path Variables

```bash
# Custom PATH for utilities
export WINUTILS_PATH="C:/custom/bin"

# Disable path normalization (not recommended)
export WINUTILS_NO_NORMALIZE=1
```

## Integration

### Git Bash

```bash
# Add to ~/.bashrc
export PATH="$PATH:/c/users/david/.local/bin"
alias ls='wu-ls --color=auto'
alias cat='wu-cat'
alias grep='wu-grep --color=auto'
```

### PowerShell

```powershell
# Add to $PROFILE
$env:Path += ";C:\users\david\.local\bin"
Set-Alias ls wu-ls
Set-Alias cat wu-cat
Set-Alias grep wu-grep
```

### Windows Terminal

```json
// settings.json
{
  "profiles": {
    "defaults": {
      "env": {
        "PATH": "C:\\users\\david\\.local\\bin;%PATH%"
      }
    }
  }
}
```

## Best Practices

### 1. Always Use Make

```bash
# Right
make clean && make release

# Wrong
cargo build  # Never do this!
```

### 2. Quote Paths

```bash
# Good practice
wu-ls "C:\Program Files"
wu-cat "$HOME/My Documents/file.txt"
```

### 3. Use Appropriate Tools

```bash
# Text files: use text utilities
wu-cat text.txt
wu-grep "pattern" *.log

# Binary files: use binary-safe utilities
wu-od binary.exe
wu-hexdump data.bin
```

### 4. Chain Commands

```bash
# Efficient pipeline
wu-cat *.log | wu-grep "ERROR" | wu-sort | wu-uniq -c

# Save intermediate results
wu-find . -type f | wu-tee files.txt | wu-wc -l
```

## Next Steps

### Learn More

1. **Read the API documentation**: [API_REFERENCE.md](../API_REFERENCE.md)
1. **Explore performance guide**: [PERFORMANCE.md](../PERFORMANCE.md)
1. **Check architecture details**: [ARCHITECTURE.md](../ARCHITECTURE.md)

### Advanced Topics

- [Custom utility development](../developer/INTEGRATION.md)
- [Performance optimization](../developer/OPTIMIZATION.md)
- [Testing strategies](../developer/TESTING.md)

### Contributing

- Read [CONTRIBUTING.md](../CONTRIBUTING.md)
- Check open issues on GitHub
- Join discussions

### Support

- **GitHub Issues**: Report bugs or request features
- **Email**: david.martel@auricleinc.com
- **Documentation**: Full docs in `/docs` directory

## Quick Reference Card

```bash
# File Operations
wu-ls -la              # List files
wu-cp src dst          # Copy
wu-mv old new          # Move/rename
wu-rm file             # Remove
wu-mkdir dir           # Create directory

# Text Processing
wu-cat file            # Display file
wu-grep pattern file   # Search
wu-sort file           # Sort
wu-uniq file           # Remove duplicates
wu-wc -l file          # Count lines

# System Info
wu-pwd                 # Current directory
wu-whoami              # Current user
wu-df -h               # Disk space
wu-du -sh dir          # Directory size

# Path Tools
winpath path           # Normalize path
wu-which cmd           # Find command
wu-where file          # Search for file
wu-tree dir            # Directory tree
```

______________________________________________________________________

*Getting Started Guide Version: 1.0.0*
*Last Updated: January 2025*
*Get productive with WinUtils in minutes!*
