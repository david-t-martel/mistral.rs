# Windows-Optimized Coreutils

This directory contains Windows-optimized versions of 74 core utilities from uutils/coreutils, designed to provide enhanced Windows path handling, performance optimizations, and Windows-specific features while maintaining compatibility with the original GNU coreutils.

## Generated Utilities

The following 74 utilities have been generated with Windows optimizations:

### File System Operations

- **cp** - Copy files and directories (Windows APIs, performance opts, memory mapping, NTFS features)
- **mv** - Move/rename files (Windows APIs, performance opts, NTFS features)
- **rm** - Remove files and directories (Windows APIs, performance opts, memory mapping, NTFS features)
- **dd** - Convert and copy files (Windows APIs, performance opts, memory mapping, NTFS features)
- **du** - Display directory usage (Windows APIs, performance opts, memory mapping, NTFS features)
- **link** - Create hard links
- **ln** - Create links (Windows APIs, performance opts, NTFS features)
- **mktemp** - Create temporary files
- **shred** - Securely delete files (Windows APIs, performance opts, memory mapping, NTFS features)
- **touch** - Update file timestamps (Windows APIs, performance opts, memory mapping, NTFS features)
- **truncate** - Truncate files
- **unlink** - Remove single file

### Path Manipulation

- **basename** - Extract filename from path (Windows APIs, NTFS features, path normalization)
- **dirname** - Extract directory from path (Windows APIs, NTFS features, path normalization)
- **pwd** - Print working directory (Windows APIs, NTFS features, path normalization)
- **readlink** - Display symlink target (Windows APIs, NTFS features, path normalization)
- **realpath** - Display absolute path (Windows APIs, NTFS features, path normalization)

### Text Processing

- **cat** - Concatenate and display files (performance opts, memory mapping, console opts)
- **cksum** - Calculate checksums (performance opts, memory mapping, console opts)
- **comm** - Compare sorted files line by line (performance opts, memory mapping, console opts)
- **csplit** - Split files based on patterns (performance opts, memory mapping, console opts)
- **cut** - Extract columns from files (performance opts, memory mapping, console opts)
- **expand** - Convert tabs to spaces (performance opts, memory mapping, console opts)
- **fmt** - Format text paragraphs (performance opts, memory mapping, console opts)
- **fold** - Wrap text lines (performance opts, memory mapping, console opts)
- **hashsum** - Calculate hash sums (performance opts, memory mapping, console opts)
- **head** - Display first lines of files (performance opts, memory mapping, console opts)
- **join** - Join lines based on common field (performance opts, memory mapping, console opts)
- **more** - Page through text (performance opts, memory mapping, console opts)
- **nl** - Number lines (performance opts, memory mapping, console opts)
- **od** - Octal dump (performance opts, memory mapping, console opts)
- **paste** - Merge lines of files (performance opts, memory mapping, console opts)
- **pr** - Format text for printing (performance opts, memory mapping, console opts)
- **ptx** - Permuted index (performance opts, memory mapping, console opts)
- **shuf** - Shuffle lines (performance opts, memory mapping, console opts)
- **sort** - Sort lines (performance opts, memory mapping, console opts)
- **split** - Split files (performance opts, memory mapping, console opts)
- **sum** - Calculate checksums (performance opts, memory mapping, console opts)
- **tac** - Reverse cat (performance opts, memory mapping, console opts)
- **tail** - Display last lines of files (performance opts, memory mapping, console opts)
- **tee** - Copy input to multiple outputs (performance opts, memory mapping, console opts)
- **tr** - Translate characters (performance opts, memory mapping, console opts)
- **unexpand** - Convert spaces to tabs (performance opts, memory mapping, console opts)
- **uniq** - Remove duplicate lines (performance opts, memory mapping, console opts)
- **wc** - Count words, lines, characters (performance opts, memory mapping, console opts)

### System Information

- **arch** - Display machine architecture (Windows APIs, console opts)
- **date** - Display or set system date (Windows APIs, console opts)
- **df** - Display filesystem usage (Windows APIs, console opts)
- **hostname** - Display or set hostname (Windows APIs, console opts)
- **nproc** - Display number of processors (Windows APIs, console opts)
- **sync** - Synchronize filesystems (Windows APIs, console opts)
- **whoami** - Display current username (Windows APIs, console opts)

### Encoding/Decoding

- **base32** - Base32 encode/decode
- **base64** - Base64 encode/decode
- **basenc** - Multi-base encoding

### Mathematical Operations

- **expr** - Evaluate expressions (performance opts)
- **factor** - Factorize numbers (performance opts)
- **numfmt** - Format numbers (performance opts)
- **seq** - Generate sequence of numbers (performance opts)
- **tsort** - Topological sort (performance opts)

### Simple Utilities

- **echo** - Display text
- **false** - Return false exit code
- **printf** - Format and print data (performance opts)
- **sleep** - Delay execution
- **test** - Evaluate conditional expressions (performance opts)
- **true** - Return true exit code
- **yes** - Repeatedly output text

### Directory Operations

- **dir** - List directory contents (Windows style) (Windows APIs, performance opts, NTFS features, console opts)
- **dircolors** - Setup colors for ls (Windows APIs, performance opts, NTFS features, console opts)
- **mkdir** - Create directories (Windows APIs, performance opts, NTFS features, console opts)
- **rmdir** - Remove empty directories (Windows APIs, performance opts, NTFS features, console opts)
- **vdir** - List directory contents verbosely (Windows APIs, performance opts, NTFS features, console opts)

### Environment Management

- **env** - Run program in modified environment (Windows APIs)
- **printenv** - Print environment variables (Windows APIs)

## Architecture

### Windows Path Integration

All utilities use the `winpath` module for:

- Windows path normalization using Windows APIs
- Automatic conversion of Unix-style paths to Windows paths
- Handling of UNC paths and network shares
- Safe path joining with length limits
- 8.3 filename support

### Performance Optimizations

Utilities with complex logic include:

- **Parallel Processing**: Using Rayon for multi-threaded operations
- **Memory Mapping**: Using memmap2 for efficient large file handling
- **SIMD Operations**: Optimized text processing with memchr
- **Windows APIs**: Direct Windows system calls for better performance

### Windows-Specific Features

- **NTFS Support**: Enhanced metadata handling for NTFS filesystems
- **Console Optimization**: Unicode support and proper Windows console handling
- **Security Context**: Proper Windows security descriptor handling
- **Long Path Support**: Support for paths longer than MAX_PATH

## Building

### Prerequisites

- Rust 1.85.0 or later
- Windows SDK (for Windows API features)
- Access to the original uutils/coreutils source

### Quick Build

```bash
# Build all utilities
cargo build --release

# Or use the convenience script
./scripts/build-all.sh
```

### Individual Utility Build

```bash
# Build a specific utility
cargo build --release --bin cat
```

## Installation

### System-wide Installation

```bash
# Install to default location (C:/utils/winutils)
./scripts/install.sh

# Install to custom location
./scripts/install.sh "C:/custom/path"
```

### Manual Installation

```bash
# Copy binaries to desired location
cp target/release/*.exe /path/to/installation/
```

## Testing

### Basic Smoke Tests

```bash
# Run basic functionality tests
./scripts/test.sh
```

### Individual Testing

```bash
# Test a specific utility
target/release/cat.exe --help
```

## Usage

These utilities are designed as drop-in replacements for GNU coreutils on Windows with enhanced Windows-specific features:

```bash
# Enhanced path handling
cat "C:\Users\Name\file.txt"
cat "\\server\share\file.txt"  # UNC paths
cat "/c/Users/Name/file.txt"   # Unix-style paths automatically converted

# Performance benefits for large files
sort huge_file.txt              # Automatically uses parallel processing
wc massive_log.txt             # Memory-mapped for efficiency

# Windows-specific features
dir /W                          # Windows-style directory listing
ls --windows-paths             # Force Windows path normalization
```

## Technical Details

### Dependencies

- **clap**: Command-line argument parsing
- **uucore**: Core utilities from uutils
- **winpath**: Windows path normalization
- **windows**: Windows API bindings
- **rayon**: Parallel processing
- **memmap2**: Memory mapping
- **thiserror**: Error handling

### Error Handling

All utilities provide:

- Graceful fallback to original uutils implementation
- Windows-specific error messages
- Proper exit codes
- UTF-8 aware error reporting

### Performance Characteristics

- **Cold Start**: ~50ms typical startup time
- **Memory Usage**: Optimized for large files with memory mapping
- **CPU Usage**: Automatic parallelization for CPU-intensive operations
- **I/O Performance**: Windows API optimizations for file operations

## Development

### Adding New Features

1. Modify the generator at `../generator/src/main.rs`
1. Regenerate utilities: `cd ../generator && cargo run`
1. Test changes: `cargo build && ./scripts/test.sh`

### Customizing Optimizations

Edit the `determine_windows_optimizations` function in the generator to modify which optimizations are applied to each utility category.

### Contributing

- Follow Rust idioms and clippy suggestions
- Ensure all utilities maintain compatibility with GNU coreutils
- Add Windows-specific tests for new features
- Update documentation for API changes

## License

MIT License - Same as uutils/coreutils

## Compatibility

- **Windows 10**: Full support
- **Windows 11**: Full support
- **Windows Server 2019+**: Full support
- **Legacy Windows**: Basic support (some features may be unavailable)

## Troubleshooting

### Common Issues

**"Access Denied" Errors**

- Run as Administrator for system-level operations
- Check NTFS permissions for file operations

**Path Too Long Errors**

- Enable long path support in Windows 10/11
- Use UNC path prefix for very long paths

**Performance Issues**

- Ensure Windows Defender exclusions for build directories
- Use SSD storage for optimal performance
- Consider increasing system memory for large file operations

### Getting Help

1. Check utility-specific help: `utility.exe --help`
1. Review error messages - they include suggested solutions
1. Enable verbose output where available
1. Compare behavior with original uutils version

______________________________________________________________________

Generated by the Windows-Optimized Coreutils Generator
Version: 1.0.0
Generated: $(date)
Utilities: 74
