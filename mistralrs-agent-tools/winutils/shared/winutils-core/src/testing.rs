//! Built-in Testing Framework
//!
//! Provides self-testing capabilities, performance benchmarks, and diagnostic
//! functions that utilities can use for internal validation and troubleshooting.

use crate::{WinUtilsError, WinUtilsResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::time::{Duration, Instant};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[cfg(feature = "testing")]
use criterion::{Criterion, black_box};

#[cfg(feature = "testing")]
use tempfile::TempDir;

/// Result of a single test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub duration: Duration,
    pub message: Option<String>,
    pub error: Option<String>,
}

impl TestResult {
    /// Create a successful test result
    pub fn success<S: Into<String>>(name: S, duration: Duration) -> Self {
        Self {
            name: name.into(),
            passed: true,
            duration,
            message: None,
            error: None,
        }
    }

    /// Create a successful test result with a message
    pub fn success_with_message<S: Into<String>>(name: S, duration: Duration, message: S) -> Self {
        Self {
            name: name.into(),
            passed: true,
            duration,
            message: Some(message.into()),
            error: None,
        }
    }

    /// Create a failed test result
    pub fn failure<S: Into<String>>(name: S, duration: Duration, error: S) -> Self {
        Self {
            name: name.into(),
            passed: false,
            duration,
            message: None,
            error: Some(error.into()),
        }
    }

    /// Create a failed test result with additional message
    pub fn failure_with_message<S: Into<String>>(
        name: S,
        duration: Duration,
        message: S,
        error: S,
    ) -> Self {
        Self {
            name: name.into(),
            passed: false,
            duration,
            message: Some(message.into()),
            error: Some(error.into()),
        }
    }
}

/// Collection of test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    pub utility_name: String,
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub total_duration: Duration,
    pub results: Vec<TestResult>,
    pub summary: String,
}

impl TestResults {
    /// Create new test results collection
    pub fn new<S: Into<String>>(utility_name: S) -> Self {
        Self {
            utility_name: utility_name.into(),
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            total_duration: Duration::new(0, 0),
            results: Vec::new(),
            summary: String::new(),
        }
    }

    /// Add a test result
    pub fn add_result(&mut self, result: TestResult) {
        self.total_tests += 1;
        self.total_duration += result.duration;

        if result.passed {
            self.passed_tests += 1;
        } else {
            self.failed_tests += 1;
        }

        self.results.push(result);
        self.update_summary();
    }

    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed_tests == 0 && self.total_tests > 0
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.passed_tests as f64 / self.total_tests as f64) * 100.0
        }
    }

    fn update_summary(&mut self) {
        if self.total_tests == 0 {
            self.summary = "No tests run".to_string();
        } else if self.all_passed() {
            self.summary = format!(
                "All {} tests passed in {:.2}s",
                self.total_tests,
                self.total_duration.as_secs_f64()
            );
        } else {
            self.summary = format!(
                "{}/{} tests passed ({:.1}%) in {:.2}s",
                self.passed_tests,
                self.total_tests,
                self.success_rate(),
                self.total_duration.as_secs_f64()
            );
        }
    }

    /// Display results with color formatting
    pub fn display(&self) -> WinUtilsResult<()> {
        self.display_with_color(ColorChoice::Auto)
    }

    /// Display results with specified color choice
    pub fn display_with_color(&self, color_choice: ColorChoice) -> WinUtilsResult<()> {
        let mut stdout = StandardStream::stdout(color_choice);

        // Header
        stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Blue)))?;
        writeln!(stdout, "SELF-TEST RESULTS: {}", self.utility_name.to_uppercase())?;
        stdout.reset()?;
        writeln!(stdout)?;

        // Summary
        if self.all_passed() {
            stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Green)))?;
            writeln!(stdout, "✓ {}", self.summary)?;
        } else {
            stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Red)))?;
            writeln!(stdout, "✗ {}", self.summary)?;
        }
        stdout.reset()?;
        writeln!(stdout)?;

        // Individual results
        for result in &self.results {
            if result.passed {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                write!(stdout, "✓")?;
            } else {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                write!(stdout, "✗")?;
            }
            stdout.reset()?;

            write!(stdout, " {} ", result.name)?;

            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
            write!(stdout, "({:.3}s)", result.duration.as_secs_f64())?;
            stdout.reset()?;

            if let Some(ref message) = result.message {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
                write!(stdout, " - {}", message)?;
                stdout.reset()?;
            }

            writeln!(stdout)?;

            if let Some(ref error) = result.error {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                writeln!(stdout, "    Error: {}", error)?;
                stdout.reset()?;
            }
        }

        writeln!(stdout)?;
        Ok(())
    }
}

/// Benchmark result for a single operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u64,
    pub total_time: Duration,
    pub avg_time: Duration,
    pub min_time: Duration,
    pub max_time: Duration,
    pub operations_per_second: f64,
}

impl BenchmarkResult {
    /// Create a new benchmark result
    pub fn new<S: Into<String>>(
        name: S,
        iterations: u64,
        times: &[Duration],
    ) -> Self {
        let total_time: Duration = times.iter().sum();
        let avg_time = total_time / iterations as u32;
        let min_time = *times.iter().min().unwrap_or(&Duration::new(0, 0));
        let max_time = *times.iter().max().unwrap_or(&Duration::new(0, 0));
        let operations_per_second = if total_time.as_secs_f64() > 0.0 {
            iterations as f64 / total_time.as_secs_f64()
        } else {
            0.0
        };

        Self {
            name: name.into(),
            iterations,
            total_time,
            avg_time,
            min_time,
            max_time,
            operations_per_second,
        }
    }
}

/// Collection of benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    pub utility_name: String,
    pub results: Vec<BenchmarkResult>,
    pub total_duration: Duration,
}

impl BenchmarkResults {
    /// Create new benchmark results collection
    pub fn new<S: Into<String>>(utility_name: S) -> Self {
        Self {
            utility_name: utility_name.into(),
            results: Vec::new(),
            total_duration: Duration::new(0, 0),
        }
    }

    /// Add a benchmark result
    pub fn add_result(&mut self, result: BenchmarkResult) {
        self.total_duration += result.total_time;
        self.results.push(result);
    }

    /// Display benchmark results
    pub fn display(&self) -> WinUtilsResult<()> {
        self.display_with_color(ColorChoice::Auto)
    }

    /// Display benchmark results with specified color choice
    pub fn display_with_color(&self, color_choice: ColorChoice) -> WinUtilsResult<()> {
        let mut stdout = StandardStream::stdout(color_choice);

        // Header
        stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Blue)))?;
        writeln!(stdout, "BENCHMARK RESULTS: {}", self.utility_name.to_uppercase())?;
        stdout.reset()?;
        writeln!(stdout)?;

        if self.results.is_empty() {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
            writeln!(stdout, "No benchmarks available for this utility.")?;
            stdout.reset()?;
            return Ok(());
        }

        // Results table
        for result in &self.results {
            stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Green)))?;
            writeln!(stdout, "{}", result.name)?;
            stdout.reset()?;

            writeln!(stdout, "  Iterations: {}", result.iterations)?;
            writeln!(stdout, "  Total time: {:.3}s", result.total_time.as_secs_f64())?;
            writeln!(stdout, "  Average time: {:.6}s", result.avg_time.as_secs_f64())?;
            writeln!(stdout, "  Min time: {:.6}s", result.min_time.as_secs_f64())?;
            writeln!(stdout, "  Max time: {:.6}s", result.max_time.as_secs_f64())?;

            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
            writeln!(stdout, "  Operations/sec: {:.0}", result.operations_per_second)?;
            stdout.reset()?;
            writeln!(stdout)?;
        }

        stdout.set_color(ColorSpec::new().set_bold(true))?;
        writeln!(stdout, "Total benchmark time: {:.3}s", self.total_duration.as_secs_f64())?;
        stdout.reset()?;

        Ok(())
    }
}

/// Diagnostic mode for troubleshooting
#[derive(Debug, Clone)]
pub struct DiagnosticMode {
    pub utility_name: String,
    pub checks: Vec<DiagnosticCheck>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticCheck {
    pub name: String,
    pub description: String,
    pub check_fn: fn() -> WinUtilsResult<DiagnosticResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticResult {
    pub name: String,
    pub status: DiagnosticStatus,
    pub message: String,
    pub details: HashMap<String, String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagnosticStatus {
    Ok,
    Warning,
    Error,
    Info,
}

impl DiagnosticResult {
    /// Create an OK diagnostic result
    pub fn ok<N: Into<String>, M: Into<String>>(name: N, message: M) -> Self {
        Self {
            name: name.into(),
            status: DiagnosticStatus::Ok,
            message: message.into(),
            details: HashMap::new(),
            recommendations: Vec::new(),
        }
    }

    /// Create a warning diagnostic result
    pub fn warning<N: Into<String>, M: Into<String>>(name: N, message: M) -> Self {
        Self {
            name: name.into(),
            status: DiagnosticStatus::Warning,
            message: message.into(),
            details: HashMap::new(),
            recommendations: Vec::new(),
        }
    }

    /// Create an error diagnostic result
    pub fn error<N: Into<String>, M: Into<String>>(name: N, message: M) -> Self {
        Self {
            name: name.into(),
            status: DiagnosticStatus::Error,
            message: message.into(),
            details: HashMap::new(),
            recommendations: Vec::new(),
        }
    }

    /// Create an info diagnostic result
    pub fn info<N: Into<String>, M: Into<String>>(name: N, message: M) -> Self {
        Self {
            name: name.into(),
            status: DiagnosticStatus::Info,
            message: message.into(),
            details: HashMap::new(),
            recommendations: Vec::new(),
        }
    }

    /// Add a detail to the result
    pub fn with_detail<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    /// Add a recommendation
    pub fn with_recommendation<S: Into<String>>(mut self, recommendation: S) -> Self {
        self.recommendations.push(recommendation.into());
        self
    }
}

/// Collection of diagnostic results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticResults {
    pub utility_name: String,
    pub results: Vec<DiagnosticResult>,
    pub summary: String,
}

impl DiagnosticResults {
    /// Create new diagnostic results
    pub fn new<S: Into<String>>(utility_name: S) -> Self {
        Self {
            utility_name: utility_name.into(),
            results: Vec::new(),
            summary: String::new(),
        }
    }

    /// Add a diagnostic result
    pub fn add_result(&mut self, result: DiagnosticResult) {
        self.results.push(result);
        self.update_summary();
    }

    fn update_summary(&mut self) {
        let ok_count = self.results.iter().filter(|r| matches!(r.status, DiagnosticStatus::Ok)).count();
        let warning_count = self.results.iter().filter(|r| matches!(r.status, DiagnosticStatus::Warning)).count();
        let error_count = self.results.iter().filter(|r| matches!(r.status, DiagnosticStatus::Error)).count();
        let info_count = self.results.iter().filter(|r| matches!(r.status, DiagnosticStatus::Info)).count();

        self.summary = format!(
            "{} checks: {} OK, {} warnings, {} errors, {} info",
            self.results.len(),
            ok_count,
            warning_count,
            error_count,
            info_count
        );
    }

    /// Display diagnostic results
    pub fn display(&self) -> WinUtilsResult<()> {
        self.display_with_color(ColorChoice::Auto)
    }

    /// Display diagnostic results with specified color choice
    pub fn display_with_color(&self, color_choice: ColorChoice) -> WinUtilsResult<()> {
        let mut stdout = StandardStream::stdout(color_choice);

        // Header
        stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Blue)))?;
        writeln!(stdout, "DIAGNOSTIC RESULTS: {}", self.utility_name.to_uppercase())?;
        stdout.reset()?;
        writeln!(stdout)?;

        // Summary
        stdout.set_color(ColorSpec::new().set_bold(true))?;
        writeln!(stdout, "{}", self.summary)?;
        stdout.reset()?;
        writeln!(stdout)?;

        // Individual results
        for result in &self.results {
            let (symbol, color) = match result.status {
                DiagnosticStatus::Ok => ("✓", Color::Green),
                DiagnosticStatus::Warning => ("⚠", Color::Yellow),
                DiagnosticStatus::Error => ("✗", Color::Red),
                DiagnosticStatus::Info => ("ℹ", Color::Blue),
            };

            stdout.set_color(ColorSpec::new().set_fg(Some(color)))?;
            write!(stdout, "{}", symbol)?;
            stdout.reset()?;

            stdout.set_color(ColorSpec::new().set_bold(true))?;
            write!(stdout, " {}: ", result.name)?;
            stdout.reset()?;

            writeln!(stdout, "{}", result.message)?;

            // Details
            if !result.details.is_empty() {
                for (key, value) in &result.details {
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
                    writeln!(stdout, "    {}: {}", key, value)?;
                    stdout.reset()?;
                }
            }

            // Recommendations
            if !result.recommendations.is_empty() {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
                writeln!(stdout, "    Recommendations:")?;
                for rec in &result.recommendations {
                    writeln!(stdout, "      • {}", rec)?;
                }
                stdout.reset()?;
            }

            writeln!(stdout)?;
        }

        Ok(())
    }
}

/// Self-test trait that utilities can implement
pub trait SelfTest {
    /// Run self-tests for the utility
    fn self_test(&self) -> WinUtilsResult<TestResults>;

    /// Get the name of the utility for test reporting
    fn test_name(&self) -> &str;

    /// Run a basic functionality test
    fn test_basic_functionality(&self) -> WinUtilsResult<TestResult> {
        let start = Instant::now();

        // Default implementation - utilities should override this
        let duration = start.elapsed();
        Ok(TestResult::success_with_message(
            "Basic Functionality",
            duration,
            "Default test passed - utility should implement specific tests",
        ))
    }

    /// Test path handling capabilities
    #[cfg(feature = "testing")]
    fn test_path_handling(&self) -> WinUtilsResult<TestResult> {
        let start = Instant::now();

        // Create a temporary directory for testing
        let temp_dir = TempDir::new()
            .map_err(|e| WinUtilsError::testing(format!("Failed to create temp dir: {}", e)))?;

        let test_path = temp_dir.path().join("test_file.txt");
        std::fs::write(&test_path, "test content")
            .map_err(|e| WinUtilsError::testing(format!("Failed to write test file: {}", e)))?;

        // Test that the path exists and is readable
        if !test_path.exists() {
            let duration = start.elapsed();
            return Ok(TestResult::failure(
                "Path Handling",
                duration,
                "Test file was not created successfully",
            ));
        }

        let duration = start.elapsed();
        Ok(TestResult::success_with_message(
            "Path Handling",
            duration,
            format!("Successfully handled path: {}", test_path.display()),
        ))
    }

    #[cfg(not(feature = "testing"))]
    fn test_path_handling(&self) -> WinUtilsResult<TestResult> {
        let start = Instant::now();
        let duration = start.elapsed();
        Ok(TestResult::success_with_message(
            "Path Handling",
            duration,
            "Path handling test skipped (testing feature not enabled)",
        ))
    }
}

/// Benchmark suite trait for performance testing
pub trait BenchmarkSuite {
    /// Run benchmarks for the utility
    fn benchmark(&self) -> WinUtilsResult<BenchmarkResults>;

    /// Get the name of the utility for benchmark reporting
    fn benchmark_name(&self) -> &str;

    /// Run a simple performance benchmark
    fn benchmark_basic_operation(&self) -> WinUtilsResult<BenchmarkResult> {
        let iterations = 1000;
        let mut times = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let start = Instant::now();
            // Default implementation - utilities should override this
            black_box(42); // Prevent optimization
            times.push(start.elapsed());
        }

        Ok(BenchmarkResult::new(
            "Basic Operation",
            iterations as u64,
            &times,
        ))
    }
}

// Note: Removed orphan trait implementation for StandardStream
// The termcolor crate already provides Write trait implementation
// Using writeln! macro which works with io::Write trait

/// Common diagnostic checks that utilities can use
pub mod common_diagnostics {
    use super::*;
    use std::env;
    use std::path::Path;

    /// Check if winpath is available and functioning
    pub fn check_winpath() -> WinUtilsResult<DiagnosticResult> {
        match which::which("winpath") {
            Ok(path) => {
                let mut result = DiagnosticResult::ok(
                    "WinPath Integration",
                    "winpath.exe is available and accessible",
                )
                .with_detail("Location", path.display().to_string());

                // Try to run winpath to verify it works
                match std::process::Command::new("winpath").arg("--version").output() {
                    Ok(output) => {
                        if output.status.success() {
                            result = result.with_detail(
                                "Version",
                                String::from_utf8_lossy(&output.stdout).trim().to_string(),
                            );
                        } else {
                            result = DiagnosticResult::warning(
                                "WinPath Integration",
                                "winpath.exe found but failed to run --version",
                            )
                            .with_recommendation("Check winpath.exe installation and permissions");
                        }
                    }
                    Err(e) => {
                        result = DiagnosticResult::warning(
                            "WinPath Integration",
                            format!("winpath.exe found but failed to execute: {}", e),
                        )
                        .with_recommendation("Check winpath.exe permissions and dependencies");
                    }
                }

                Ok(result)
            }
            Err(_) => {
                Ok(DiagnosticResult::error(
                    "WinPath Integration",
                    "winpath.exe not found in PATH",
                )
                .with_recommendation("Install winpath.exe or add it to PATH")
                .with_recommendation("winpath.exe is required for proper Git Bash path handling"))
            }
        }
    }

    /// Check system PATH environment
    pub fn check_path_environment() -> WinUtilsResult<DiagnosticResult> {
        let path = env::var("PATH").unwrap_or_default();
        let path_entries: Vec<&str> = path.split(';').filter(|s| !s.is_empty()).collect();

        let mut result = DiagnosticResult::info(
            "PATH Environment",
            format!("PATH contains {} entries", path_entries.len()),
        )
        .with_detail("Total Entries", path_entries.len().to_string());

        // Check for common issues
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Check for duplicate entries
        let mut seen_paths = std::collections::HashSet::new();
        let mut duplicates = 0;
        for entry in &path_entries {
            if !seen_paths.insert(entry.to_lowercase()) {
                duplicates += 1;
            }
        }

        if duplicates > 0 {
            issues.push(format!("Found {} duplicate PATH entries", duplicates));
            recommendations.push("Remove duplicate PATH entries to improve performance".to_string());
        }

        // Check for non-existent directories
        let mut missing_dirs = 0;
        for entry in &path_entries {
            if !Path::new(entry).exists() {
                missing_dirs += 1;
            }
        }

        if missing_dirs > 0 {
            issues.push(format!("Found {} non-existent directories in PATH", missing_dirs));
            recommendations.push("Remove non-existent directories from PATH".to_string());
        }

        // Update result based on issues found
        if issues.is_empty() {
            result = result.with_detail("Status", "PATH environment looks good".to_string());
        } else {
            result = DiagnosticResult::warning(
                "PATH Environment",
                format!("PATH environment has issues: {}", issues.join(", ")),
            );

            for rec in recommendations {
                result = result.with_recommendation(rec);
            }
        }

        result = result
            .with_detail("Duplicate Entries", duplicates.to_string())
            .with_detail("Missing Directories", missing_dirs.to_string());

        Ok(result)
    }

    /// Check file system permissions
    pub fn check_file_permissions() -> WinUtilsResult<DiagnosticResult> {
        use std::fs;

        // Test read permission on current directory
        let current_dir = env::current_dir()
            .map_err(|e| WinUtilsError::diagnostics(format!("Failed to get current directory: {}", e)))?;

        let mut result = DiagnosticResult::info(
            "File Permissions",
            "Checking basic file system permissions",
        )
        .with_detail("Current Directory", current_dir.display().to_string());

        // Test read access
        match fs::read_dir(&current_dir) {
            Ok(_) => {
                result = result.with_detail("Read Access", "✓ OK".to_string());
            }
            Err(e) => {
                return Ok(DiagnosticResult::error(
                    "File Permissions",
                    format!("Cannot read current directory: {}", e),
                )
                .with_recommendation("Check directory permissions")
                .with_recommendation("Run with elevated privileges if necessary"));
            }
        }

        // Test write access (try to create a temporary file)
        let temp_file = current_dir.join(".winutils_permission_test");
        match fs::write(&temp_file, b"test") {
            Ok(_) => {
                result = result.with_detail("Write Access", "✓ OK".to_string());
                // Clean up
                let _ = fs::remove_file(&temp_file);
            }
            Err(e) => {
                result = DiagnosticResult::warning(
                    "File Permissions",
                    format!("Limited write access: {}", e),
                )
                .with_detail("Read Access", "✓ OK".to_string())
                .with_detail("Write Access", "✗ Limited".to_string())
                .with_recommendation("Some operations may require elevated privileges");
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_result_creation() {
        let result = TestResult::success("test", Duration::from_millis(100));
        assert!(result.passed);
        assert_eq!(result.name, "test");
        assert_eq!(result.duration, Duration::from_millis(100));
    }

    #[test]
    fn test_test_results_collection() {
        let mut results = TestResults::new("test-util");
        assert_eq!(results.total_tests, 0);
        assert!(results.all_passed()); // Vacuous truth - no tests means all passed

        results.add_result(TestResult::success("test1", Duration::from_millis(50)));
        assert_eq!(results.total_tests, 1);
        assert_eq!(results.passed_tests, 1);
        assert!(results.all_passed());

        results.add_result(TestResult::failure("test2", Duration::from_millis(25), "Failed"));
        assert_eq!(results.total_tests, 2);
        assert_eq!(results.passed_tests, 1);
        assert_eq!(results.failed_tests, 1);
        assert!(!results.all_passed());
        assert_eq!(results.success_rate(), 50.0);
    }

    #[test]
    fn test_benchmark_result() {
        let times = vec![
            Duration::from_millis(10),
            Duration::from_millis(15),
            Duration::from_millis(12),
        ];

        let result = BenchmarkResult::new("test_benchmark", 3, &times);
        assert_eq!(result.iterations, 3);
        assert_eq!(result.min_time, Duration::from_millis(10));
        assert_eq!(result.max_time, Duration::from_millis(15));
        assert!(result.operations_per_second > 0.0);
    }

    #[test]
    fn test_diagnostic_result() {
        let result = DiagnosticResult::ok("test", "All good")
            .with_detail("key", "value")
            .with_recommendation("Keep it up");

        assert!(matches!(result.status, DiagnosticStatus::Ok));
        assert_eq!(result.details.get("key"), Some(&"value".to_string()));
        assert_eq!(result.recommendations.len(), 1);
    }
}
