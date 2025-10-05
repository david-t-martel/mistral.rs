# WinUtils-Core Integration Guide

This guide walks you through integrating the winutils-core enhanced features into existing and new utilities.

## Table of Contents

1. [Basic Integration](#basic-integration)
1. [Enhanced Help System](#enhanced-help-system)
1. [Version and Source Management](#version-and-source-management)
1. [Built-in Testing Framework](#built-in-testing-framework)
1. [Windows-Specific Features](#windows-specific-features)
1. [System Diagnostics](#system-diagnostics)
1. [Complete Examples](#complete-examples)
1. [Best Practices](#best-practices)
1. [Troubleshooting](#troubleshooting)

## Basic Integration

### Step 1: Add Dependencies

Add to your utility's `Cargo.toml`:

```toml
[dependencies]
winutils-core = { workspace = true }
clap = { workspace = true }
# ... other dependencies
```

### Step 2: Basic Structure

```rust
use winutils_core::{
    enhanced_app, handle_enhanced_args,
    HelpSystem, EnhancedHelp, VersionInfo,
    traits::EnhancedUtility,
    WinUtilsResult, WinUtilsError,
};

struct MyUtility {
    help_system: HelpSystem,
    version_info: VersionInfo,
}

impl MyUtility {
    fn new() -> WinUtilsResult<Self> {
        // Create enhanced help (see detailed section below)
        let help = create_my_help();
        
        Ok(Self {
            help_system: HelpSystem::new(help),
            version_info: VersionInfo::new("my-util", "Description of my utility"),
        })
    }
    
    fn run(&mut self, args: Vec<String>) -> WinUtilsResult<()> {
        // Create command-line parser with enhanced features
        let app = enhanced_app!("my-util", "My utility description", env!("CARGO_PKG_VERSION"))
            // Add your specific arguments here
            .arg(clap::Arg::new("input")
                .help("Input file")
                .required(true));
        
        let matches = app.try_get_matches_from(args)?;
        
        // This macro handles all enhanced arguments automatically
        handle_enhanced_args!(matches, self);
        
        // Your utility logic here
        let input = matches.get_one::<String>("input").unwrap();
        self.process_input(input)?;
        
        Ok(())
    }
    
    fn process_input(&self, input: &str) -> WinUtilsResult<()> {
        // Your utility's main logic
        println!("Processing: {}", input);
        Ok(())
    }
}

// Required trait implementation
impl EnhancedUtility for MyUtility {
    fn name(&self) -> &'static str { "my-util" }
    fn description(&self) -> &'static str { "My enhanced utility" }
    fn help_system(&self) -> &HelpSystem { &self.help_system }
    fn version_info(&self) -> &VersionInfo { &self.version_info }
    
    #[cfg(feature = "testing")]
    fn self_test(&self) -> WinUtilsResult<winutils_core::testing::TestResults> {
        // Implement if you want self-testing (optional)
        todo!("Implement self-test or remove this method")
    }
    
    #[cfg(feature = "testing")]
    fn benchmark(&self) -> WinUtilsResult<winutils_core::testing::BenchmarkResults> {
        // Implement if you want benchmarking (optional)
        todo!("Implement benchmark or remove this method")
    }
    
    #[cfg(feature = "diagnostics")]
    fn diagnose(&self) -> WinUtilsResult<winutils_core::testing::DiagnosticResults> {
        // Implement if you want diagnostics (optional)
        todo!("Implement diagnose or remove this method")
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    let mut utility = match MyUtility::new() {
        Ok(util) => util,
        Err(e) => {
            eprintln!("Error initializing utility: {}", e);
            std::process::exit(1);
        }
    };
    
    if let Err(e) = utility.run(args) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
```

## Enhanced Help System

### Creating Rich Help Content

```rust
use winutils_core::{EnhancedHelp, ExampleSet, Example, WindowsNotes};

fn create_my_help() -> EnhancedHelp {
    // Create examples organized by category
    let examples = ExampleSet::new()
        // Basic examples (shown in brief help)
        .add_basic(Example::new(
            "Process a single file",
            "my-util input.txt"
        ).with_output("Processing input.txt... Done!"))
        
        .add_basic(Example::new(
            "Process with verbose output",
            "my-util -v input.txt"
        ).with_output("Verbose: Reading input.txt\nVerbose: Processing...\nDone!"))
        
        // Common use cases
        .add_common_use_case(Example::new(
            "Process multiple files",
            "my-util file1.txt file2.txt file3.txt"
        ).with_notes("Files are processed in the order specified"))
        
        .add_common_use_case(Example::new(
            "Process with output redirection",
            "my-util input.txt > output.txt"
        ).with_notes("Results are written to the specified output file"))
        
        // Advanced examples
        .add_advanced(Example::new(
            "Process with custom configuration",
            "my-util --config custom.conf input.txt"
        ).with_notes("Configuration file overrides default settings"))
        
        // Windows-specific examples
        .add_windows_specific(Example::new(
            "Process with Windows attributes",
            "my-util --windows-attributes input.txt"
        ).with_output("File attributes: Hidden, Archive\nProcessing input.txt... Done!")
        .windows_specific())
        
        // Troubleshooting examples
        .add_troubleshooting(Example::new(
            "Debug processing issues",
            "my-util --diagnose"
        ).with_output("Running diagnostics...\n✓ File permissions OK\n✓ Path handling OK")
        .with_notes("Use when experiencing unexpected behavior"));
    
    // Windows-specific notes
    let windows_notes = WindowsNotes::new()
        .add_path_note("Supports UNC paths: \\\\server\\share\\file.txt")
        .add_path_note("Handles long paths (>260 characters) automatically")
        .add_path_note("Git Bash paths are normalized automatically")
        .add_permissions_note("Respects Windows file permissions and ACLs")
        .add_permissions_note("Hidden files are processed unless --skip-hidden is used")
        .add_file_attributes_note("Can read and preserve Windows file attributes")
        .add_performance_note("Optimized for NTFS file system characteristics")
        .add_compatibility_note("Maintains compatibility with Unix versions");
    
    EnhancedHelp::new(
        "my-util",
        "Process files with advanced features",
        "This utility processes text files with support for various input formats, \
         Windows-specific features, and comprehensive error handling. It provides \
         enhanced functionality for Windows environments while maintaining \
         cross-platform compatibility."
    )
    .with_examples(examples)
    .with_windows_notes(windows_notes)
    .add_see_also("related-util(1)")
    .add_see_also("another-tool(1)")
    .with_author("Your Name <your.email@example.com>")
    .add_custom_section("Configuration", vec![
        "Configuration files are searched in the following order:".to_string(),
        "1. ~/.config/my-util/config.toml".to_string(),
        "2. ./my-util.toml".to_string(),
        "3. Built-in defaults".to_string(),
    ])
}
```

### Help Display Options

Users can access help in multiple ways:

```bash
my-util --help              # Brief help with basic examples
my-util --help-full         # Comprehensive help with all sections
my-util --source            # Source repository information
```

## Version and Source Management

### Basic Version Setup

The version system automatically provides detailed build information:

```rust
let version_info = VersionInfo::new("my-util", "My utility description");

// Users can access:
// --version          # Brief version
// --version-full     # Detailed version with build info
// --features         # List compiled features
// --check-updates    # Check for newer versions
```

### Custom Version Information

```rust
let version_info = VersionInfo::new("my-util", "My utility description")
    .with_color_choice(ColorChoice::Auto);

// The version system automatically includes:
// - Git commit hash and branch
// - Build date and profile
// - Rust version and target
// - Enabled features
// - Optimization flags
```

## Built-in Testing Framework

### Implementing Self-Tests

```rust
use winutils_core::testing::{SelfTest, TestResults, TestResult};
use std::time::Instant;

impl SelfTest for MyUtility {
    fn self_test(&self) -> WinUtilsResult<TestResults> {
        let mut results = TestResults::new(self.test_name());
        
        // Test basic functionality
        let basic_result = self.test_basic_functionality()?;
        results.add_result(basic_result);
        
        // Test path handling
        let path_result = self.test_path_handling()?;
        results.add_result(path_result);
        
        // Custom test
        let start = Instant::now();
        match self.test_custom_feature() {
            Ok(_) => {
                results.add_result(TestResult::success_with_message(
                    "Custom Feature",
                    start.elapsed(),
                    "Custom feature working correctly"
                ));
            }
            Err(e) => {
                results.add_result(TestResult::failure(
                    "Custom Feature",
                    start.elapsed(),
                    format!("Custom feature failed: {}", e)
                ));
            }
        }
        
        Ok(results)
    }
    
    fn test_name(&self) -> &str {
        "My Utility Test Suite"
    }
    
    fn test_custom_feature(&self) -> WinUtilsResult<()> {
        // Your custom test logic here
        Ok(())
    }
}
```

### Implementing Benchmarks

```rust
use winutils_core::testing::{BenchmarkSuite, BenchmarkResults, BenchmarkResult};

impl BenchmarkSuite for MyUtility {
    fn benchmark(&self) -> WinUtilsResult<BenchmarkResults> {
        let mut results = BenchmarkResults::new(self.benchmark_name());
        
        // Basic operation benchmark
        let basic_bench = self.benchmark_basic_operation()?;
        results.add_result(basic_bench);
        
        // Custom benchmark
        let iterations = 1000;
        let mut times = Vec::with_capacity(iterations);
        
        for _ in 0..iterations {
            let start = Instant::now();
            self.perform_operation(); // Your operation to benchmark
            times.push(start.elapsed());
        }
        
        let bench_result = BenchmarkResult::new(
            "Custom Operation",
            iterations as u64,
            &times,
        );
        results.add_result(bench_result);
        
        Ok(results)
    }
    
    fn benchmark_name(&self) -> &str {
        "My Utility Benchmarks"
    }
    
    fn perform_operation(&self) {
        // Operation to benchmark
    }
}
```

Users can run tests and benchmarks:

```bash
my-util --self-test    # Run self-validation tests
my-util --benchmark    # Run performance benchmarks
```

## Windows-Specific Features

### Implementing Windows Enhancements

```rust
use winutils_core::{
    windows::{WindowsHandler, AttributeInfo, AclInfo, ShortcutInfo},
    traits::WindowsEnhanced,
};

impl WindowsEnhanced for MyUtility {
    fn handle_windows_attributes(&self, path: &std::path::Path) -> WinUtilsResult<AttributeInfo> {
        WindowsHandler::get_attributes(path)
    }
    
    fn handle_windows_acl(&self, path: &std::path::Path) -> WinUtilsResult<AclInfo> {
        WindowsHandler::get_acl(path)
    }
    
    fn handle_shortcut(&self, path: &std::path::Path) -> WinUtilsResult<Option<ShortcutInfo>> {
        WindowsHandler::read_shortcut(path)
    }
}

impl MyUtility {
    fn process_file_with_windows_features(&self, path: &std::path::Path) -> WinUtilsResult<()> {
        // Get Windows file attributes
        let attr_info = self.handle_windows_attributes(path)?;
        if attr_info.attributes.hidden {
            println!("Processing hidden file: {}", path.display());
        }
        
        // Check if it's a shortcut
        if let Some(shortcut_info) = self.handle_shortcut(path)? {
            println!("Shortcut target: {}", shortcut_info.target_path.display());
            // Process the target instead
            return self.process_file_with_windows_features(&shortcut_info.target_path);
        }
        
        // Get ACL information if needed
        let acl_info = self.handle_windows_acl(path)?;
        if !acl_info.acl.has_access(0x80000000) { // GENERIC_READ
            return Err(WinUtilsError::permission_denied(
                "read file",
                "read access to file"
            ));
        }
        
        // Process the file...
        Ok(())
    }
}
```

### Registry Integration

```rust
#[cfg(windows)]
use winutils_core::windows::registry::{RegistryKey, get_file_association};

impl MyUtility {
    #[cfg(windows)]
    fn get_file_handler(&self, extension: &str) -> WinUtilsResult<Option<String>> {
        get_file_association(extension)
    }
    
    #[cfg(not(windows))]
    fn get_file_handler(&self, _extension: &str) -> WinUtilsResult<Option<String>> {
        Ok(None) // No registry on non-Windows platforms
    }
}
```

## System Diagnostics

### Implementing Diagnostics

```rust
use winutils_core::diagnostics::{SystemDiagnostics, DiagnosticResults};

impl MyUtility {
    fn setup_diagnostics(&mut self) -> WinUtilsResult<()> {
        #[cfg(feature = "diagnostics")]
        {
            let mut diagnostics = SystemDiagnostics::new("my-util");
            diagnostics.initialize()?;
            self.diagnostics = Some(diagnostics);
        }
        Ok(())
    }
}

// In the EnhancedUtility implementation:
#[cfg(feature = "diagnostics")]
fn diagnose(&self) -> WinUtilsResult<DiagnosticResults> {
    if let Some(ref diagnostics) = self.diagnostics {
        diagnostics.run_diagnostics()
    } else {
        Err(WinUtilsError::diagnostics("Diagnostics not initialized"))
    }
}
```

Users can run diagnostics:

```bash
my-util --diagnose    # Run comprehensive system diagnostics
```

## Complete Examples

### Simple File Processor

See [`examples/enhanced_utility_demo.rs`](examples/enhanced_utility_demo.rs) for a complete example.

### Enhanced ls Utility

See [`coreutils/src/ls/src/enhanced_main.rs`](coreutils/src/ls/src/enhanced_main.rs) for a real-world example.

## Best Practices

### 1. Feature Gates

Use feature gates appropriately:

```rust
#[cfg(feature = "windows-enhanced")]
impl WindowsEnhanced for MyUtility {
    // Windows-specific implementations
}

#[cfg(feature = "testing")]
impl SelfTest for MyUtility {
    // Test implementations
}
```

### 2. Error Handling

Use comprehensive error handling:

```rust
use winutils_core::{WinUtilsResult, WinUtilsError};

fn process_file(&self, path: &str) -> WinUtilsResult<()> {
    let normalized_path = winpath::normalize_path(path)
        .map_err(|e| WinUtilsError::path(format!("Failed to normalize path: {}", e)))?;
    
    let content = std::fs::read_to_string(&normalized_path)
        .map_err(|e| WinUtilsError::io(e))?;
    
    // Process content...
    Ok(())
}
```

### 3. Graceful Degradation

Handle missing features gracefully:

```rust
fn get_windows_info(&self, path: &std::path::Path) -> Option<String> {
    #[cfg(feature = "windows-enhanced")]
    {
        if let Ok(attr_info) = WindowsHandler::get_attributes(path) {
            return Some(attr_info.attributes.description());
        }
    }
    
    None // Feature not available or failed
}
```

### 4. Performance Considerations

Use diagnostics for performance monitoring:

```rust
fn run(&mut self, args: Vec<String>) -> WinUtilsResult<()> {
    #[cfg(feature = "diagnostics")]
    {
        if let Some(ref mut diagnostics) = self.diagnostics {
            diagnostics.start_monitoring()?;
        }
    }
    
    // Your processing logic...
    
    #[cfg(feature = "diagnostics")]
    {
        if let Some(ref mut diagnostics) = self.diagnostics {
            diagnostics.collect_metrics()?;
        }
    }
    
    Ok(())
}
```

## Troubleshooting

### Common Issues

#### 1. Build Failures

**Problem**: Build fails with missing dependencies.

**Solution**: Ensure all required features are enabled:

```toml
[dependencies]
winutils-core = { workspace = true, features = ["help", "version", "testing", "windows-enhanced", "diagnostics"] }
```

#### 2. Feature Not Available

**Problem**: Features show as not available at runtime.

**Solution**: Check feature compilation:

```bash
cargo build --features "help,version,testing,windows-enhanced,diagnostics"
```

#### 3. Windows-Specific Features Not Working

**Problem**: Windows features don't work on Windows.

**Solution**: Ensure Windows-specific dependencies are available:

```toml
[dependencies]
windows-sys = { workspace = true }
windows = { workspace = true }
```

#### 4. Git Information Not Available

**Problem**: Version system shows no git information.

**Solution**: Ensure you're building from a git repository:

```bash
git init  # If not already a git repo
git add .
git commit -m "Initial commit"
```

### Debug Mode

Enable debug logging for troubleshooting:

```rust
// In your utility's main function
if std::env::var("WINUTILS_DEBUG").is_ok() {
    println!("Debug mode enabled");
    // Enable additional logging
}
```

### Testing Integration

Test your integration:

```bash
# Test all enhanced features
my-util --help-full
my-util --version-full
my-util --features
my-util --self-test
my-util --benchmark
my-util --diagnose
```

## Migration Guide

### From Basic Utility

If you have an existing basic utility, follow these steps:

1. **Add winutils-core dependency**
1. **Wrap your existing main logic in the new structure**
1. **Create enhanced help content**
1. **Implement the EnhancedUtility trait**
1. **Add optional features (testing, Windows support, etc.)**
1. **Update documentation and examples**

### Minimal Changes

For minimal integration, you only need:

```rust
// Add this to your existing utility
use winutils_core::{enhanced_app, handle_enhanced_args, EnhancedHelp, HelpSystem, VersionInfo, traits::EnhancedUtility};

// Wrap your existing logic in the structure shown in Basic Integration
// Implement EnhancedUtility trait with minimal methods
// Use enhanced_app! and handle_enhanced_args! macros
```

______________________________________________________________________

This integration guide should help you add comprehensive enhanced features to any Windows utility. For more examples and detailed API documentation, see the [API documentation](https://docs.rs/winutils-core) and [examples directory](examples/).
