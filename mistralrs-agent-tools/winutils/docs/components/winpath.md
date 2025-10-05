# WinPath Component Documentation

## Overview

WinPath is the cornerstone library of the WinUtils project, providing universal path normalization across all Windows environments. It ensures consistent path handling whether running in DOS, WSL, Git Bash, Cygwin, or standard Windows terminals.

## Architecture

```
winpath/
├── Core Components
│   ├── PathNormalizer     # Main normalization interface
│   ├── PathDetector       # Path type detection
│   ├── PathConverter      # Format conversion engine
│   └── PathCache          # LRU caching system
│
├── Platform Support
│   ├── DOS/Windows        # C:\Path\To\File
│   ├── WSL                # /mnt/c/Path/To/File
│   ├── Git Bash           # /c/Path/To/File
│   ├── Cygwin             # /cygdrive/c/Path/To/File
│   └── UNC                # \\?\C:\Very\Long\Path
│
└── Performance Features
    ├── LRU Cache          # <1ms lookups
    ├── SIMD Detection     # Fast pattern matching
    └── Zero-Copy Design   # Minimal allocations
```

## Key Features

### 1. Universal Path Detection

WinPath automatically detects and handles:

- **DOS/Windows Paths**: `C:\Windows\System32`
- **WSL Paths**: `/mnt/c/Windows/System32`
- **Git Bash Paths**: `/c/Windows/System32`
- **Cygwin Paths**: `/cygdrive/c/Windows/System32`
- **UNC Paths**: `\\?\C:\VeryLongPath`
- **Unix Paths**: `/usr/local/bin`
- **Mixed Separators**: `C:/Windows\System32`

### 2. Intelligent Normalization

```rust
// Automatic format detection and conversion
let normalizer = PathNormalizer::new();

// All these normalize to "C:\Windows\System32"
normalizer.normalize("C:\\Windows\\System32");     // DOS
normalizer.normalize("C:/Windows/System32");       // Mixed
normalizer.normalize("/mnt/c/Windows/System32");   // WSL
normalizer.normalize("/c/Windows/System32");       // Git Bash
normalizer.normalize("/cygdrive/c/Windows/System32"); // Cygwin
```

### 3. Performance Optimization

- **LRU Cache**: Recently normalized paths cached for \<1ms access
- **SIMD Pattern Matching**: AVX2-accelerated path detection
- **Zero-Copy Operations**: Minimize memory allocations
- **Lazy Evaluation**: Defer expensive operations

## API Reference

### Core Types

```rust
/// Main path normalizer
pub struct PathNormalizer {
    cache: Option<Cache<String, PathBuf>>,
    options: NormalizationOptions,
}

/// Path type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathType {
    Dos,
    Wsl,
    Cygwin,
    Unc,
    GitBash,
    Unix,
    Unknown,
}

/// Normalization configuration
#[derive(Debug, Clone)]
pub struct NormalizationOptions {
    pub use_cache: bool,
    pub cache_size: usize,
    pub resolve_symlinks: bool,
    pub canonicalize: bool,
    pub long_path_support: bool,
    pub case_sensitive: bool,
}
```

### Primary Functions

#### `normalize()`

Normalizes any path to the native Windows format.

```rust
pub fn normalize(&self, path: &str) -> Result<PathBuf>
```

**Example:**

```rust
let normalizer = PathNormalizer::new();
let path = normalizer.normalize("/mnt/c/Users/David")?;
assert_eq!(path, PathBuf::from("C:\\Users\\David"));
```

#### `detect_type()`

Detects the type of input path.

```rust
pub fn detect_type(&self, path: &str) -> PathType
```

**Example:**

```rust
let normalizer = PathNormalizer::new();
assert_eq!(normalizer.detect_type("/mnt/c/Windows"), PathType::Wsl);
assert_eq!(normalizer.detect_type("C:\\Windows"), PathType::Dos);
```

#### `to_windows()` / `to_unix()` / `to_wsl()`

Convert paths to specific formats.

```rust
pub fn to_windows(&self, path: &str) -> Result<PathBuf>
pub fn to_unix(&self, path: &str) -> Result<PathBuf>
pub fn to_wsl(&self, path: &str) -> Result<PathBuf>
```

**Example:**

```rust
let normalizer = PathNormalizer::new();

// Convert to Windows format
let win = normalizer.to_windows("/mnt/c/Users")?;
assert_eq!(win, PathBuf::from("C:\\Users"));

// Convert to WSL format
let wsl = normalizer.to_wsl("C:\\Users")?;
assert_eq!(wsl, PathBuf::from("/mnt/c/Users"));
```

## Usage Examples

### Basic Usage

```rust
use winpath::PathNormalizer;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let normalizer = PathNormalizer::new();

    // Normalize various path formats
    let paths = vec![
        "/c/Windows/System32",
        "/mnt/c/Windows/System32",
        "C:\\Windows\\System32",
        "C:/Windows/System32",
    ];

    for path in paths {
        let normalized = normalizer.normalize(path)?;
        println!("{} -> {}", path, normalized.display());
    }

    Ok(())
}
```

### With Caching

```rust
use winpath::{PathNormalizer, NormalizationOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = NormalizationOptions {
        use_cache: true,
        cache_size: 1024,  // Cache up to 1024 paths
        ..Default::default()
    };

    let mut normalizer = PathNormalizer::with_options(options);

    // First call: ~5ms (cache miss)
    let path1 = normalizer.normalize("/mnt/c/Windows")?;

    // Second call: <1ms (cache hit)
    let path2 = normalizer.normalize("/mnt/c/Windows")?;

    // Get cache statistics
    let stats = normalizer.cache_stats();
    println!("Cache hits: {}, misses: {}", stats.hits, stats.misses);

    Ok(())
}
```

### Long Path Support

```rust
use winpath::{PathNormalizer, NormalizationOptions};

fn handle_long_paths() -> Result<(), Box<dyn std::error::Error>> {
    let options = NormalizationOptions {
        long_path_support: true,  // Enable >260 char support
        ..Default::default()
    };

    let normalizer = PathNormalizer::with_options(options);

    let long_path = "C:\\Very\\Long\\Path\\That\\Exceeds\\The\\Traditional\\260\\Character\\Windows\\Path\\Length\\Limit\\...";

    // Automatically adds \\?\ prefix for long paths
    let normalized = normalizer.normalize(long_path)?;
    assert!(normalized.to_string_lossy().starts_with("\\\\?\\"));

    Ok(())
}
```

### Environment Detection

```rust
use winpath::detection::{is_wsl_environment, is_git_bash_environment};

fn detect_environment() {
    if is_wsl_environment() {
        println!("Running in WSL");
        // Use WSL-specific path handling
    } else if is_git_bash_environment() {
        println!("Running in Git Bash");
        // Use Git Bash-specific path handling
    } else {
        println!("Running in standard Windows");
        // Use standard Windows path handling
    }
}
```

## Performance Characteristics

### Benchmarks

| Operation         | Cold Cache | Warm Cache | Notes                              |
| ----------------- | ---------- | ---------- | ---------------------------------- |
| Simple Path       | 5.2 µs     | 0.8 µs     | e.g., "C:\\Windows"                |
| Complex Path      | 12.4 µs    | 0.9 µs     | e.g., "/mnt/c/Program Files (x86)" |
| Long Path         | 18.7 µs    | 1.1 µs     | >260 characters                    |
| Pattern Detection | 2.1 µs     | N/A        | SIMD-accelerated                   |

### Memory Usage

| Component                | Memory  | Notes                 |
| ------------------------ | ------- | --------------------- |
| Base Library             | 128 KB  | Core functionality    |
| LRU Cache (1024 entries) | 256 KB  | Configurable size     |
| Path Buffer              | 4 KB    | Per operation         |
| Total Typical            | ~400 KB | With default settings |

### Optimization Techniques

1. **SIMD Pattern Matching**

   ```rust
   #[cfg(target_feature = "avx2")]
   fn detect_separator_simd(path: &[u8]) -> bool {
       // AVX2-accelerated separator detection
   }
   ```

1. **Zero-Copy Slicing**

   ```rust
   fn extract_drive_letter(path: &str) -> Option<&str> {
       // Return slice without allocation
       if path.len() >= 2 && path.as_bytes()[1] == b':' {
           Some(&path[0..1])
       } else {
           None
       }
   }
   ```

1. **Lazy Canonicalization**

   ```rust
   fn normalize_lazy(path: &str) -> Cow<str> {
       if needs_normalization(path) {
           Cow::Owned(normalize_full(path))
       } else {
           Cow::Borrowed(path)
       }
   }
   ```

## Configuration

### Environment Variables

```bash
# Enable debug output
WINPATH_DEBUG=1

# Set cache size
WINPATH_CACHE_SIZE=2048

# Disable caching
WINPATH_NO_CACHE=1

# Force case sensitivity
WINPATH_CASE_SENSITIVE=1
```

### Runtime Configuration

```rust
use winpath::{NormalizationOptions, CaseSensitivity};

let options = NormalizationOptions {
    use_cache: true,
    cache_size: 2048,
    resolve_symlinks: true,
    canonicalize: true,
    long_path_support: true,
    case_sensitive: false,
};

let normalizer = PathNormalizer::with_options(options);
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("Invalid path format: {0}")]
    InvalidFormat(String),

    #[error("Path not found: {0}")]
    NotFound(PathBuf),

    #[error("Path too long: {0} characters (max: {1})")]
    TooLong(usize, usize),

    #[error("Invalid characters in path: {0}")]
    InvalidCharacters(String),

    #[error("Cannot normalize path: {0}")]
    NormalizationFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

### Error Recovery

```rust
use winpath::{PathNormalizer, PathError};

fn handle_path_safely(path: &str) -> String {
    let normalizer = PathNormalizer::new();

    match normalizer.normalize(path) {
        Ok(normalized) => normalized.to_string_lossy().into_owned(),
        Err(PathError::NotFound(_)) => {
            // Try without canonicalization
            normalizer.normalize_no_canonicalize(path)
                .unwrap_or_else(|_| path.to_string())
        }
        Err(PathError::TooLong(_, _)) => {
            // Add UNC prefix for long paths
            format!("\\\\?\\{}", path)
        }
        Err(_) => {
            // Fallback to original
            path.to_string()
        }
    }
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_detection() {
        assert_eq!(detect_type("C:\\Windows"), PathType::Dos);
        assert_eq!(detect_type("/mnt/c/Windows"), PathType::Wsl);
        assert_eq!(detect_type("/c/Windows"), PathType::GitBash);
    }

    #[test]
    fn test_normalization() {
        let normalizer = PathNormalizer::new();

        let test_cases = vec![
            ("/c/Windows", "C:\\Windows"),
            ("/mnt/c/Users", "C:\\Users"),
            ("C:/Program Files", "C:\\Program Files"),
        ];

        for (input, expected) in test_cases {
            let result = normalizer.normalize(input).unwrap();
            assert_eq!(result, PathBuf::from(expected));
        }
    }
}
```

### Integration Tests

```rust
#[test]
fn test_git_bash_integration() {
    if !is_git_bash_environment() {
        return; // Skip if not in Git Bash
    }

    let normalizer = PathNormalizer::new();
    let home = std::env::var("HOME").unwrap();

    let normalized = normalizer.normalize(&home).unwrap();
    assert!(normalized.exists());
}
```

## Troubleshooting

### Common Issues

1. **Cache Not Working**

   ```rust
   // Ensure cache is enabled
   let options = NormalizationOptions {
       use_cache: true,  // Must be true
       cache_size: 1024, // Must be > 0
       ..Default::default()
   };
   ```

1. **Long Paths Failing**

   ```rust
   // Enable long path support
   let options = NormalizationOptions {
       long_path_support: true,
       ..Default::default()
   };
   ```

1. **Case Sensitivity Issues**

   ```rust
   // Match Windows case-insensitive behavior
   let options = NormalizationOptions {
       case_sensitive: false,  // Windows default
       ..Default::default()
   };
   ```

## Migration Guide

### From std::path

```rust
// Before: Using std::path
use std::path::{Path, PathBuf};

let path = PathBuf::from(user_input);
let canonical = path.canonicalize()?;

// After: Using winpath
use winpath::PathNormalizer;

let normalizer = PathNormalizer::new();
let normalized = normalizer.normalize(user_input)?;
```

### From path-slash

```rust
// Before: Using path-slash
use path_slash::PathExt;

let path = Path::new(input).to_slash().unwrap();

// After: Using winpath
use winpath::PathNormalizer;

let normalizer = PathNormalizer::new();
let normalized = normalizer.to_unix(input)?;
```

## Future Enhancements

### Planned Features

1. **Network Path Support**: SMB/CIFS path handling
1. **Cloud Storage Paths**: OneDrive, Google Drive integration
1. **Symbolic Link Resolution**: Advanced symlink handling
1. **Path Validation**: Comprehensive validation rules
1. **Async API**: Non-blocking normalization for large batches

### Performance Roadmap

- SIMD optimization for all detection patterns
- Parallel batch normalization
- Persistent cache with disk backing
- GPU-accelerated pattern matching for massive datasets

______________________________________________________________________

*WinPath Component Documentation Version: 1.0.0*
*Last Updated: January 2025*
*The foundation of universal Windows path handling*
