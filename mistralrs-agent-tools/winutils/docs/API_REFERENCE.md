# WinUtils API Reference

## Table of Contents

1. [WinPath API](#winpath-api)
1. [WinUtils-Core API](#winutils-core-api)
1. [Utility Common APIs](#utility-common-apis)
1. [Derive Utilities APIs](#derive-utilities-apis)
1. [Error Types](#error-types)
1. [Traits and Interfaces](#traits-and-interfaces)
1. [Configuration APIs](#configuration-apis)
1. [Performance APIs](#performance-apis)
1. [Testing APIs](#testing-apis)
1. [Windows-Specific APIs](#windows-specific-apis)

## WinPath API

The winpath library provides universal path normalization across all Windows environments.

### Core Module (`winpath`)

```rust
use winpath::{PathNormalizer, PathType, NormalizationOptions};
```

#### `PathNormalizer`

Main interface for path normalization operations.

````rust
pub struct PathNormalizer {
    cache: Option<Cache<String, PathBuf>>,
    options: NormalizationOptions,
}

impl PathNormalizer {
    /// Create a new PathNormalizer with default options
    pub fn new() -> Self;

    /// Create with custom options
    pub fn with_options(options: NormalizationOptions) -> Self;

    /// Normalize any path to native Windows format
    ///
    /// # Examples
    /// ```
    /// let normalizer = PathNormalizer::new();
    /// let path = normalizer.normalize("/mnt/c/Windows")?;
    /// assert_eq!(path, PathBuf::from("C:\\Windows"));
    /// ```
    pub fn normalize(&self, path: &str) -> Result<PathBuf>;

    /// Detect the type of input path
    pub fn detect_type(&self, path: &str) -> PathType;

    /// Convert path to Windows native format
    pub fn to_windows(&self, path: &str) -> Result<PathBuf>;

    /// Convert path to Unix format
    pub fn to_unix(&self, path: &str) -> Result<PathBuf>;

    /// Convert path to WSL format
    pub fn to_wsl(&self, path: &str) -> Result<PathBuf>;

    /// Clear the internal cache
    pub fn clear_cache(&mut self);

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats;
}
````

#### `PathType`

Enumeration of supported path types.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathType {
    /// DOS/Windows paths (C:\Windows)
    Dos,
    /// WSL paths (/mnt/c/Windows)
    Wsl,
    /// Cygwin paths (/cygdrive/c/Windows)
    Cygwin,
    /// UNC paths (\\?\C:\Windows)
    Unc,
    /// Git Bash paths (/c/Windows)
    GitBash,
    /// Unix paths (/usr/bin)
    Unix,
    /// Unknown or invalid path format
    Unknown,
}

impl PathType {
    /// Check if path type is Windows-native
    pub fn is_windows(&self) -> bool;

    /// Check if path type is Unix-like
    pub fn is_unix(&self) -> bool;

    /// Get the path separator for this type
    pub fn separator(&self) -> &str;
}
```

#### `NormalizationOptions`

Configuration for path normalization behavior.

```rust
#[derive(Debug, Clone)]
pub struct NormalizationOptions {
    /// Enable LRU caching
    pub use_cache: bool,
    /// Cache size (number of entries)
    pub cache_size: usize,
    /// Resolve symbolic links
    pub resolve_symlinks: bool,
    /// Canonicalize paths (make absolute)
    pub canonicalize: bool,
    /// Handle long paths (>260 chars)
    pub long_path_support: bool,
    /// Case sensitivity mode
    pub case_sensitive: bool,
}

impl Default for NormalizationOptions {
    fn default() -> Self {
        Self {
            use_cache: true,
            cache_size: 1024,
            resolve_symlinks: false,
            canonicalize: true,
            long_path_support: true,
            case_sensitive: false,
        }
    }
}
```

### Detection Module (`winpath::detection`)

```rust
use winpath::detection::{detect_path_type, is_absolute, has_drive_letter};
```

#### Path Detection Functions

```rust
/// Detect the type of a given path string
pub fn detect_path_type(path: &str) -> PathType;

/// Check if a path is absolute
pub fn is_absolute(path: &str) -> bool;

/// Check if path contains a Windows drive letter
pub fn has_drive_letter(path: &str) -> bool;

/// Check if path is a UNC path
pub fn is_unc_path(path: &str) -> bool;

/// Extract drive letter from path if present
pub fn extract_drive_letter(path: &str) -> Option<char>;

/// Check if running in WSL environment
pub fn is_wsl_environment() -> bool;

/// Check if running in Git Bash environment
pub fn is_git_bash_environment() -> bool;

/// Check if running in Cygwin environment
pub fn is_cygwin_environment() -> bool;
```

### Cache Module (`winpath::cache`)

```rust
use winpath::cache::{Cache, CacheStats, CacheEntry};
```

#### `Cache<K, V>`

LRU cache implementation for path normalization.

```rust
pub struct Cache<K, V> {
    capacity: usize,
    store: LruCache<K, V>,
}

impl<K: Hash + Eq, V: Clone> Cache<K, V> {
    /// Create new cache with specified capacity
    pub fn new(capacity: usize) -> Self;

    /// Insert or update entry
    pub fn insert(&mut self, key: K, value: V) -> Option<V>;

    /// Get entry by key
    pub fn get(&mut self, key: &K) -> Option<&V>;

    /// Remove entry
    pub fn remove(&mut self, key: &K) -> Option<V>;

    /// Clear all entries
    pub fn clear(&mut self);

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats;
}
```

#### `CacheStats`

Statistics for cache performance monitoring.

```rust
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub current_size: usize,
    pub capacity: usize,
}

impl CacheStats {
    /// Calculate hit ratio
    pub fn hit_ratio(&self) -> f64;

    /// Get memory usage estimate
    pub fn memory_usage(&self) -> usize;
}
```

## WinUtils-Core API

Enhanced features framework for all utilities.

### Help System (`winutils_core::help`)

```rust
use winutils_core::help::{HelpBuilder, HelpFormat, HelpSection};
```

#### `HelpBuilder`

Unified help system for consistent documentation.

```rust
pub struct HelpBuilder {
    name: String,
    version: String,
    description: String,
    sections: Vec<HelpSection>,
}

impl HelpBuilder {
    /// Create new help builder
    pub fn new(name: &str) -> Self;

    /// Set version
    pub fn version(mut self, version: &str) -> Self;

    /// Set description
    pub fn description(mut self, desc: &str) -> Self;

    /// Add usage section
    pub fn usage(mut self, usage: &str) -> Self;

    /// Add options section
    pub fn options(mut self, options: Vec<(&str, &str)>) -> Self;

    /// Add examples section
    pub fn examples(mut self, examples: Vec<(&str, &str)>) -> Self;

    /// Build help text
    pub fn build(&self, format: HelpFormat) -> String;

    /// Build and print help
    pub fn print(&self, format: HelpFormat);
}
```

### Version System (`winutils_core::version`)

```rust
use winutils_core::version::{Version, VersionInfo, GitInfo};
```

#### `Version`

Semantic versioning with Git integration.

```rust
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
    pub build_metadata: Option<String>,
}

impl Version {
    /// Parse from string
    pub fn parse(s: &str) -> Result<Self>;

    /// Get current crate version
    pub fn current() -> Self;

    /// Format as string
    pub fn to_string(&self) -> String;

    /// Check compatibility
    pub fn is_compatible_with(&self, other: &Version) -> bool;
}

/// Get complete version information including Git details
pub fn get_version_info() -> VersionInfo {
    VersionInfo {
        version: Version::current(),
        git: GitInfo::current(),
        build_date: compile_time_date(),
        rust_version: rust_version(),
        target: target_triple(),
    }
}
```

### Testing Framework (`winutils_core::testing`)

```rust
use winutils_core::testing::{TestCase, TestRunner, TestResult};
```

#### `TestCase`

Test case definition and execution.

```rust
pub struct TestCase {
    pub name: String,
    pub description: String,
    pub test_fn: Box<dyn Fn() -> TestResult>,
}

impl TestCase {
    /// Create new test case
    pub fn new(name: &str, test_fn: impl Fn() -> TestResult + 'static) -> Self;

    /// Run the test
    pub fn run(&self) -> TestResult;

    /// Run with timeout
    pub fn run_with_timeout(&self, timeout: Duration) -> TestResult;
}

pub enum TestResult {
    Pass,
    Fail(String),
    Skip(String),
    Timeout,
}
```

### Windows Enhancements (`winutils_core::windows`)

```rust
use winutils_core::windows::{WindowsAttributes, Registry, ProcessInfo};
```

#### `WindowsAttributes`

Extended file attributes for Windows.

```rust
pub struct WindowsAttributes {
    pub readonly: bool,
    pub hidden: bool,
    pub system: bool,
    pub archive: bool,
    pub compressed: bool,
    pub encrypted: bool,
    pub temporary: bool,
    pub offline: bool,
}

impl WindowsAttributes {
    /// Get attributes for a file
    pub fn from_path(path: &Path) -> Result<Self>;

    /// Set attributes on a file
    pub fn apply_to_path(&self, path: &Path) -> Result<()>;

    /// Convert to Windows DWORD flags
    pub fn to_flags(&self) -> u32;

    /// Parse from Windows DWORD flags
    pub fn from_flags(flags: u32) -> Self;
}
```

#### `Registry`

Windows Registry access.

```rust
pub struct Registry;

impl Registry {
    /// Read a registry value
    pub fn read_value(key: &str, value_name: &str) -> Result<String>;

    /// Write a registry value
    pub fn write_value(key: &str, value_name: &str, data: &str) -> Result<()>;

    /// Check if key exists
    pub fn key_exists(key: &str) -> bool;

    /// Enumerate subkeys
    pub fn enumerate_keys(key: &str) -> Result<Vec<String>>;
}
```

### Diagnostics (`winutils_core::diagnostics`)

```rust
use winutils_core::diagnostics::{Timer, MemoryTracker, PerfCounter};
```

#### `Timer`

High-precision timing for performance measurement.

```rust
pub struct Timer {
    start: Instant,
    laps: Vec<Duration>,
}

impl Timer {
    /// Start a new timer
    pub fn start() -> Self;

    /// Record a lap time
    pub fn lap(&mut self) -> Duration;

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration;

    /// Get all lap times
    pub fn laps(&self) -> &[Duration];

    /// Print timing report
    pub fn report(&self, label: &str);
}
```

## Utility Common APIs

APIs shared across all utilities.

### Error Handling

```rust
use winutils_core::error::{UtilityError, ErrorKind, Result};
```

#### `UtilityError`

Standard error type for all utilities.

```rust
#[derive(Debug, thiserror::Error)]
pub enum UtilityError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path error: {0}")]
    Path(PathError),

    #[error("Windows API error: {code}: {message}")]
    Windows { code: u32, message: String },

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Custom(String),
}

impl UtilityError {
    /// Get error code for exit status
    pub fn exit_code(&self) -> i32;

    /// Create from Windows error code
    pub fn from_windows_error(code: u32) -> Self;

    /// Create from last OS error
    pub fn from_last_os_error() -> Self;
}
```

### Configuration

```rust
use winutils_core::config::{Config, ConfigBuilder, OutputFormat};
```

#### `Config`

Common configuration for utilities.

```rust
#[derive(Debug, Clone)]
pub struct Config {
    pub verbose: bool,
    pub quiet: bool,
    pub color: ColorChoice,
    pub output_format: OutputFormat,
    pub follow_symlinks: bool,
    pub recursive: bool,
    pub force: bool,
}

impl Config {
    /// Create from command-line arguments
    pub fn from_args(args: &Args) -> Result<Self>;

    /// Create builder for custom configuration
    pub fn builder() -> ConfigBuilder;

    /// Load from file
    pub fn from_file(path: &Path) -> Result<Self>;

    /// Save to file
    pub fn save(&self, path: &Path) -> Result<()>;
}
```

## Derive Utilities APIs

APIs specific to Windows derive utilities.

### Where Utility API

```rust
use where_util::{WhereCommand, SearchOptions, SearchResult};
```

#### `WhereCommand`

Enhanced path search utility.

```rust
pub struct WhereCommand {
    patterns: Vec<String>,
    options: SearchOptions,
}

impl WhereCommand {
    /// Create new where command
    pub fn new(patterns: Vec<String>) -> Self;

    /// Set search options
    pub fn with_options(mut self, options: SearchOptions) -> Self;

    /// Execute search
    pub fn execute(&self) -> Result<Vec<SearchResult>>;

    /// Execute and print results
    pub fn run(&self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub search_path: Option<String>,
    pub recursive: bool,
    pub show_size: bool,
    pub show_time: bool,
    pub quiet: bool,
    pub format: OutputFormat,
}

#[derive(Debug)]
pub struct SearchResult {
    pub path: PathBuf,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
    pub attributes: Option<WindowsAttributes>,
}
```

### Which Utility API

```rust
use which_util::{WhichCommand, WhichOptions};
```

#### `WhichCommand`

Command location utility.

```rust
pub struct WhichCommand {
    commands: Vec<String>,
    options: WhichOptions,
}

impl WhichCommand {
    /// Find single command
    pub fn which(command: &str) -> Result<PathBuf>;

    /// Find all instances of command
    pub fn which_all(command: &str) -> Result<Vec<PathBuf>>;

    /// Check if command exists
    pub fn exists(command: &str) -> bool;

    /// Find with custom options
    pub fn find_with_options(
        command: &str,
        options: &WhichOptions
    ) -> Result<Vec<PathBuf>>;
}

#[derive(Debug, Clone)]
pub struct WhichOptions {
    pub search_path: Option<String>,
    pub show_all: bool,
    pub silent: bool,
    pub show_type: bool,
}
```

### Tree Utility API

```rust
use tree_util::{TreeCommand, TreeOptions, TreeNode};
```

#### `TreeCommand`

Directory tree visualization.

```rust
pub struct TreeCommand {
    root: PathBuf,
    options: TreeOptions,
}

impl TreeCommand {
    /// Create tree from directory
    pub fn new(root: impl Into<PathBuf>) -> Self;

    /// Build tree structure
    pub fn build(&self) -> Result<TreeNode>;

    /// Render tree to string
    pub fn render(&self) -> Result<String>;

    /// Print tree with formatting
    pub fn print(&self) -> Result<()>;

    /// Walk tree with callback
    pub fn walk<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(&TreeNode, usize) -> WalkResult;
}

#[derive(Debug)]
pub struct TreeNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone)]
pub struct TreeOptions {
    pub max_depth: Option<usize>,
    pub show_hidden: bool,
    pub show_size: bool,
    pub show_permissions: bool,
    pub dirs_only: bool,
    pub follow_links: bool,
    pub sort_by: SortOrder,
    pub charset: TreeCharset,
}
```

## Error Types

Comprehensive error types for robust error handling.

```rust
use winutils_core::error::{
    PathError, IoError, WindowsError,
    ParseError, ValidationError
};
```

### `PathError`

Path-related errors.

```rust
#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("Invalid path format: {0}")]
    InvalidFormat(String),

    #[error("Path not found: {0}")]
    NotFound(PathBuf),

    #[error("Path too long: {0} (max: {1})")]
    TooLong(usize, usize),

    #[error("Invalid characters in path: {0}")]
    InvalidCharacters(String),

    #[error("Cannot normalize path: {0}")]
    NormalizationFailed(String),
}
```

## Traits and Interfaces

Core traits for extensibility.

### `Utility` Trait

Base trait for all utilities.

```rust
pub trait Utility {
    /// Get utility name
    fn name(&self) -> &str;

    /// Get utility version
    fn version(&self) -> &str;

    /// Execute utility with arguments
    fn execute(&self, args: Vec<String>) -> Result<i32>;

    /// Get help text
    fn help(&self) -> String;

    /// Validate arguments
    fn validate_args(&self, args: &[String]) -> Result<()>;
}
```

### `PathHandler` Trait

Path manipulation interface.

```rust
pub trait PathHandler {
    /// Normalize path
    fn normalize(&self, path: &str) -> Result<PathBuf>;

    /// Join paths
    fn join(&self, base: &Path, relative: &str) -> PathBuf;

    /// Make relative path
    fn relative(&self, path: &Path, base: &Path) -> Result<PathBuf>;

    /// Canonicalize path
    fn canonicalize(&self, path: &Path) -> Result<PathBuf>;
}
```

## Configuration APIs

Configuration management for utilities.

### Environment Variables

```rust
use winutils_core::env::{EnvVar, EnvManager};

pub struct EnvManager;

impl EnvManager {
    /// Get environment variable
    pub fn get(key: &str) -> Option<String>;

    /// Set environment variable
    pub fn set(key: &str, value: &str) -> Result<()>;

    /// Get all variables matching pattern
    pub fn get_matching(pattern: &str) -> Vec<(String, String)>;

    /// Expand environment variables in string
    pub fn expand(s: &str) -> String;
}
```

### Command-Line Parsing

```rust
use winutils_core::cli::{Parser, ArgSpec, ParseResult};

pub struct Parser {
    specs: Vec<ArgSpec>,
}

impl Parser {
    /// Add argument specification
    pub fn arg(mut self, spec: ArgSpec) -> Self;

    /// Parse arguments
    pub fn parse(&self, args: &[String]) -> ParseResult;

    /// Parse from environment
    pub fn parse_env(&self) -> ParseResult;
}
```

## Performance APIs

Performance monitoring and optimization.

### Benchmarking

```rust
use winutils_core::bench::{Benchmark, BenchResult};

pub struct Benchmark {
    name: String,
    iterations: usize,
}

impl Benchmark {
    /// Run benchmark
    pub fn run<F>(&self, f: F) -> BenchResult
    where
        F: Fn() -> Result<()>;

    /// Compare two functions
    pub fn compare<F1, F2>(
        &self,
        f1: F1,
        f2: F2
    ) -> (BenchResult, BenchResult)
    where
        F1: Fn() -> Result<()>,
        F2: Fn() -> Result<()>;
}
```

## Testing APIs

Testing infrastructure for utilities.

### Integration Testing

```rust
use winutils_core::test::{TestHarness, TestCase, Assertion};

pub struct TestHarness {
    cases: Vec<TestCase>,
}

impl TestHarness {
    /// Add test case
    pub fn case(mut self, case: TestCase) -> Self;

    /// Run all tests
    pub fn run(&self) -> TestResults;

    /// Run specific test
    pub fn run_case(&self, name: &str) -> TestResult;
}
```

## Windows-Specific APIs

Windows platform-specific functionality.

### Console Operations

```rust
use winutils_core::windows::console::{Console, ConsoleMode};

pub struct Console;

impl Console {
    /// Set console code page
    pub fn set_code_page(cp: u32) -> Result<()>;

    /// Get console code page
    pub fn get_code_page() -> u32;

    /// Enable virtual terminal processing
    pub fn enable_virtual_terminal() -> Result<()>;

    /// Set console mode
    pub fn set_mode(mode: ConsoleMode) -> Result<()>;

    /// Clear console
    pub fn clear() -> Result<()>;
}
```

### Process Management

```rust
use winutils_core::windows::process::{Process, ProcessInfo};

pub struct Process;

impl Process {
    /// Get current process info
    pub fn current() -> ProcessInfo;

    /// List all processes
    pub fn list() -> Result<Vec<ProcessInfo>>;

    /// Find process by name
    pub fn find_by_name(name: &str) -> Result<Vec<ProcessInfo>>;

    /// Check if elevated
    pub fn is_elevated() -> bool;
}
```

______________________________________________________________________

*API Reference Version: 1.0.0*
*Last Updated: January 2025*
*Maintained by: David Martel*
