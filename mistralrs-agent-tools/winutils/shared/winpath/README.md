# WinPath - Comprehensive Windows Path Normalization Library

[![Rust](https://img.shields.io/badge/rust-1.85%2B-blue.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Docs](https://docs.rs/winpath/badge.svg)](https://docs.rs/winpath)

A high-performance Rust library for normalizing Windows paths from various formats including DOS, WSL, Cygwin, UNC, and mixed formats. Designed for cross-platform tools that need to handle Windows paths consistently.

## Features

- **Universal Format Support**: Handles DOS (`C:\`), WSL (`/mnt/c/`), Cygwin (`/cygdrive/c/`), UNC (`\\?\`), and mixed formats
- **Zero-Copy Optimization**: Uses `Cow<str>` to avoid allocations when paths are already normalized
- **Thread-Safe Caching**: Optional LRU cache for improved performance with repeated operations
- **Long Path Support**: Automatic handling of paths >260 characters with UNC prefix
- **Type Safety**: Strong typing with compile-time path format detection
- **Unicode Support**: Proper handling of Unicode paths with optional normalization
- **Performance Optimized**: SIMD-accelerated operations and efficient algorithms

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
winpath = "0.1.0"

# Optional features
winpath = { version = "0.1.0", features = ["cache", "unicode", "serde"] }
```

## Basic Usage

```rust
use winpath::{normalize_path, PathNormalizer, detect_path_format};

// Quick normalization
let normalized = normalize_path("/mnt/c/users/david")?;
assert_eq!(normalized, r"C:\users\david");

// Using the normalizer with caching
let normalizer = PathNormalizer::new();
let result = normalizer.normalize("/cygdrive/c/program files/app")?;
assert_eq!(result.path(), r"C:\program files\app");

// Format detection
let format = detect_path_format("/mnt/c/users");
println!("Detected format: {:?}", format); // PathFormat::Wsl
```

## Supported Path Formats

| Format      | Example                   | Description                              |
| ----------- | ------------------------- | ---------------------------------------- |
| DOS         | `C:\Users\David`          | Standard Windows paths                   |
| DOS Forward | `C:/Users/David`          | Windows paths with forward slashes       |
| WSL         | `/mnt/c/users/david`      | Windows Subsystem for Linux mount points |
| Cygwin      | `/cygdrive/c/users/david` | Cygwin-style drive paths                 |
| UNC         | `\\?\C:\Users\David`      | Long path support format                 |
| Unix-like   | `//c/users/david`         | Unix-style with drive letters            |
| Mixed       | `C:\Users/David\file.txt` | Combination of separators                |

All formats pointing to the same location are normalized to the canonical Windows format.

## Advanced Usage

### Zero-Copy Optimization

```rust
use winpath::normalize_path_cow;

// No allocation needed - input is already normalized
let result = normalize_path_cow(r"C:\Users\David")?;
assert!(!result.was_modified());

// Allocation needed - input requires normalization
let result = normalize_path_cow("/mnt/c/users/david")?;
assert!(result.was_modified());
```

### Batch Processing

```rust
let normalizer = PathNormalizer::new();
let paths = vec![
    "/mnt/c/users/alice",
    "/cygdrive/d/projects",
    "C:/Windows/System32",
];

let normalized_paths = normalizer.normalize_batch(&paths)?;
```

### Custom Configuration

```rust
use winpath::{PathNormalizer, NormalizerConfig};

let config = NormalizerConfig {
    cache_enabled: true,
    cache_size: 2048,
    auto_long_prefix: true,
    validate_components: true,
    unicode_normalize: true, // Requires "unicode" feature
    ..Default::default()
};

let normalizer = PathNormalizer::with_config(config);
```

### Long Path Handling

```rust
// Paths longer than 260 characters automatically get UNC prefix
let long_path = format!("/mnt/c/{}", "very_long_directory_name/".repeat(20));
let result = normalize_path(&long_path)?;
assert!(result.starts_with(r"\\?\C:\"));
```

## Performance

The library is optimized for high-performance scenarios:

- **SIMD-accelerated** string operations where available
- **LRU caching** for repeated path operations
- **Zero-copy** optimization for already-normalized paths
- **Efficient algorithms** for path parsing and validation

### Benchmarks

```
Path Format Detection    : ~50ns per path
WSL Path Normalization  : ~150ns per path
Cached Normalization    : ~25ns per path (cache hit)
Batch Processing        : ~100ns per path (100 paths)
```

Run benchmarks with:

```bash
cargo bench --features cache
```

## Error Handling

The library provides detailed error information:

```rust
use winpath::{normalize_path, PathError};

match normalize_path("/mnt/invalid_drive/test") {
    Ok(path) => println!("Normalized: {}", path),
    Err(PathError::InvalidDriveLetter(c)) => {
        println!("Invalid drive letter: {}", c);
    }
    Err(e) => println!("Other error: {}", e),
}
```

## Feature Flags

- `std` (default): Enable standard library features
- `cache`: Enable LRU caching support
- `unicode`: Enable Unicode normalization
- `serde`: Enable serialization support

## Platform Support

- **Windows**: Full native support with Windows APIs
- **Linux/macOS**: Cross-platform path normalization (without Windows-specific features)
- **WASM**: Basic path normalization support

## Thread Safety

All types are thread-safe and can be shared across threads:

```rust
use std::sync::Arc;
use std::thread;

let normalizer = Arc::new(PathNormalizer::new());

let handles: Vec<_> = (0..10)
    .map(|i| {
        let normalizer = normalizer.clone();
        thread::spawn(move || {
            normalizer.normalize(&format!("/mnt/c/test{}", i))
        })
    })
    .collect();

for handle in handles {
    let result = handle.join().unwrap()?;
    println!("Normalized: {}", result.path());
}
```

## Integration Examples

### With `std::path::PathBuf`

```rust
use std::path::PathBuf;
use winpath::normalize_path;

let wsl_path = "/mnt/c/users/david/documents";
let normalized = normalize_path(wsl_path)?;
let path_buf = PathBuf::from(normalized);
```

### With `clap` CLI Arguments

```rust
use clap::Parser;
use winpath::PathNormalizer;

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    input_path: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let normalizer = PathNormalizer::new();
    let normalized = normalizer.normalize_to_string(&args.input_path)?;
    println!("Normalized path: {}", normalized);
    Ok(())
}
```

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built for the [uutils/coreutils](https://github.com/uutils/coreutils) project
- Inspired by cross-platform path handling challenges in Unix-like tools on Windows
- Performance optimizations guided by real-world usage patterns

______________________________________________________________________

For more examples and detailed API documentation, see the [docs](https://docs.rs/winpath).
