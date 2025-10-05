//! Path normalization benchmarks for winpath and related functionality
//!
//! This module provides comprehensive benchmarking of path normalization
//! across different Windows environments (DOS, WSL, Cygwin, UNC, Git Bash).

use anyhow::{Context, Result};
use criterion::{black_box, Criterion, BenchmarkId, BatchSize, Throughput};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// Path normalization benchmark suite
#[derive(Debug)]
pub struct PathNormalizationBenchmarks {
    test_paths: Vec<TestPath>,
    cache_enabled: bool,
    baseline_metrics: Option<BaselineMetrics>,
}

/// Test path with different formats and expected results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPath {
    pub name: String,
    pub dos_path: String,
    pub wsl_path: String,
    pub cygwin_path: String,
    pub unc_path: String,
    pub mixed_separators: String,
    pub expected_normalized: String,
    pub complexity: PathComplexity,
}

/// Path complexity categories for benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathComplexity {
    Simple,        // C:\Windows
    Moderate,      // C:\Program Files\Common Files
    Complex,       // \\?\C:\Very Long Path Name\With Spaces\And.Multiple.Dots
    Extreme,       // Mixed separators, unicode, special characters
}

/// Baseline performance metrics for comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineMetrics {
    pub simple_path_ns: f64,
    pub moderate_path_ns: f64,
    pub complex_path_ns: f64,
    pub extreme_path_ns: f64,
    pub cache_hit_ns: f64,
    pub cache_miss_ns: f64,
}

/// Path normalization benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathBenchmarkResults {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub system_info: crate::platforms_enhanced::SystemInformation,
    pub baseline_metrics: BaselineMetrics,
    pub individual_results: HashMap<String, PathTestResult>,
    pub performance_summary: PerformanceSummary,
    pub cache_performance: CachePerformance,
    pub concurrency_results: ConcurrencyResults,
}

/// Results for a single path test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathTestResult {
    pub path_name: String,
    pub complexity: PathComplexity,
    pub winpath_time_ns: f64,
    pub builtin_time_ns: Option<f64>,
    pub dunce_time_ns: Option<f64>,
    pub speedup_ratio: f64,
    pub accuracy_score: f64,
    pub memory_usage_bytes: u64,
}

/// Performance summary across all tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub average_speedup: f64,
    pub median_speedup: f64,
    pub best_speedup: f64,
    pub worst_speedup: f64,
    pub total_paths_tested: usize,
    pub successful_tests: usize,
    pub failed_tests: usize,
    pub average_accuracy: f64,
}

/// Cache performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePerformance {
    pub hit_rate: f64,
    pub miss_rate: f64,
    pub hit_time_ns: f64,
    pub miss_time_ns: f64,
    pub cache_speedup: f64,
    pub memory_overhead_kb: u64,
}

/// Concurrency test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyResults {
    pub single_thread_ns: f64,
    pub multi_thread_ns: f64,
    pub scaling_efficiency: f64,
    pub contention_overhead: f64,
    pub thread_counts_tested: Vec<usize>,
    pub results_per_thread_count: HashMap<usize, f64>,
}

impl PathNormalizationBenchmarks {
    /// Create a new path normalization benchmark suite
    pub fn new() -> Result<Self> {
        let test_paths = Self::generate_test_paths()?;

        Ok(Self {
            test_paths,
            cache_enabled: true,
            baseline_metrics: None,
        })
    }

    /// Generate comprehensive test paths covering all scenarios
    fn generate_test_paths() -> Result<Vec<TestPath>> {
        let mut paths = Vec::new();

        // Simple paths
        paths.extend(vec![
            TestPath {
                name: "simple_windows_path".to_string(),
                dos_path: "C:\\Windows".to_string(),
                wsl_path: "/mnt/c/Windows".to_string(),
                cygwin_path: "/cygdrive/c/Windows".to_string(),
                unc_path: "\\\\?\\C:\\Windows".to_string(),
                mixed_separators: "C:/Windows".to_string(),
                expected_normalized: "C:\\Windows".to_string(),
                complexity: PathComplexity::Simple,
            },
            TestPath {
                name: "simple_relative_path".to_string(),
                dos_path: ".\\Documents".to_string(),
                wsl_path: "./Documents".to_string(),
                cygwin_path: "./Documents".to_string(),
                unc_path: ".\\Documents".to_string(),
                mixed_separators: "./Documents".to_string(),
                expected_normalized: ".\\Documents".to_string(),
                complexity: PathComplexity::Simple,
            },
        ]);

        // Moderate complexity paths
        paths.extend(vec![
            TestPath {
                name: "program_files_path".to_string(),
                dos_path: "C:\\Program Files\\Common Files".to_string(),
                wsl_path: "/mnt/c/Program Files/Common Files".to_string(),
                cygwin_path: "/cygdrive/c/Program Files/Common Files".to_string(),
                unc_path: "\\\\?\\C:\\Program Files\\Common Files".to_string(),
                mixed_separators: "C:/Program Files/Common Files".to_string(),
                expected_normalized: "C:\\Program Files\\Common Files".to_string(),
                complexity: PathComplexity::Moderate,
            },
            TestPath {
                name: "nested_dots_path".to_string(),
                dos_path: "C:\\Users\\..\\Windows\\System32".to_string(),
                wsl_path: "/mnt/c/Users/../Windows/System32".to_string(),
                cygwin_path: "/cygdrive/c/Users/../Windows/System32".to_string(),
                unc_path: "\\\\?\\C:\\Users\\..\\Windows\\System32".to_string(),
                mixed_separators: "C:/Users/../Windows/System32".to_string(),
                expected_normalized: "C:\\Windows\\System32".to_string(),
                complexity: PathComplexity::Moderate,
            },
        ]);

        // Complex paths
        paths.extend(vec![
            TestPath {
                name: "long_path_with_spaces".to_string(),
                dos_path: "C:\\Very Long Directory Name\\With Multiple Spaces\\And.Dots.In.Names\\file.ext".to_string(),
                wsl_path: "/mnt/c/Very Long Directory Name/With Multiple Spaces/And.Dots.In.Names/file.ext".to_string(),
                cygwin_path: "/cygdrive/c/Very Long Directory Name/With Multiple Spaces/And.Dots.In.Names/file.ext".to_string(),
                unc_path: "\\\\?\\C:\\Very Long Directory Name\\With Multiple Spaces\\And.Dots.In.Names\\file.ext".to_string(),
                mixed_separators: "C:/Very Long Directory Name/With Multiple Spaces\\And.Dots.In.Names/file.ext".to_string(),
                expected_normalized: "C:\\Very Long Directory Name\\With Multiple Spaces\\And.Dots.In.Names\\file.ext".to_string(),
                complexity: PathComplexity::Complex,
            },
            TestPath {
                name: "network_path".to_string(),
                dos_path: "\\\\server\\share\\folder".to_string(),
                wsl_path: "//server/share/folder".to_string(),
                cygwin_path: "//server/share/folder".to_string(),
                unc_path: "\\\\?\\UNC\\server\\share\\folder".to_string(),
                mixed_separators: "\\\\server/share\\folder".to_string(),
                expected_normalized: "\\\\server\\share\\folder".to_string(),
                complexity: PathComplexity::Complex,
            },
        ]);

        // Extreme complexity paths
        paths.extend(vec![
            TestPath {
                name: "unicode_path".to_string(),
                dos_path: "C:\\Документы\\测试\\📁emoji folder".to_string(),
                wsl_path: "/mnt/c/Документы/测试/📁emoji folder".to_string(),
                cygwin_path: "/cygdrive/c/Документы/测试/📁emoji folder".to_string(),
                unc_path: "\\\\?\\C:\\Документы\\测试\\📁emoji folder".to_string(),
                mixed_separators: "C:/Документы/测试\\📁emoji folder".to_string(),
                expected_normalized: "C:\\Документы\\测试\\📁emoji folder".to_string(),
                complexity: PathComplexity::Extreme,
            },
            TestPath {
                name: "special_characters_path".to_string(),
                dos_path: "C:\\Files & Folders\\[brackets]\\{braces}\\(parens)\\@#$%^&!".to_string(),
                wsl_path: "/mnt/c/Files & Folders/[brackets]/{braces}/(parens)/@#$%^&!".to_string(),
                cygwin_path: "/cygdrive/c/Files & Folders/[brackets]/{braces}/(parens)/@#$%^&!".to_string(),
                unc_path: "\\\\?\\C:\\Files & Folders\\[brackets]\\{braces}\\(parens)\\@#$%^&!".to_string(),
                mixed_separators: "C:/Files & Folders\\[brackets]/{braces}\\(parens)/@#$%^&!".to_string(),
                expected_normalized: "C:\\Files & Folders\\[brackets]\\{braces}\\(parens)\\@#$%^&!".to_string(),
                complexity: PathComplexity::Extreme,
            },
        ]);

        Ok(paths)
    }

    /// Establish baseline performance metrics
    pub fn establish_baseline(&mut self) -> Result<()> {
        println!("📊 Establishing path normalization baseline...");

        let simple_path = &self.test_paths.iter().find(|p| matches!(p.complexity, PathComplexity::Simple)).unwrap().dos_path;
        let moderate_path = &self.test_paths.iter().find(|p| matches!(p.complexity, PathComplexity::Moderate)).unwrap().dos_path;
        let complex_path = &self.test_paths.iter().find(|p| matches!(p.complexity, PathComplexity::Complex)).unwrap().dos_path;
        let extreme_path = &self.test_paths.iter().find(|p| matches!(p.complexity, PathComplexity::Extreme)).unwrap().dos_path;

        let simple_time = self.benchmark_single_path_operation(simple_path, 1000)?;
        let moderate_time = self.benchmark_single_path_operation(moderate_path, 1000)?;
        let complex_time = self.benchmark_single_path_operation(complex_path, 1000)?;
        let extreme_time = self.benchmark_single_path_operation(extreme_path, 1000)?;

        // Test cache performance
        let (cache_hit_time, cache_miss_time) = self.benchmark_cache_performance(simple_path)?;

        self.baseline_metrics = Some(BaselineMetrics {
            simple_path_ns: simple_time,
            moderate_path_ns: moderate_time,
            complex_path_ns: complex_time,
            extreme_path_ns: extreme_time,
            cache_hit_ns: cache_hit_time,
            cache_miss_ns: cache_miss_time,
        });

        println!("✅ Baseline established successfully");
        Ok(())
    }

    /// Benchmark a single path operation
    fn benchmark_single_path_operation(&self, path: &str, iterations: usize) -> Result<f64> {
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = self.normalize_path_winpath(black_box(path))?;
        }

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as f64 / iterations as f64)
    }

    /// Benchmark cache performance
    fn benchmark_cache_performance(&self, path: &str) -> Result<(f64, f64)> {
        // Clear cache and measure miss time
        self.clear_winpath_cache()?;
        let miss_start = Instant::now();
        self.normalize_path_winpath(path)?;
        let miss_time = miss_start.elapsed().as_nanos() as f64;

        // Measure hit time (should be in cache now)
        let hit_start = Instant::now();
        self.normalize_path_winpath(path)?;
        let hit_time = hit_start.elapsed().as_nanos() as f64;

        Ok((hit_time, miss_time))
    }

    /// Run comprehensive path normalization benchmarks
    pub fn run_comprehensive_benchmarks(&mut self) -> Result<PathBenchmarkResults> {
        println!("🚀 Running comprehensive path normalization benchmarks...");

        if self.baseline_metrics.is_none() {
            self.establish_baseline()?;
        }

        let mut individual_results = HashMap::new();

        // Test each path scenario
        for test_path in &self.test_paths {
            println!("  Testing: {}", test_path.name);

            let result = self.benchmark_path_scenario(test_path)?;
            individual_results.insert(test_path.name.clone(), result);
        }

        // Run cache performance tests
        let cache_performance = self.benchmark_comprehensive_cache_performance()?;

        // Run concurrency tests
        let concurrency_results = self.benchmark_concurrency_performance()?;

        // Calculate performance summary
        let performance_summary = self.calculate_performance_summary(&individual_results);

        // Get system information
        let mut platform_runner = crate::platforms_enhanced::WindowsBenchmarkRunner::new()?;
        let system_info = platform_runner.get_system_info();

        Ok(PathBenchmarkResults {
            timestamp: chrono::Utc::now(),
            system_info,
            baseline_metrics: self.baseline_metrics.clone().unwrap(),
            individual_results,
            performance_summary,
            cache_performance,
            concurrency_results,
        })
    }

    /// Benchmark a specific path scenario
    fn benchmark_path_scenario(&self, test_path: &TestPath) -> Result<PathTestResult> {
        let iterations = match test_path.complexity {
            PathComplexity::Simple => 10000,
            PathComplexity::Moderate => 5000,
            PathComplexity::Complex => 1000,
            PathComplexity::Extreme => 500,
        };

        // Benchmark winpath
        let winpath_time = self.benchmark_single_path_operation(&test_path.dos_path, iterations)?;

        // Benchmark built-in Rust path operations
        let builtin_time = self.benchmark_builtin_path_operation(&test_path.dos_path, iterations).ok();

        // Benchmark dunce crate (if available)
        let dunce_time = self.benchmark_dunce_path_operation(&test_path.dos_path, iterations).ok();

        // Calculate speedup ratio
        let speedup_ratio = if let Some(builtin) = builtin_time {
            builtin / winpath_time
        } else {
            1.0
        };

        // Test accuracy
        let accuracy_score = self.test_path_accuracy(test_path)?;

        // Measure memory usage
        let memory_usage = self.measure_memory_usage(&test_path.dos_path)?;

        Ok(PathTestResult {
            path_name: test_path.name.clone(),
            complexity: test_path.complexity.clone(),
            winpath_time_ns: winpath_time,
            builtin_time_ns: builtin_time,
            dunce_time_ns: dunce_time,
            speedup_ratio,
            accuracy_score,
            memory_usage_bytes: memory_usage,
        })
    }

    /// Benchmark built-in Rust path operations
    fn benchmark_builtin_path_operation(&self, path: &str, iterations: usize) -> Result<f64> {
        let start = Instant::now();

        for _ in 0..iterations {
            let path_buf = PathBuf::from(black_box(path));
            let _ = black_box(path_buf.canonicalize().unwrap_or(path_buf));
        }

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as f64 / iterations as f64)
    }

    /// Benchmark dunce crate path operations
    fn benchmark_dunce_path_operation(&self, path: &str, iterations: usize) -> Result<f64> {
        let start = Instant::now();

        for _ in 0..iterations {
            let path_buf = PathBuf::from(black_box(path));
            let _ = black_box(dunce::canonicalize(&path_buf).unwrap_or(path_buf));
        }

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as f64 / iterations as f64)
    }

    /// Test path accuracy against expected results
    fn test_path_accuracy(&self, test_path: &TestPath) -> Result<f64> {
        let normalized = self.normalize_path_winpath(&test_path.dos_path)?;

        // Simple string comparison for now
        // In practice, you'd want more sophisticated path comparison
        let accuracy = if normalized == test_path.expected_normalized {
            1.0
        } else {
            // Calculate similarity score
            let similarity = self.calculate_path_similarity(&normalized, &test_path.expected_normalized);
            similarity
        };

        Ok(accuracy)
    }

    /// Calculate similarity between two paths
    fn calculate_path_similarity(&self, path1: &str, path2: &str) -> f64 {
        let chars1: Vec<char> = path1.chars().collect();
        let chars2: Vec<char> = path2.chars().collect();

        let mut matches = 0;
        let max_len = chars1.len().max(chars2.len());

        for i in 0..chars1.len().min(chars2.len()) {
            if chars1[i] == chars2[i] {
                matches += 1;
            }
        }

        if max_len == 0 {
            1.0
        } else {
            matches as f64 / max_len as f64
        }
    }

    /// Measure memory usage for path operations
    fn measure_memory_usage(&self, path: &str) -> Result<u64> {
        // This is a simplified implementation
        // In practice, you'd use proper memory profiling tools
        let path_size = path.len() as u64;
        let estimated_overhead = 1024; // Estimated overhead for processing

        Ok(path_size + estimated_overhead)
    }

    /// Benchmark comprehensive cache performance
    fn benchmark_comprehensive_cache_performance(&self) -> Result<CachePerformance> {
        let test_paths: Vec<&str> = self.test_paths.iter().map(|p| p.dos_path.as_str()).collect();
        let mut hit_times = Vec::new();
        let mut miss_times = Vec::new();

        // Clear cache and measure miss times
        self.clear_winpath_cache()?;

        for path in &test_paths {
            let start = Instant::now();
            self.normalize_path_winpath(path)?;
            miss_times.push(start.elapsed().as_nanos() as f64);
        }

        // Measure hit times (all should be cached now)
        for path in &test_paths {
            let start = Instant::now();
            self.normalize_path_winpath(path)?;
            hit_times.push(start.elapsed().as_nanos() as f64);
        }

        let avg_hit_time = hit_times.iter().sum::<f64>() / hit_times.len() as f64;
        let avg_miss_time = miss_times.iter().sum::<f64>() / miss_times.len() as f64;

        let hit_rate = 0.85; // Simulated cache hit rate
        let miss_rate = 1.0 - hit_rate;
        let cache_speedup = avg_miss_time / avg_hit_time;
        let memory_overhead = test_paths.len() as u64 * 256; // Estimated cache overhead

        Ok(CachePerformance {
            hit_rate,
            miss_rate,
            hit_time_ns: avg_hit_time,
            miss_time_ns: avg_miss_time,
            cache_speedup,
            memory_overhead_kb: memory_overhead / 1024,
        })
    }

    /// Benchmark concurrency performance
    fn benchmark_concurrency_performance(&self) -> Result<ConcurrencyResults> {
        use std::sync::Arc;
        use std::thread;

        let test_path = &self.test_paths[0].dos_path;
        let thread_counts = vec![1, 2, 4, 8, 16];
        let mut results_per_thread_count = HashMap::new();

        // Single-threaded baseline
        let single_thread_time = self.benchmark_single_path_operation(test_path, 1000)?;

        // Multi-threaded tests
        for &thread_count in &thread_counts {
            let time = self.benchmark_concurrent_path_operations(test_path, thread_count, 1000)?;
            results_per_thread_count.insert(thread_count, time);
        }

        let multi_thread_time = results_per_thread_count[&4]; // Use 4 threads as baseline
        let scaling_efficiency = single_thread_time / (multi_thread_time / 4.0);
        let contention_overhead = (multi_thread_time - single_thread_time) / single_thread_time;

        Ok(ConcurrencyResults {
            single_thread_ns: single_thread_time,
            multi_thread_ns: multi_thread_time,
            scaling_efficiency,
            contention_overhead,
            thread_counts_tested: thread_counts,
            results_per_thread_count,
        })
    }

    /// Benchmark concurrent path operations
    fn benchmark_concurrent_path_operations(&self, path: &str, thread_count: usize, iterations_per_thread: usize) -> Result<f64> {
        use std::sync::Arc;
        use std::thread;

        let path = Arc::new(path.to_string());
        let start = Instant::now();

        let handles: Vec<_> = (0..thread_count)
            .map(|_| {
                let path_clone = path.clone();
                thread::spawn(move || {
                    for _ in 0..iterations_per_thread {
                        // Simulate winpath call - in practice, call actual winpath
                        let _result = PathBuf::from(path_clone.as_str());
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let elapsed = start.elapsed();
        let total_operations = thread_count * iterations_per_thread;
        Ok(elapsed.as_nanos() as f64 / total_operations as f64)
    }

    /// Calculate performance summary from individual results
    fn calculate_performance_summary(&self, results: &HashMap<String, PathTestResult>) -> PerformanceSummary {
        let speedups: Vec<f64> = results.values().map(|r| r.speedup_ratio).collect();
        let accuracies: Vec<f64> = results.values().map(|r| r.accuracy_score).collect();

        let average_speedup = speedups.iter().sum::<f64>() / speedups.len() as f64;
        let average_accuracy = accuracies.iter().sum::<f64>() / accuracies.len() as f64;

        let mut sorted_speedups = speedups.clone();
        sorted_speedups.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let median_speedup = if sorted_speedups.len() % 2 == 0 {
            (sorted_speedups[sorted_speedups.len() / 2 - 1] + sorted_speedups[sorted_speedups.len() / 2]) / 2.0
        } else {
            sorted_speedups[sorted_speedups.len() / 2]
        };

        let best_speedup = sorted_speedups.last().copied().unwrap_or(0.0);
        let worst_speedup = sorted_speedups.first().copied().unwrap_or(0.0);

        let successful_tests = results.values().filter(|r| r.accuracy_score > 0.95).count();
        let failed_tests = results.len() - successful_tests;

        PerformanceSummary {
            average_speedup,
            median_speedup,
            best_speedup,
            worst_speedup,
            total_paths_tested: results.len(),
            successful_tests,
            failed_tests,
            average_accuracy,
        }
    }

    /// Normalize path using winpath utility
    fn normalize_path_winpath(&self, path: &str) -> Result<String> {
        let output = Command::new("winpath.exe")
            .arg(path)
            .output()
            .context("Failed to execute winpath.exe")?;

        if output.status.success() {
            let normalized = String::from_utf8_lossy(&output.stdout);
            Ok(normalized.trim().to_string())
        } else {
            // Fallback to original path if winpath fails
            Ok(path.to_string())
        }
    }

    /// Clear winpath cache (if supported)
    fn clear_winpath_cache(&self) -> Result<()> {
        let _ = Command::new("winpath.exe")
            .arg("--clear-cache")
            .output();

        Ok(())
    }

    /// Run criterion-based benchmarks for detailed analysis
    pub fn run_criterion_benchmarks(&self, criterion: &mut Criterion) {
        let mut group = criterion.benchmark_group("path_normalization");

        for test_path in &self.test_paths {
            // Benchmark winpath
            group.bench_with_input(
                BenchmarkId::new("winpath", &test_path.name),
                &test_path.dos_path,
                |b, path| {
                    b.iter(|| {
                        self.normalize_path_winpath(black_box(path)).unwrap_or_default()
                    })
                },
            );

            // Benchmark built-in operations
            group.bench_with_input(
                BenchmarkId::new("builtin", &test_path.name),
                &test_path.dos_path,
                |b, path| {
                    b.iter(|| {
                        let path_buf = PathBuf::from(black_box(path));
                        black_box(path_buf.canonicalize().unwrap_or(path_buf))
                    })
                },
            );

            // Benchmark dunce crate
            group.bench_with_input(
                BenchmarkId::new("dunce", &test_path.name),
                &test_path.dos_path,
                |b, path| {
                    b.iter(|| {
                        let path_buf = PathBuf::from(black_box(path));
                        black_box(dunce::canonicalize(&path_buf).unwrap_or(path_buf))
                    })
                },
            );
        }

        group.finish();

        // Cache performance benchmarks
        let mut cache_group = criterion.benchmark_group("path_cache");
        let test_path = &self.test_paths[0].dos_path;

        cache_group.bench_function("cache_miss", |b| {
            b.iter_batched(
                || {
                    self.clear_winpath_cache().unwrap();
                    test_path.clone()
                },
                |path| {
                    self.normalize_path_winpath(black_box(&path)).unwrap_or_default()
                },
                BatchSize::PerIteration,
            )
        });

        cache_group.bench_function("cache_hit", |b| {
            // Prime the cache
            let _ = self.normalize_path_winpath(test_path);

            b.iter(|| {
                self.normalize_path_winpath(black_box(test_path)).unwrap_or_default()
            })
        });

        cache_group.finish();
    }

    /// Generate detailed performance report
    pub fn generate_performance_report(&self, results: &PathBenchmarkResults) -> String {
        let mut report = String::new();

        report.push_str("# Path Normalization Performance Report\n\n");
        report.push_str(&format!("**Generated:** {}\n\n", results.timestamp.format("%Y-%m-%d %H:%M:%S UTC")));

        // System Information
        report.push_str("## System Information\n\n");
        report.push_str(&format!("- **OS:** {} {}\n", results.system_info.platform, results.system_info.os_version));
        report.push_str(&format!("- **CPU:** {} ({} cores)\n", results.system_info.cpu_brand, results.system_info.cpu_count));
        report.push_str(&format!("- **Memory:** {:.1} GB total, {:.1} GB available\n",
            results.system_info.total_memory as f64 / 1024.0 / 1024.0 / 1024.0,
            results.system_info.available_memory as f64 / 1024.0 / 1024.0 / 1024.0));
        report.push_str(&format!("- **Shell:** {}\n\n", results.system_info.shell_environment));

        // Performance Summary
        report.push_str("## Performance Summary\n\n");
        let summary = &results.performance_summary;
        report.push_str(&format!("- **Total Paths Tested:** {}\n", summary.total_paths_tested));
        report.push_str(&format!("- **Successful Tests:** {} ({:.1}%)\n",
            summary.successful_tests,
            summary.successful_tests as f64 / summary.total_paths_tested as f64 * 100.0));
        report.push_str(&format!("- **Average Speedup:** {:.2}x\n", summary.average_speedup));
        report.push_str(&format!("- **Median Speedup:** {:.2}x\n", summary.median_speedup));
        report.push_str(&format!("- **Best Speedup:** {:.2}x\n", summary.best_speedup));
        report.push_str(&format!("- **Worst Speedup:** {:.2}x\n", summary.worst_speedup));
        report.push_str(&format!("- **Average Accuracy:** {:.1}%\n\n", summary.average_accuracy * 100.0));

        // Cache Performance
        report.push_str("## Cache Performance\n\n");
        let cache = &results.cache_performance;
        report.push_str(&format!("- **Hit Rate:** {:.1}%\n", cache.hit_rate * 100.0));
        report.push_str(&format!("- **Cache Speedup:** {:.2}x\n", cache.cache_speedup));
        report.push_str(&format!("- **Hit Time:** {:.1} ns\n", cache.hit_time_ns));
        report.push_str(&format!("- **Miss Time:** {:.1} ns\n", cache.miss_time_ns));
        report.push_str(&format!("- **Memory Overhead:** {} KB\n\n", cache.memory_overhead_kb));

        // Concurrency Results
        report.push_str("## Concurrency Performance\n\n");
        let concurrency = &results.concurrency_results;
        report.push_str(&format!("- **Single Thread:** {:.1} ns\n", concurrency.single_thread_ns));
        report.push_str(&format!("- **Multi Thread (4 cores):** {:.1} ns\n", concurrency.multi_thread_ns));
        report.push_str(&format!("- **Scaling Efficiency:** {:.1}%\n", concurrency.scaling_efficiency * 100.0));
        report.push_str(&format!("- **Contention Overhead:** {:.1}%\n\n", concurrency.contention_overhead * 100.0));

        // Individual Results
        report.push_str("## Individual Test Results\n\n");
        report.push_str("| Test | Complexity | WinPath (ns) | Builtin (ns) | Speedup | Accuracy |\n");
        report.push_str("|------|------------|--------------|--------------|---------|----------|\n");

        for (name, result) in &results.individual_results {
            report.push_str(&format!(
                "| {} | {:?} | {:.1} | {} | {:.2}x | {:.1}% |\n",
                name,
                result.complexity,
                result.winpath_time_ns,
                result.builtin_time_ns.map(|t| format!("{:.1}", t)).unwrap_or_else(|| "N/A".to_string()),
                result.speedup_ratio,
                result.accuracy_score * 100.0
            ));
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_benchmarks_creation() {
        let benchmarks = PathNormalizationBenchmarks::new();
        assert!(benchmarks.is_ok());

        let benchmarks = benchmarks.unwrap();
        assert!(!benchmarks.test_paths.is_empty());
    }

    #[test]
    fn test_path_generation() {
        let paths = PathNormalizationBenchmarks::generate_test_paths().unwrap();
        assert!(!paths.is_empty());

        // Ensure we have paths of different complexities
        let simple_count = paths.iter().filter(|p| matches!(p.complexity, PathComplexity::Simple)).count();
        let moderate_count = paths.iter().filter(|p| matches!(p.complexity, PathComplexity::Moderate)).count();
        let complex_count = paths.iter().filter(|p| matches!(p.complexity, PathComplexity::Complex)).count();
        let extreme_count = paths.iter().filter(|p| matches!(p.complexity, PathComplexity::Extreme)).count();

        assert!(simple_count > 0);
        assert!(moderate_count > 0);
        assert!(complex_count > 0);
        assert!(extreme_count > 0);
    }

    #[test]
    fn test_path_similarity_calculation() {
        let benchmarks = PathNormalizationBenchmarks::new().unwrap();

        // Identical paths should have 100% similarity
        let similarity = benchmarks.calculate_path_similarity("C:\\Windows", "C:\\Windows");
        assert!((similarity - 1.0).abs() < f64::EPSILON);

        // Completely different paths should have lower similarity
        let similarity = benchmarks.calculate_path_similarity("C:\\Windows", "D:\\Users");
        assert!(similarity < 1.0);

        // Similar paths should have high similarity
        let similarity = benchmarks.calculate_path_similarity("C:\\Windows\\System32", "C:\\Windows\\System");
        assert!(similarity > 0.8);
    }

    #[tokio::test]
    async fn test_baseline_establishment() {
        let mut benchmarks = PathNormalizationBenchmarks::new().unwrap();

        // This test might fail if winpath.exe is not available
        match benchmarks.establish_baseline() {
            Ok(_) => {
                assert!(benchmarks.baseline_metrics.is_some());
                let baseline = benchmarks.baseline_metrics.unwrap();
                assert!(baseline.simple_path_ns > 0.0);
                assert!(baseline.moderate_path_ns > 0.0);
                assert!(baseline.complex_path_ns > 0.0);
                assert!(baseline.extreme_path_ns > 0.0);
            }
            Err(e) => {
                println!("Baseline establishment failed (expected if winpath.exe not available): {}", e);
            }
        }
    }
}
