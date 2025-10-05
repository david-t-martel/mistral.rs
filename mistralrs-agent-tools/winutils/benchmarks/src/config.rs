use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    pub winutils_path: Option<PathBuf>,
    pub native_utilities_path: Option<PathBuf>,
    pub output_path: PathBuf,
    pub utilities: Vec<UtilityConfig>,
    pub performance_thresholds: PerformanceThresholds,
    pub memory_limits: MemoryLimits,
    pub test_environment: TestEnvironment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilityConfig {
    pub name: String,
    pub description: String,
    pub test_cases: Vec<TestCase>,
    pub expected_speedup: Option<f64>, // Expected speedup vs native implementation
    pub memory_limit_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub description: String,
    pub args: Vec<String>,
    pub input_file: Option<String>, // Relative to test data directory
    pub expected_duration_ms: Option<u64>,
    pub category: String, // "file-ops", "text-processing", "system-info", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    pub regression_threshold_percent: f64,
    pub min_speedup_vs_native: f64,
    pub max_acceptable_slowdown: f64,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLimits {
    pub max_heap_mb: u64,
    pub max_stack_mb: u64,
    pub leak_threshold_kb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEnvironment {
    pub temp_dir_size_mb: u64,
    pub test_file_sizes: Vec<u64>, // File sizes in bytes
    pub directory_depth: u32,
    pub files_per_directory: u32,
}

impl BenchmarkConfig {
    pub fn load(path: &std::path::Path) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let config: BenchmarkConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::generate_default())
        }
    }

    pub fn save_to_file(&self, path: &std::path::Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn generate_default() -> Self {
        let utilities = vec![
            // File operations
            UtilityConfig {
                name: "ls".to_string(),
                description: "List directory contents".to_string(),
                test_cases: vec![
                    TestCase {
                        name: "simple_list".to_string(),
                        description: "List current directory".to_string(),
                        args: vec![],
                        input_file: None,
                        expected_duration_ms: Some(50),
                        category: "file-ops".to_string(),
                    },
                    TestCase {
                        name: "detailed_list".to_string(),
                        description: "List with detailed information".to_string(),
                        args: vec!["-la".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(100),
                        category: "file-ops".to_string(),
                    },
                    TestCase {
                        name: "recursive_list".to_string(),
                        description: "Recursive directory listing".to_string(),
                        args: vec!["-R".to_string(), "tree".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(500),
                        category: "file-ops".to_string(),
                    },
                ],
                expected_speedup: Some(4.0),
                memory_limit_mb: Some(50),
            },

            UtilityConfig {
                name: "cat".to_string(),
                description: "Display file contents".to_string(),
                test_cases: vec![
                    TestCase {
                        name: "small_file".to_string(),
                        description: "Display small file".to_string(),
                        args: vec!["small.txt".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(10),
                        category: "file-ops".to_string(),
                    },
                    TestCase {
                        name: "medium_file".to_string(),
                        description: "Display medium file".to_string(),
                        args: vec!["medium.txt".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(100),
                        category: "file-ops".to_string(),
                    },
                    TestCase {
                        name: "large_file".to_string(),
                        description: "Display large file".to_string(),
                        args: vec!["large.txt".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(2000),
                        category: "file-ops".to_string(),
                    },
                ],
                expected_speedup: Some(3.0),
                memory_limit_mb: Some(100),
            },

            UtilityConfig {
                name: "wc".to_string(),
                description: "Word, line, character, and byte count".to_string(),
                test_cases: vec![
                    TestCase {
                        name: "line_count".to_string(),
                        description: "Count lines in file".to_string(),
                        args: vec!["-l".to_string(), "unix_lines.txt".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(50),
                        category: "text-processing".to_string(),
                    },
                    TestCase {
                        name: "word_count".to_string(),
                        description: "Count words in file".to_string(),
                        args: vec!["-w".to_string(), "mixed_content.txt".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(100),
                        category: "text-processing".to_string(),
                    },
                    TestCase {
                        name: "full_count".to_string(),
                        description: "Count lines, words, and characters".to_string(),
                        args: vec!["large.txt".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(500),
                        category: "text-processing".to_string(),
                    },
                ],
                expected_speedup: Some(12.0),
                memory_limit_mb: Some(50),
            },

            UtilityConfig {
                name: "cp".to_string(),
                description: "Copy files and directories".to_string(),
                test_cases: vec![
                    TestCase {
                        name: "copy_small_file".to_string(),
                        description: "Copy small file".to_string(),
                        args: vec!["small.txt".to_string(), "small_copy.txt".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(20),
                        category: "file-ops".to_string(),
                    },
                    TestCase {
                        name: "copy_large_file".to_string(),
                        description: "Copy large file".to_string(),
                        args: vec!["large.txt".to_string(), "large_copy.txt".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(3000),
                        category: "file-ops".to_string(),
                    },
                    TestCase {
                        name: "copy_directory".to_string(),
                        description: "Copy directory recursively".to_string(),
                        args: vec!["-r".to_string(), "tree".to_string(), "tree_copy".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(1000),
                        category: "file-ops".to_string(),
                    },
                ],
                expected_speedup: Some(2.0),
                memory_limit_mb: Some(100),
            },

            UtilityConfig {
                name: "sort".to_string(),
                description: "Sort lines of text".to_string(),
                test_cases: vec![
                    TestCase {
                        name: "simple_sort".to_string(),
                        description: "Sort text file".to_string(),
                        args: vec!["mixed_content.txt".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(200),
                        category: "text-processing".to_string(),
                    },
                    TestCase {
                        name: "numeric_sort".to_string(),
                        description: "Sort numerically".to_string(),
                        args: vec!["-n".to_string(), "mixed_content.txt".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(250),
                        category: "text-processing".to_string(),
                    },
                    TestCase {
                        name: "reverse_sort".to_string(),
                        description: "Sort in reverse order".to_string(),
                        args: vec!["-r".to_string(), "mixed_content.txt".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(220),
                        category: "text-processing".to_string(),
                    },
                ],
                expected_speedup: Some(8.0),
                memory_limit_mb: Some(200),
            },

            UtilityConfig {
                name: "grep".to_string(),
                description: "Search text using patterns".to_string(),
                test_cases: vec![
                    TestCase {
                        name: "simple_search".to_string(),
                        description: "Search for pattern in file".to_string(),
                        args: vec!["Line".to_string(), "unix_lines.txt".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(100),
                        category: "text-processing".to_string(),
                    },
                    TestCase {
                        name: "regex_search".to_string(),
                        description: "Search using regular expression".to_string(),
                        args: vec!["-E".to_string(), "Line [0-9]+".to_string(), "unix_lines.txt".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(150),
                        category: "text-processing".to_string(),
                    },
                    TestCase {
                        name: "recursive_search".to_string(),
                        description: "Search recursively in directory".to_string(),
                        args: vec!["-r".to_string(), "file".to_string(), "tree".to_string()],
                        input_file: None,
                        expected_duration_ms: Some(300),
                        category: "text-processing".to_string(),
                    },
                ],
                expected_speedup: Some(6.0),
                memory_limit_mb: Some(100),
            },
        ];

        Self {
            winutils_path: Some(PathBuf::from("../target/release")),
            native_utilities_path: None, // Auto-detect
            output_path: PathBuf::from("benchmarks/reports"),
            utilities,
            performance_thresholds: PerformanceThresholds {
                regression_threshold_percent: 5.0,
                min_speedup_vs_native: 1.5,
                max_acceptable_slowdown: 0.9,
                timeout_seconds: 300,
            },
            memory_limits: MemoryLimits {
                max_heap_mb: 1024,
                max_stack_mb: 8,
                leak_threshold_kb: 100,
            },
            test_environment: TestEnvironment {
                temp_dir_size_mb: 500,
                test_file_sizes: vec![1024, 1024 * 1024, 100 * 1024 * 1024],
                directory_depth: 3,
                files_per_directory: 10,
            },
        }
    }
}
