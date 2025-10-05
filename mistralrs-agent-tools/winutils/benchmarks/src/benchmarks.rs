use anyhow::{Context, Result};
use colored::*;
use criterion::{Criterion, Benchmark, BenchmarkId, Throughput};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::process::Command as AsyncCommand;

use crate::config::{BenchmarkConfig, UtilityConfig};
use crate::memory::MemoryProfiler;
use crate::metrics::{BenchmarkResults, UtilityResult, TestCase};
use crate::platforms::{Platform, get_current_platform, get_native_command};

pub struct BenchmarkSuite {
    config: BenchmarkConfig,
    memory_profiling: bool,
    native_comparison: bool,
    baseline_mode: bool,
    filter_pattern: Option<String>,
    temp_dir: TempDir,
}

impl BenchmarkSuite {
    pub fn new(config: BenchmarkConfig) -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");

        Self {
            config,
            memory_profiling: false,
            native_comparison: false,
            baseline_mode: false,
            filter_pattern: None,
            temp_dir,
        }
    }

    pub fn filter_benchmarks(&mut self, pattern: &str) {
        self.filter_pattern = Some(pattern.to_string());
    }

    pub fn set_memory_profiling(&mut self, enabled: bool) {
        self.memory_profiling = enabled;
    }

    pub fn set_native_comparison(&mut self, enabled: bool) {
        self.native_comparison = enabled;
    }

    pub fn set_baseline_mode(&mut self, enabled: bool) {
        self.baseline_mode = enabled;
    }

    pub async fn run(&self) -> Result<BenchmarkResults> {
        println!("{}", "🔍 Discovering utilities to benchmark...".bright_blue());

        let utilities = self.discover_utilities()?;

        println!("{}", format!("Found {} utilities to benchmark", utilities.len()).bright_green());

        let mut results = BenchmarkResults::new(
            chrono::Utc::now(),
            get_current_platform(),
            self.baseline_mode,
        );

        // Generate test data
        self.generate_test_data().await?;

        for utility in &utilities {
            if let Some(pattern) = &self.filter_pattern {
                if !utility.name.contains(pattern) {
                    continue;
                }
            }

            println!("{}", format!("📋 Benchmarking {}...", utility.name).bright_cyan());

            let utility_result = self.benchmark_utility(utility).await
                .with_context(|| format!("Failed to benchmark {}", utility.name))?;

            results.add_utility_result(utility_result);
        }

        Ok(results)
    }

    fn discover_utilities(&self) -> Result<Vec<UtilityConfig>> {
        let mut utilities = Vec::new();

        // Check for built winutils binaries
        let winutils_dir = self.config.winutils_path.clone().unwrap_or_else(|| {
            PathBuf::from("../target/release")
        });

        for utility_config in &self.config.utilities {
            let binary_path = winutils_dir.join(&format!("{}.exe", utility_config.name));

            if binary_path.exists() {
                utilities.push(utility_config.clone());
            } else {
                println!("{}", format!("⚠️  Binary not found for {}: {}",
                    utility_config.name, binary_path.display()).bright_yellow());
            }
        }

        Ok(utilities)
    }

    async fn generate_test_data(&self) -> Result<()> {
        println!("{}", "📁 Generating test data...".bright_blue());

        // Create test files of various sizes
        let data_dir = self.temp_dir.path().join("data");
        std::fs::create_dir_all(&data_dir)?;

        // Small file (1KB)
        self.create_test_file(&data_dir.join("small.txt"), 1024).await?;

        // Medium file (1MB)
        self.create_test_file(&data_dir.join("medium.txt"), 1024 * 1024).await?;

        // Large file (100MB)
        self.create_test_file(&data_dir.join("large.txt"), 100 * 1024 * 1024).await?;

        // Create directory structure for tree-like operations
        let tree_dir = data_dir.join("tree");
        self.create_directory_tree(&tree_dir, 3, 10).await?;

        // Create files with different line endings for text processing
        self.create_text_files(&data_dir).await?;

        println!("{}", "✅ Test data generated".bright_green());
        Ok(())
    }

    async fn create_test_file(&self, path: &Path, size: usize) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let mut file = tokio::fs::File::create(path).await?;
        let chunk = vec![b'A'; 8192]; // 8KB chunks
        let mut remaining = size;

        while remaining > 0 {
            let write_size = std::cmp::min(remaining, chunk.len());
            file.write_all(&chunk[..write_size]).await?;
            remaining -= write_size;
        }

        file.flush().await?;
        Ok(())
    }

    async fn create_directory_tree(&self, root: &Path, depth: usize, files_per_dir: usize) -> Result<()> {
        if depth == 0 {
            return Ok(());
        }

        tokio::fs::create_dir_all(root).await?;

        // Create files in current directory
        for i in 0..files_per_dir {
            let file_path = root.join(format!("file_{}.txt", i));
            self.create_test_file(&file_path, 1024).await?;
        }

        // Create subdirectories
        for i in 0..3 {
            let subdir = root.join(format!("subdir_{}", i));
            Box::pin(self.create_directory_tree(&subdir, depth - 1, files_per_dir)).await?;
        }

        Ok(())
    }

    async fn create_text_files(&self, data_dir: &Path) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        // File with Unix line endings
        let unix_file = data_dir.join("unix_lines.txt");
        let mut file = tokio::fs::File::create(&unix_file).await?;
        for i in 0..1000 {
            file.write_all(format!("Line {} with Unix ending\n", i).as_bytes()).await?;
        }

        // File with Windows line endings
        let windows_file = data_dir.join("windows_lines.txt");
        let mut file = tokio::fs::File::create(&windows_file).await?;
        for i in 0..1000 {
            file.write_all(format!("Line {} with Windows ending\r\n", i).as_bytes()).await?;
        }

        // Mixed content file for sorting
        let mixed_file = data_dir.join("mixed_content.txt");
        let mut file = tokio::fs::File::create(&mixed_file).await?;
        let content = [
            "zebra", "apple", "banana", "cat", "dog", "elephant", "fox", "giraffe",
            "house", "ice", "jungle", "kite", "lion", "mouse", "north", "ocean",
        ];
        for _ in 0..100 {
            for word in &content {
                file.write_all(format!("{}\n", word).as_bytes()).await?;
            }
        }

        Ok(())
    }

    async fn benchmark_utility(&self, utility: &UtilityConfig) -> Result<UtilityResult> {
        let mut test_cases = Vec::new();

        for test_case in &utility.test_cases {
            println!("{}", format!("  🧪 Running test case: {}", test_case.name).bright_white());

            let winutils_result = self.run_winutils_test(utility, test_case).await?;

            let native_result = if self.native_comparison {
                Some(self.run_native_test(utility, test_case).await?)
            } else {
                None
            };

            let memory_stats = if self.memory_profiling {
                Some(self.profile_memory(utility, test_case).await?)
            } else {
                None
            };

            test_cases.push(TestCase {
                name: test_case.name.clone(),
                winutils_result,
                native_result,
                memory_stats,
            });
        }

        Ok(UtilityResult {
            name: utility.name.clone(),
            test_cases,
        })
    }

    async fn run_winutils_test(&self, utility: &UtilityConfig, test_case: &crate::config::TestCase) -> Result<crate::metrics::ExecutionResult> {
        let binary_path = self.config.winutils_path.as_ref()
            .unwrap_or(&PathBuf::from("../target/release"))
            .join(&format!("{}.exe", utility.name));

        self.run_command_with_timing(&binary_path, &test_case.args, &test_case.input_file).await
    }

    async fn run_native_test(&self, utility: &UtilityConfig, test_case: &crate::config::TestCase) -> Result<crate::metrics::ExecutionResult> {
        let native_cmd = get_native_command(&utility.name)?;
        self.run_command_with_timing(&PathBuf::from(native_cmd), &test_case.args, &test_case.input_file).await
    }

    async fn run_command_with_timing(&self, binary: &Path, args: &[String], input_file: &Option<String>) -> Result<crate::metrics::ExecutionResult> {
        // Resolve input file path relative to test data directory
        let input_path = if let Some(input) = input_file {
            Some(self.temp_dir.path().join("data").join(input))
        } else {
            None
        };

        // Warm up run
        let _ = self.execute_command(binary, args, &input_path).await;

        // Timed runs
        let mut durations = Vec::new();
        let iterations = 5; // Run multiple times for statistical significance

        for _ in 0..iterations {
            let start = Instant::now();
            let output = self.execute_command(binary, args, &input_path).await?;
            let duration = start.elapsed();

            durations.push(duration);

            // Verify the command succeeded
            if !output.status.success() {
                anyhow::bail!("Command failed with exit code: {}",
                    output.status.code().unwrap_or(-1));
            }
        }

        let avg_duration = Duration::from_nanos(
            durations.iter().map(|d| d.as_nanos()).sum::<u128>() / iterations as u128
        );

        let min_duration = *durations.iter().min().unwrap();
        let max_duration = *durations.iter().max().unwrap();

        Ok(crate::metrics::ExecutionResult {
            duration: avg_duration,
            min_duration,
            max_duration,
            iterations,
            success: true,
        })
    }

    async fn execute_command(&self, binary: &Path, args: &[String], input_file: &Option<PathBuf>) -> Result<std::process::Output> {
        let mut cmd = AsyncCommand::new(binary);
        cmd.args(args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Set working directory to temp directory
        cmd.current_dir(self.temp_dir.path());

        // Handle input redirection
        if let Some(input_path) = input_file {
            let input_file = tokio::fs::File::open(input_path).await?;
            cmd.stdin(input_file.into_std().await);
        }

        let output = cmd.output().await?;
        Ok(output)
    }

    async fn profile_memory(&self, utility: &UtilityConfig, test_case: &crate::config::TestCase) -> Result<crate::memory::MemoryStats> {
        #[cfg(feature = "memory-profiling")]
        {
            let profiler = MemoryProfiler::new();
            profiler.profile_command(utility, test_case, &self.temp_dir.path()).await
        }

        #[cfg(not(feature = "memory-profiling"))]
        {
            Ok(crate::memory::MemoryStats::default())
        }
    }
}
