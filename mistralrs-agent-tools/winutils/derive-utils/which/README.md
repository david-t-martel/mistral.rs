# which - Cross-Platform Executable Finder

A cross-platform implementation of the `which` command with Windows-specific enhancements.

## Features

- **Cross-platform**: Works on Windows, macOS, and Linux
- **Windows enhancements**:
  - Checks current directory first (Windows convention)
  - Supports PATHEXT environment variable
  - Handles Windows executable extensions (.exe, .bat, .cmd, .ps1, etc.)
  - Normalizes path output for consistency
- **Multiple output modes**: Show first match or all matches
- **Silent operation**: Exit code only without output
- **Standard compatibility**: Compatible with Unix `which` behavior

## Usage

```bash
# Find the first executable in PATH
which cargo

# Find all instances of an executable
which -a python

# Silent mode (exit code only)
which -s some_command
echo $?  # 0 if found, 1 if not found

# Find multiple commands
which cargo rustc

# Read commands from stdin
echo -e "cargo\nrustc\npython" | which --read-alias
```

## Options

- `-a, --all`: Show all matches in PATH (not just the first)
- `-s, --silent`: Silent mode - exit code only (don't print anything)
- `--skip-alias`: Skip shell aliases (placeholder for future shell integration)
- `--skip-functions`: Skip shell functions (placeholder for future shell integration)
- `--read-alias`: Read paths from stdin (one per line)

## Windows-Specific Behavior

### Current Directory Priority

On Windows, the utility checks the current directory first before searching PATH, following Windows convention.

### PATHEXT Support

Respects the Windows `PATHEXT` environment variable for executable extensions. Default extensions include:

- `.COM`, `.EXE`, `.BAT`, `.CMD`
- `.VBS`, `.VBE`, `.JS`, `.JSE`
- `.WSF`, `.WSH`, `.PS1`

### Path Normalization

Converts UNC paths to standard paths and normalizes path separators for consistent output.

## Exit Codes

- `0`: All requested commands were found
- `1`: One or more commands were not found, or an error occurred

## Examples

### Basic Usage

```bash
# Find where cargo is installed
$ which cargo
C:/Users/user/.cargo/bin/cargo.exe

# Find all Python installations
$ which -a python
C:/Python39/python.exe
C:/Users/user/AppData/Local/Programs/Python/Python39/python.exe
```

### Windows-Specific Examples

```bash
# Find batch files in current directory
$ echo "test.bat" > test.bat
$ which test
./test.bat

# Use PATHEXT extensions
$ which notepad
C:/Windows/System32/notepad.exe
```

### Silent Mode

```bash
# Check if a command exists without output
$ which -s nonexistent_command
$ echo $?
1

$ which -s cargo
$ echo $?
0
```

## Building

```bash
# Build the utility
cargo build --release --package which

# Run tests
cargo test --package which

# Install locally
cargo install --path .
```

## Dependencies

- `which` crate: Core PATH searching functionality
- `clap`: Command-line argument parsing
- `anyhow`: Error handling
- Windows-specific:
  - `windows-sys`: Windows API access
  - `winapi-util`: Windows utilities
  - `dunce`: Path canonicalization
  - `path-slash`: Path normalization

## Compatibility

This implementation is designed to be compatible with:

- Unix `which` command behavior
- Windows `where` command behavior
- PowerShell `Get-Command` basic functionality

## Testing

The utility includes comprehensive tests covering:

- Basic command finding
- Windows extension handling
- Path normalization
- All/first match modes
- Error conditions

Run tests with:

```bash
cargo test --package which
```
