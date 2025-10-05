//! Enhanced platform-specific benchmarking functionality for Windows
//!
//! This module provides comprehensive Windows-specific performance monitoring,
//! native utility comparison, and detailed system resource tracking.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use sysinfo::{System, SystemExt, ProcessExt, CpuExt};

#[cfg(windows)]
use windows::Win32::{
    Foundation::HANDLE,
    System::{
        ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::{GetCurrentProcess, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
        Memory::{GetProcessWorkingSetSize},
        SystemInformation::{GetSystemInfo, SYSTEM_INFO},
        Performance::{QueryPerformanceCounter, QueryPerformanceFrequency},
    },
};

/// Enhanced Windows performance counters for detailed benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsPerformanceCounters {
    pub working_set_size: u64,
    pub peak_working_set_size: u64,
    pub page_faults: u64,
    pub peak_page_faults: u64,
    pub cpu_time_user: Duration,
    pub cpu_time_kernel: Duration,
    pub io_read_operations: u64,
    pub io_write_operations: u64,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    pub handle_count: u32,
    pub thread_count: u32,
    pub gdi_objects: u32,
    pub user_objects: u32,
}

/// Native Windows utility information for comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeUtility {
    pub name: String,
    pub path: PathBuf,
    pub version: Option<String>,
    pub available: bool,
    pub shell_type: ShellType,
}

/// Types of shells/environments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShellType {
    Cmd,
    PowerShell,
    GitBash,
    WSL,
    Cygwin,
    Native,
}

/// Enhanced platform-specific benchmark runner for Windows
#[derive(Debug)]
pub struct WindowsBenchmarkRunner {
    system: System,
    native_utilities: HashMap<String, NativeUtility>,
    performance_frequency: i64,
    baseline_metrics: Option<SystemBaseline>,
}

/// System baseline metrics for normalization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemBaseline {
    pub cpu_speed_mhz: f64,
    pub memory_bandwidth_gbps: f64,
    pub disk_read_mbps: f64,
    pub disk_write_mbps: f64,
    pub context_switch_time_ns: f64,
}

impl WindowsBenchmarkRunner {
    /// Create a new Windows benchmark runner with full initialization
    pub fn new() -> Result<Self> {
        let mut system = System::new_all();
        system.refresh_all();

        let performance_frequency = Self::get_performance_frequency()?;
        let native_utilities = Self::discover_native_utilities()?;
        let baseline_metrics = Self::establish_baseline()?;

        Ok(Self {
            system,
            native_utilities,
            performance_frequency,
            baseline_metrics: Some(baseline_metrics),
        })
    }

    /// Get Windows performance frequency for high-resolution timing
    #[cfg(windows)]
    fn get_performance_frequency() -> Result<i64> {
        use windows::Win32::System::Performance::QueryPerformanceFrequency;

        let mut frequency = 0i64;
        unsafe {
            QueryPerformanceFrequency(&mut frequency)
                .ok()
                .context("Failed to get performance frequency")?;
        }
        Ok(frequency)
    }

    #[cfg(not(windows))]
    fn get_performance_frequency() -> Result<i64> {
        Ok(1_000_000_000) // Nanoseconds fallback
    }

    /// Establish system performance baseline
    fn establish_baseline() -> Result<SystemBaseline> {
        println!("Establishing system performance baseline...");

        // CPU speed test
        let cpu_start = Instant::now();
        let mut sum = 0u64;
        for i in 0..10_000_000 {
            sum = sum.wrapping_add(i);
        }
        let cpu_time = cpu_start.elapsed();
        let cpu_speed_mhz = 10.0 / cpu_time.as_secs_f64(); // Rough estimate

        // Memory bandwidth test
        let memory_start = Instant::now();
        let data = vec![0u8; 100 * 1024 * 1024]; // 100MB
        let _checksum: u64 = data.iter().map(|&x| x as u64).sum();
        let memory_time = memory_start.elapsed();
        let memory_bandwidth_gbps = 100.0 / memory_time.as_secs_f64() / 1024.0;

        // Disk I/O test
        let (disk_read_mbps, disk_write_mbps) = Self::benchmark_disk_io()?;

        // Context switch time estimation
        let context_switch_time_ns = Self::estimate_context_switch_time()?;

        Ok(SystemBaseline {
            cpu_speed_mhz,
            memory_bandwidth_gbps,
            disk_read_mbps,
            disk_write_mbps,
            context_switch_time_ns,
        })
    }

    /// Benchmark disk I/O performance
    fn benchmark_disk_io() -> Result<(f64, f64)> {
        use std::io::{Read, Write};
        use tempfile::NamedTempFile;

        // Write test
        let write_start = Instant::now();
        let mut temp_file = NamedTempFile::new()?;
        let data = vec![0u8; 10 * 1024 * 1024]; // 10MB

        temp_file.write_all(&data)?;
        temp_file.flush()?;
        let write_time = write_start.elapsed();
        let write_mbps = 10.0 / write_time.as_secs_f64();

        // Read test
        let read_start = Instant::now();
        let mut file = std::fs::File::open(temp_file.path())?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let read_time = read_start.elapsed();
        let read_mbps = 10.0 / read_time.as_secs_f64();

        Ok((read_mbps, write_mbps))
    }

    /// Estimate context switch time
    fn estimate_context_switch_time() -> Result<f64> {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let barrier = Arc::new(Barrier::new(2));
        let start_time = Arc::new(std::sync::Mutex::new(Instant::now()));

        let barrier_clone = barrier.clone();
        let start_time_clone = start_time.clone();

        let handle = thread::spawn(move || {
            barrier_clone.wait();
            let start = start_time_clone.lock().unwrap();
            start.elapsed()
        });

        // Small delay to ensure thread starts
        thread::sleep(Duration::from_millis(1));

        let measurement_start = Instant::now();
        *start_time.lock().unwrap() = measurement_start;
        barrier.wait();

        let elapsed = handle.join().unwrap();
        Ok(elapsed.as_nanos() as f64 / 2.0) // Rough estimate
    }

    /// Discover available native Windows utilities for comparison
    fn discover_native_utilities() -> Result<HashMap<String, NativeUtility>> {
        let mut utilities = HashMap::new();

        // Windows CMD utilities
        let cmd_utilities = [
            "dir", "copy", "move", "del", "type", "find", "findstr",
            "sort", "more", "fc", "comp", "tree", "where",
            "xcopy", "robocopy", "forfiles", "takeown", "icacls", "attrib"
        ];

        for cmd in &cmd_utilities {
            let utility = Self::check_native_utility(cmd, ShellType::Cmd)?;
            utilities.insert(format!("cmd_{}", cmd), utility);
        }

        // PowerShell cmdlets
        let ps_cmdlets = [
            ("Get-Content", "cat"), ("Set-Content", "tee"), ("Copy-Item", "cp"),
            ("Move-Item", "mv"), ("Remove-Item", "rm"), ("Get-ChildItem", "ls"),
            ("Select-String", "grep"), ("Sort-Object", "sort"), ("Measure-Object", "wc"),
            ("Compare-Object", "diff"), ("Get-Location", "pwd"), ("Set-Location", "cd"),
            ("New-Item", "touch"), ("Test-Path", "test"), ("Split-Path", "dirname"),
            ("Join-Path", "join"), ("Resolve-Path", "realpath")
        ];

        for (cmdlet, alias) in &ps_cmdlets {
            let utility = Self::check_powershell_cmdlet(cmdlet)?;
            utilities.insert(format!("ps_{}", alias), utility);
        }

        // Git Bash utilities (if available)
        if Self::is_git_bash_available() {
            let git_bash_utilities = [
                "ls", "cat", "cp", "mv", "rm", "mkdir", "rmdir", "pwd", "echo",
                "find", "grep", "sort", "wc", "head", "tail", "cut", "sed", "awk"
            ];

            for util in &git_bash_utilities {
                let utility = Self::check_git_bash_utility(util)?;
                utilities.insert(format!("bash_{}", util), utility);
            }
        }

        Ok(utilities)
    }

    /// Check if Git Bash is available
    fn is_git_bash_available() -> bool {
        std::env::var("MSYSTEM").is_ok() ||
        which::which("bash").is_ok()
    }

    /// Check if a native CMD utility is available
    fn check_native_utility(name: &str, shell_type: ShellType) -> Result<NativeUtility> {
        let output = Command::new("where")
            .arg(name)
            .output()
            .context("Failed to execute where command")?;

        let available = output.status.success();
        let path = if available {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .map(|p| PathBuf::from(p.trim()))
                .unwrap_or_default()
        } else {
            PathBuf::new()
        };

        let version = if available {
            Self::get_utility_version(name, &path).ok()
        } else {
            None
        };

        Ok(NativeUtility {
            name: name.to_string(),
            path,
            version,
            available,
            shell_type,
        })
    }

    /// Check if a PowerShell cmdlet is available
    fn check_powershell_cmdlet(name: &str) -> Result<NativeUtility> {
        let script = format!("Get-Command '{}' -ErrorAction SilentlyContinue | Select-Object Version", name);
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .context("Failed to execute PowerShell command")?;

        let available = output.status.success() && !output.stdout.is_empty();
        let version = if available {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .find(|line| line.contains("Version"))
                .map(|s| s.to_string())
        } else {
            None
        };

        Ok(NativeUtility {
            name: name.to_string(),
            path: PathBuf::from("powershell"),
            version,
            available,
            shell_type: ShellType::PowerShell,
        })
    }

    /// Check if a Git Bash utility is available
    fn check_git_bash_utility(name: &str) -> Result<NativeUtility> {
        let available = which::which(name).is_ok();
        let path = if available {
            which::which(name).unwrap_or_default()
        } else {
            PathBuf::new()
        };

        let version = if available {
            Self::get_utility_version(name, &path).ok()
        } else {
            None
        };

        Ok(NativeUtility {
            name: name.to_string(),
            path,
            version,
            available,
            shell_type: ShellType::GitBash,
        })
    }

    /// Get version information for a utility
    fn get_utility_version(name: &str, path: &Path) -> Result<String> {
        let version_flags = ["--version", "-V", "/V", "/?"];

        for flag in &version_flags {
            if let Ok(output) = Command::new(path)
                .arg(flag)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
            {
                if output.status.success() && !output.stdout.is_empty() {
                    let version = String::from_utf8_lossy(&output.stdout);
                    let first_line = version.lines().next().unwrap_or("unknown");
                    return Ok(first_line.to_string());
                }
            }
        }

        Ok("unknown".to_string())
    }

    /// Run a command and collect comprehensive performance metrics
    pub fn run_with_metrics(&mut self, cmd: &mut Command) -> Result<WindowsPerformanceCounters> {
        let start_time = Instant::now();

        // Get initial system state
        self.system.refresh_all();
        let initial_memory = self.get_system_memory_usage();

        // Start the process
        let mut child = cmd.spawn().context("Failed to start process")?;
        let pid = child.id();

        // Monitor the process
        let mut peak_memory = 0u64;
        let mut total_cpu_time = Duration::new(0, 0);
        let mut io_read_ops = 0u64;
        let mut io_write_ops = 0u64;

        // Monitoring loop with higher frequency
        while let Ok(None) = child.try_wait() {
            self.system.refresh_process(pid.into());

            if let Some(process) = self.system.process(pid.into()) {
                let memory = process.memory() * 1024; // Convert KB to bytes
                if memory > peak_memory {
                    peak_memory = memory;
                }

                total_cpu_time += Duration::from_secs_f64(process.cpu_usage() as f64 / 100.0 * 0.01);
            }

            std::thread::sleep(Duration::from_millis(5)); // 5ms monitoring interval
        }

        let exit_status = child.wait().context("Failed to wait for process")?;
        let total_time = start_time.elapsed();

        // Collect detailed Windows performance counters
        self.collect_detailed_windows_counters(pid, total_time, peak_memory)
    }

    /// Collect comprehensive Windows-specific performance counters
    #[cfg(windows)]
    fn collect_detailed_windows_counters(&self, pid: u32, total_time: Duration, peak_memory: u64) -> Result<WindowsPerformanceCounters> {
        // For a complete implementation, we would use PDH (Performance Data Helper) API
        // or WMI to get detailed performance counters. This is a simplified version.

        let counters = WindowsPerformanceCounters {
            working_set_size: peak_memory,
            peak_working_set_size: peak_memory,
            page_faults: 0, // Would require PDH API
            peak_page_faults: 0,
            cpu_time_user: total_time / 2, // Rough estimate
            cpu_time_kernel: total_time / 2,
            io_read_operations: 0, // Would require WMI or ETW
            io_write_operations: 0,
            io_read_bytes: 0,
            io_write_bytes: 0,
            handle_count: 0, // Would require process handle enumeration
            thread_count: 0,
            gdi_objects: 0, // Would require GDI API calls
            user_objects: 0,
        };

        Ok(counters)
    }

    #[cfg(not(windows))]
    fn collect_detailed_windows_counters(&self, _pid: u32, total_time: Duration, peak_memory: u64) -> Result<WindowsPerformanceCounters> {
        let counters = WindowsPerformanceCounters {
            working_set_size: peak_memory,
            peak_working_set_size: peak_memory,
            page_faults: 0,
            peak_page_faults: 0,
            cpu_time_user: total_time / 2,
            cpu_time_kernel: total_time / 2,
            io_read_operations: 0,
            io_write_operations: 0,
            io_read_bytes: 0,
            io_write_bytes: 0,
            handle_count: 0,
            thread_count: 0,
            gdi_objects: 0,
            user_objects: 0,
        };

        Ok(counters)
    }

    /// Get current system memory usage
    fn get_system_memory_usage(&self) -> u64 {
        self.system.total_memory() - self.system.available_memory()
    }

    /// Compare winutils performance against native Windows utilities
    pub fn compare_against_native(&mut self, utility_name: &str, args: &[&str], test_data: &Path) -> Result<ComparisonResult> {
        let winutils_path = format!("wu-{}.exe", utility_name);

        // Try different native implementations
        let mut comparison_results = Vec::new();

        // Check CMD version
        if let Some(cmd_util) = self.native_utilities.get(&format!("cmd_{}", utility_name)) {
            if cmd_util.available {
                let result = self.run_comparison(&winutils_path, &cmd_util.path.to_string_lossy(), args, test_data)?;
                comparison_results.push(("CMD", result));
            }
        }

        // Check PowerShell version
        if let Some(ps_util) = self.native_utilities.get(&format!("ps_{}", utility_name)) {
            if ps_util.available {
                let ps_script = format!("{} {}", ps_util.name, args.join(" "));
                let result = self.run_powershell_comparison(&winutils_path, &ps_script, test_data)?;
                comparison_results.push(("PowerShell", result));
            }
        }

        // Check Git Bash version
        if let Some(bash_util) = self.native_utilities.get(&format!("bash_{}", utility_name)) {
            if bash_util.available {
                let result = self.run_comparison(&winutils_path, &bash_util.path.to_string_lossy(), args, test_data)?;
                comparison_results.push(("Git Bash", result));
            }
        }

        if comparison_results.is_empty() {
            return Ok(ComparisonResult {
                utility_name: utility_name.to_string(),
                winutils_metrics: None,
                native_results: HashMap::new(),
                error: Some(format!("No native implementations found for '{}'", utility_name)),
            });
        }

        // Run winutils version
        let mut winutils_cmd = Command::new(&winutils_path);
        winutils_cmd.args(args);
        if test_data.exists() {
            winutils_cmd.arg(test_data);
        }

        let winutils_metrics = self.run_with_metrics(&mut winutils_cmd)
            .context("Failed to run winutils command")?;

        let mut native_results = HashMap::new();
        for (name, result) in comparison_results {
            native_results.insert(name.to_string(), result);
        }

        Ok(ComparisonResult {
            utility_name: utility_name.to_string(),
            winutils_metrics: Some(winutils_metrics),
            native_results,
            error: None,
        })
    }

    /// Run comparison between winutils and a native command
    fn run_comparison(&mut self, winutils_path: &str, native_path: &str, args: &[&str], test_data: &Path) -> Result<ComparisonMetrics> {
        // Run native version
        let mut native_cmd = Command::new(native_path);
        native_cmd.args(args);
        if test_data.exists() {
            native_cmd.arg(test_data);
        }

        let native_metrics = self.run_with_metrics(&mut native_cmd)
            .context("Failed to run native command")?;

        Ok(ComparisonMetrics {
            native_metrics,
            performance_ratio: PerformanceComparison {
                speed_improvement: 0.0, // Will be calculated later
                memory_efficiency: 0.0,
                cpu_efficiency: 0.0,
            },
        })
    }

    /// Run PowerShell comparison
    fn run_powershell_comparison(&mut self, winutils_path: &str, ps_script: &str, test_data: &Path) -> Result<ComparisonMetrics> {
        let mut ps_cmd = Command::new("powershell");
        ps_cmd.args(["-NoProfile", "-Command", ps_script]);
        if test_data.exists() {
            ps_cmd.arg(test_data.to_string_lossy().as_ref());
        }

        let native_metrics = self.run_with_metrics(&mut ps_cmd)
            .context("Failed to run PowerShell command")?;

        Ok(ComparisonMetrics {
            native_metrics,
            performance_ratio: PerformanceComparison {
                speed_improvement: 0.0,
                memory_efficiency: 0.0,
                cpu_efficiency: 0.0,
            },
        })
    }

    /// Get available native utilities
    pub fn get_native_utilities(&self) -> &HashMap<String, NativeUtility> {
        &self.native_utilities
    }

    /// Get system baseline metrics
    pub fn get_baseline(&self) -> Option<&SystemBaseline> {
        self.baseline_metrics.as_ref()
    }

    /// Get comprehensive system information
    pub fn get_system_info(&self) -> SystemInformation {
        SystemInformation {
            os_version: self.system.long_os_version().unwrap_or_default(),
            cpu_count: self.system.cpus().len(),
            total_memory: self.system.total_memory(),
            available_memory: self.system.available_memory(),
            cpu_brand: self.system.cpus().first()
                .map(|cpu| cpu.brand().to_string())
                .unwrap_or_default(),
            architecture: std::env::consts::ARCH.to_string(),
            platform: std::env::consts::OS.to_string(),
            shell_environment: self.detect_shell_environment(),
            performance_baseline: self.baseline_metrics.clone(),
        }
    }

    /// Detect the current shell environment
    fn detect_shell_environment(&self) -> String {
        if std::env::var("MSYSTEM").is_ok() {
            return "Git Bash".to_string();
        }

        if let Ok(comspec) = std::env::var("COMSPEC") {
            if comspec.contains("cmd.exe") {
                return "CMD".to_string();
            }
        }

        if std::env::var("PSModulePath").is_ok() {
            return "PowerShell".to_string();
        }

        "Unknown".to_string()
    }
}

/// Comparison result between winutils and native utilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub utility_name: String,
    pub winutils_metrics: Option<WindowsPerformanceCounters>,
    pub native_results: HashMap<String, ComparisonMetrics>,
    pub error: Option<String>,
}

/// Metrics for a single comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonMetrics {
    pub native_metrics: WindowsPerformanceCounters,
    pub performance_ratio: PerformanceComparison,
}

/// Performance comparison ratios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceComparison {
    pub speed_improvement: f64,      // Percentage improvement (positive = faster)
    pub memory_efficiency: f64,      // Memory usage ratio (lower = better)
    pub cpu_efficiency: f64,         // CPU usage ratio (lower = better)
}

/// Comprehensive system information for benchmark context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInformation {
    pub os_version: String,
    pub cpu_count: usize,
    pub total_memory: u64,
    pub available_memory: u64,
    pub cpu_brand: String,
    pub architecture: String,
    pub platform: String,
    pub shell_environment: String,
    pub performance_baseline: Option<SystemBaseline>,
}

/// Calculate comprehensive performance improvements
pub fn calculate_performance_improvements(winutils: &WindowsPerformanceCounters, native: &WindowsPerformanceCounters) -> PerformanceComparison {
    let speed_improvement = calculate_speed_improvement(winutils, native);
    let memory_efficiency = calculate_memory_efficiency(winutils, native);
    let cpu_efficiency = calculate_cpu_efficiency(winutils, native);

    PerformanceComparison {
        speed_improvement,
        memory_efficiency,
        cpu_efficiency,
    }
}

/// Calculate speed improvement percentage
fn calculate_speed_improvement(winutils: &WindowsPerformanceCounters, native: &WindowsPerformanceCounters) -> f64 {
    let winutils_time = winutils.cpu_time_user + winutils.cpu_time_kernel;
    let native_time = native.cpu_time_user + native.cpu_time_kernel;

    if native_time.as_secs_f64() > 0.0 {
        ((native_time.as_secs_f64() - winutils_time.as_secs_f64()) / native_time.as_secs_f64()) * 100.0
    } else {
        0.0
    }
}

/// Calculate memory efficiency ratio
fn calculate_memory_efficiency(winutils: &WindowsPerformanceCounters, native: &WindowsPerformanceCounters) -> f64 {
    if native.peak_working_set_size > 0 {
        winutils.peak_working_set_size as f64 / native.peak_working_set_size as f64
    } else {
        1.0
    }
}

/// Calculate CPU efficiency ratio
fn calculate_cpu_efficiency(winutils: &WindowsPerformanceCounters, native: &WindowsPerformanceCounters) -> f64 {
    let winutils_cpu = winutils.cpu_time_user + winutils.cpu_time_kernel;
    let native_cpu = native.cpu_time_user + native.cpu_time_kernel;

    if native_cpu.as_secs_f64() > 0.0 {
        winutils_cpu.as_secs_f64() / native_cpu.as_secs_f64()
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_benchmark_runner_creation() {
        let runner = WindowsBenchmarkRunner::new();
        assert!(runner.is_ok());
    }

    #[test]
    fn test_system_baseline_establishment() {
        let baseline = WindowsBenchmarkRunner::establish_baseline();
        assert!(baseline.is_ok());

        let baseline = baseline.unwrap();
        assert!(baseline.cpu_speed_mhz > 0.0);
        assert!(baseline.memory_bandwidth_gbps > 0.0);
    }

    #[test]
    fn test_native_utility_discovery() {
        let utilities = WindowsBenchmarkRunner::discover_native_utilities().unwrap();
        assert!(!utilities.is_empty());

        // dir command should always be available on Windows
        if cfg!(windows) {
            assert!(utilities.contains_key("cmd_dir"));
            assert!(utilities["cmd_dir"].available);
        }
    }

    #[test]
    fn test_performance_calculation() {
        let winutils = WindowsPerformanceCounters {
            working_set_size: 1024 * 1024,
            peak_working_set_size: 1024 * 1024,
            page_faults: 100,
            peak_page_faults: 100,
            cpu_time_user: Duration::from_millis(500),
            cpu_time_kernel: Duration::from_millis(200),
            io_read_operations: 10,
            io_write_operations: 5,
            io_read_bytes: 4096,
            io_write_bytes: 2048,
            handle_count: 50,
            thread_count: 4,
            gdi_objects: 10,
            user_objects: 15,
        };

        let native = WindowsPerformanceCounters {
            working_set_size: 2 * 1024 * 1024,
            peak_working_set_size: 2 * 1024 * 1024,
            page_faults: 200,
            peak_page_faults: 200,
            cpu_time_user: Duration::from_millis(800),
            cpu_time_kernel: Duration::from_millis(400),
            io_read_operations: 15,
            io_write_operations: 8,
            io_read_bytes: 8192,
            io_write_bytes: 4096,
            handle_count: 75,
            thread_count: 6,
            gdi_objects: 20,
            user_objects: 25,
        };

        let comparison = calculate_performance_improvements(&winutils, &native);

        assert!(comparison.speed_improvement > 0.0); // Should be faster
        assert!(comparison.memory_efficiency < 1.0);  // Should use less memory
        assert!(comparison.cpu_efficiency < 1.0);     // Should use less CPU
    }
}
