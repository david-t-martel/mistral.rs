# WinUtils Core - Enhanced Features for Windows Utilities

A comprehensive enhancement framework for Windows utilities that provides unified help systems, version management, built-in testing, Windows-specific functionality, and system diagnostics.

## Features

### 🆘 Enhanced Help System

- **Comprehensive Documentation**: Rich help with examples, use cases, and troubleshooting
- **Windows-Specific Notes**: Detailed documentation for Windows-specific behavior
- **Man Page Generation**: Automatic generation of man pages from help content
- **Colored Output**: Beautiful, readable help output with syntax highlighting

### 📦 Version and Source Management

- **Detailed Build Information**: Git commit, branch, build date, and compiler details
- **Feature Detection**: Automatic detection and listing of compiled features
- **Update Checking**: Built-in capability to check for newer versions
- **Source Repository Integration**: Direct links to source code and issue tracking

### 🧪 Built-in Testing Framework

- **Self-Validation**: Utilities can test themselves for proper functionality
- **Performance Benchmarks**: Built-in benchmarking capabilities with detailed metrics
- **Integration Testing**: Framework for testing utility integration and compatibility
- **Diagnostic Mode**: Comprehensive system diagnostics and troubleshooting

### 🪟 Windows-Specific Enhancements

- **File Attributes**: Full support for Windows file attributes (Hidden, System, Archive, ReadOnly)
- **ACL Support**: Windows Access Control List handling and permission mapping
- **Shortcut Resolution**: Support for Windows shortcuts (.lnk files)
- **Registry Integration**: Access to Windows Registry for configuration and system information

### 📊 System Diagnostics and Monitoring

- **Performance Monitoring**: Real-time CPU, memory, and disk usage tracking
- **System Information**: Comprehensive system details and configuration
- **Health Checks**: Automated checks for common configuration issues
- **Troubleshooting Tools**: Built-in diagnostics for path handling, permissions, and more

## Quick Start

### Adding to Your Project

Add to your `Cargo.toml`:

```toml
[dependencies]
winutils-core = { path = "path/to/winutils-core", features = ["help", "version", "testing", "windows-enhanced", "diagnostics"] }
```

### Basic Integration

```rust
use winutils_core::{
    enhanced_app, handle_enhanced_args,
    HelpSystem, EnhancedHelp, VersionInfo,
    traits::EnhancedUtility,
    WinUtilsResult,
};

struct MyUtility {
    help_system: HelpSystem,
    version_info: VersionInfo,
}

impl MyUtility {
    fn new() -> WinUtilsResult<Self> {
        let help = EnhancedHelp::new(
            "my-util",
            "My enhanced utility",
            "A detailed description of what my utility does..."
        );
        
        Ok(Self {
            help_system: HelpSystem::new(help),
            version_info: VersionInfo::new("my-util", "My enhanced utility"),
        })
    }
    
    fn run(&mut self, args: Vec<String>) -> WinUtilsResult<()> {
        let app = enhanced_app!("my-util", "My enhanced utility", env!("CARGO_PKG_VERSION"))
            .arg(clap::Arg::new("input")
                .help("Input file")
                .required(true));
        
        let matches = app.try_get_matches_from(args)?;
        
        // Handle enhanced arguments (--help-full, --version-full, --self-test, etc.)
        handle_enhanced_args!(matches, self);
        
        // Your utility logic here...
        Ok(())
    }
}

impl EnhancedUtility for MyUtility {
    fn name(&self) -> &'static str { "my-util" }
    fn description(&self) -> &'static str { "My enhanced utility" }
    fn help_system(&self) -> &HelpSystem { &self.help_system }
    fn version_info(&self) -> &VersionInfo { &self.version_info }
    // ... implement other required methods
}
```

## Feature Documentation

### Enhanced Help System

Create rich help documentation with examples:

```rust
use winutils_core::{EnhancedHelp, ExampleSet, Example, WindowsNotes};

let examples = ExampleSet::new()
    .add_basic(Example::new(
        "Basic usage",
        "my-util input.txt"
    ).with_output("Processing input.txt..."))
    .add_windows_specific(Example::new(
        "Windows-specific feature",
        "my-util --windows-attributes input.txt"
    ).windows_specific());

let windows_notes = WindowsNotes::new()
    .add_path_note("Supports UNC paths and long file names")
    .add_permissions_note("Integrates with Windows ACLs");

let help = EnhancedHelp::new(
    "my-util",
    "Process files with Windows enhancements",
    "Detailed description..."
)
.with_examples(examples)
.with_windows_notes(windows_notes);
```

### Windows-Specific Features

```rust
use winutils_core::windows::WindowsHandler;

// Get Windows file attributes
let attr_info = WindowsHandler::get_attributes(&path)?;
println!("Attributes: {}", attr_info.attributes.description());

// Get Windows ACL information
let acl_info = WindowsHandler::get_acl(&path)?;
println!("Permissions: {}", acl_info.acl.permission_summary());

// Resolve Windows shortcut
if let Some(shortcut) = WindowsHandler::read_shortcut(&path)? {
    println!("Shortcut target: {}", shortcut.target_path.display());
}
```

### Built-in Testing

```rust
use winutils_core::testing::{SelfTest, TestResults, TestResult};

impl SelfTest for MyUtility {
    fn self_test(&self) -> WinUtilsResult<TestResults> {
        let mut results = TestResults::new("my-util");
        
        // Add your tests
        let start = std::time::Instant::now();
        // ... test logic ...
        let duration = start.elapsed();
        
        results.add_result(TestResult::success("Basic Test", duration));
        Ok(results)
    }
    
    fn test_name(&self) -> &str { "My Utility" }
}
```

### System Diagnostics

```rust
use winutils_core::diagnostics::SystemDiagnostics;

let mut diagnostics = SystemDiagnostics::new("my-util");
diagnostics.initialize()?;
diagnostics.start_monitoring()?;

// ... do work ...

let results = diagnostics.run_diagnostics()?;
results.display()?;
```

## Available Features

| Feature              | Description                     | Cargo Feature      |
| -------------------- | ------------------------------- | ------------------ |
| Enhanced Help        | Rich help system with examples  | `help`             |
| Version Management   | Detailed version and build info | `version`          |
| Testing Framework    | Self-tests and benchmarks       | `testing`          |
| Windows Enhancements | ACLs, attributes, shortcuts     | `windows-enhanced` |
| System Diagnostics   | Performance monitoring          | `diagnostics`      |
| Man Page Generation  | Generate man pages              | `man-pages`        |

## Command-Line Arguments

When you integrate winutils-core, your utility automatically gains these enhanced arguments:

| Argument          | Description                           |
| ----------------- | ------------------------------------- |
| `--help-full`     | Show comprehensive help with examples |
| `--version-full`  | Show detailed version information     |
| `--source`        | Show source repository information    |
| `--features`      | List compiled features                |
| `--self-test`     | Run internal validation tests         |
| `--benchmark`     | Run performance benchmarks            |
| `--diagnose`      | Run diagnostic checks                 |
| `--check-updates` | Check for available updates           |

## Platform Support

- **Windows**: Full feature support including Windows-specific enhancements
- **Linux/WSL**: Core features with graceful degradation of Windows-specific functionality
- **macOS**: Core features with graceful degradation of Windows-specific functionality

## Examples

### Complete Utility Example

See [`examples/enhanced_utility_demo.rs`](../../examples/enhanced_utility_demo.rs) for a complete example demonstrating all features.

### Enhanced ls Utility

See [`coreutils/src/ls/src/enhanced_main.rs`](../../coreutils/src/ls/src/enhanced_main.rs) for a real-world example of enhancing the `ls` utility.

## Testing

Run the test suite:

```bash
# Test with all features
cargo test --all-features

# Test specific features
cargo test --features "help,version,testing"

# Run the demo utility
cargo run --example enhanced_utility_demo -- --help-full
```

## Performance

WinUtils Core is designed for minimal overhead:

- **Zero-cost abstractions**: Features you don't use don't impact performance
- **Lazy initialization**: System information is collected only when needed
- **Efficient caching**: Repeated operations are optimized
- **Native Windows APIs**: Direct use of Windows APIs for maximum performance

## Contributing

1. **Fork the repository**
1. **Create a feature branch**: `git checkout -b feature/amazing-feature`
1. **Add tests**: Ensure your changes are well-tested
1. **Update documentation**: Keep README and code comments current
1. **Submit a pull request**: Describe your changes and their benefits

## License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Changelog

### v0.1.0 (Initial Release)

- ✨ Enhanced help system with examples and Windows-specific documentation
- ✨ Version and source identification with git integration
- ✨ Built-in testing framework with self-tests and benchmarks
- ✨ Windows-specific enhancements (ACL, attributes, shortcuts)
- ✨ System diagnostics and performance monitoring
- ✨ Comprehensive error handling and result types
- ✨ Integration macros for easy adoption
- ✨ Cross-platform support with graceful feature degradation

## Related Projects

- [uutils/coreutils](https://github.com/uutils/coreutils) - Cross-platform coreutils implementation
- [winpath](../winpath/) - Windows path normalization library
- [clap](https://docs.rs/clap/) - Command line argument parsing
- [sysinfo](https://docs.rs/sysinfo/) - System information library

______________________________________________________________________

**WinUtils Core** - Making Windows utilities more powerful, discoverable, and maintainable.
