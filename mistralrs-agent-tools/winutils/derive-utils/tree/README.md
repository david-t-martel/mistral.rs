# Windows-Optimized Tree Utility

A high-performance, Windows-optimized directory tree visualization tool with enhanced features beyond the standard `tree` command.

## Features

### Core Functionality

- **Visual Tree Structure**: Display directory hierarchies with Unicode/ASCII box drawing
- **Flexible Output**: Support for text and JSON output formats
- **Configurable Depth**: Limit traversal depth for large directory structures
- **Pattern Matching**: Include/exclude files and directories using glob patterns

### Windows-Specific Enhancements

- **File Attributes**: Display Windows file attributes (Hidden, System, Archive, etc.)
- **Junction Points & Symlinks**: Detect and display reparse points with target information
- **Long Path Support**: Handle paths longer than 260 characters (Windows 10 1607+)
- **Alternate Data Streams**: Show NTFS alternate data streams (ADS) information
- **Console Enhancement**: Proper Unicode and color support in Windows console

### Performance Features

- **Parallel Processing**: Multi-threaded directory traversal for faster scanning
- **Smart Buffering**: Efficient memory usage for large directory trees
- **Cross-Platform**: Optimized for Windows but works on all platforms

## Installation

```bash
# Build from source
cd winutils/derive-utils/tree
cargo build --release

# The binary will be available at:
# target/release/tree.exe (Windows)
# target/release/tree (Unix-like)
```

## Usage

### Basic Usage

```bash
# Show current directory tree
tree

# Show specific directory
tree C:\Windows\System32

# Show only directories
tree -d

# Limit depth to 2 levels
tree -L 2
```

### Windows-Specific Features

```bash
# Show file attributes
tree --attrs

# Show junction points and symbolic links
tree --links

# Show alternate data streams
tree --streams

# Show all hidden and system files
tree -a
```

### Output Options

```bash
# Show file sizes
tree --size

# Show modification times
tree --time

# Use ASCII characters (no Unicode)
tree --ascii

# JSON output
tree --json

# Full paths instead of just names
tree -f
```

### Filtering and Sorting

```bash
# Show only .txt files
tree --ext txt

# Match pattern (supports wildcards)
tree -P "*.rs"

# Ignore pattern
tree -I "*.tmp"

# Sort alphabetically
tree -s

# Sort by modification time
tree -t

# Reverse sort order
tree -r
```

### Advanced Options

```bash
# Enable parallel processing with specific thread count
tree --threads 4

# Disable parallel processing
tree --no-parallel

# Show summary statistics
tree --summary

# Force color output
tree --color always

# Disable colors
tree --color never
```

## Examples

### Basic Directory Tree

```
tree C:\Projects\MyApp
C:\Projects\MyApp/
├── src/
│   ├── main.rs
│   ├── lib.rs
│   └── utils/
│       ├── mod.rs
│       └── helpers.rs
├── tests/
│   └── integration_test.rs
├── Cargo.toml
└── README.md
```

### With File Attributes and Sizes

```
tree --attrs --size C:\Windows\System32
C:\Windows\System32/ [S]
├── calc.exe [A] [776KB]
├── cmd.exe [A] [291KB]
├── notepad.exe [A] [179KB]
└── drivers/ [HS]
    ├── etc/ [HS]
    └── UMDF/ [HS]
```

### JSON Output

```json
{
  "tree": {
    "name": "MyApp",
    "path": "C:\\Projects\\MyApp",
    "is_directory": true,
    "is_symlink": false,
    "children": [
      {
        "name": "src",
        "path": "C:\\Projects\\MyApp\\src",
        "is_directory": true,
        "is_symlink": false,
        "children": [
          {
            "name": "main.rs",
            "path": "C:\\Projects\\MyApp\\src\\main.rs",
            "is_directory": false,
            "is_symlink": false,
            "size": 1234
          }
        ]
      }
    ]
  },
  "stats": {
    "files": 5,
    "directories": 3,
    "total_size": 45678,
    "symlinks": 0,
    "junction_points": 0,
    "hidden_files": 0,
    "errors": 0
  }
}
```

## File Attribute Codes

When using `--attrs`, the following single-letter codes are displayed:

- **H**: Hidden
- **S**: System
- **A**: Archive
- **R**: Read-only
- **C**: Compressed
- **E**: Encrypted
- **T**: Temporary
- **P**: Sparse file
- **L**: Reparse point (junction/symlink)
- **O**: Offline

## Performance

The tree utility is optimized for performance:

- **Parallel processing**: Utilizes multiple CPU cores for directory traversal
- **Memory efficient**: Processes directories incrementally rather than loading everything into memory
- **Windows optimized**: Uses native Windows APIs for maximum performance
- **Smart caching**: Avoids redundant file system calls

### Benchmarks

On a typical Windows system scanning a directory with 10,000 files:

- **Single-threaded**: ~2.5 seconds
- **Multi-threaded (4 cores)**: ~0.8 seconds
- **Memory usage**: \<50MB peak

## Compatibility

- **Windows**: Full feature support including attributes, junction points, and ADS
- **Linux/macOS**: Core functionality with Unix permissions support
- **Minimum Rust version**: 1.70.0

## Build Requirements

- Rust 1.70.0 or later
- Windows SDK (Windows only, for advanced features)

## Error Handling

The utility gracefully handles common errors:

- **Permission denied**: Continues traversal, reports error count
- **Broken symlinks**: Shows as broken link, continues processing
- **Path too long**: Uses long path APIs on Windows when available
- **Unicode filenames**: Properly handles international characters

## Contributing

This utility is part of the Windows-optimized coreutils project. See the main project documentation for contribution guidelines.

## License

MIT OR Apache-2.0
