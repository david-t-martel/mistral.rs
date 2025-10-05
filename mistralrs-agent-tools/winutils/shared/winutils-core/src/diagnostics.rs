//! System Diagnostics and Performance Monitoring
//!
//! Provides comprehensive system diagnostics, performance monitoring,
//! and troubleshooting capabilities for Windows utilities.

use crate::{WinUtilsError, WinUtilsResult};
use crate::testing::{DiagnosticResult, DiagnosticStatus};

// Re-export DiagnosticResults for public API
pub use crate::testing::DiagnosticResults;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[cfg(feature = "diagnostics")]
use sysinfo::System;

/// System information and diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub architecture: String,
    pub cpu_count: usize,
    pub total_memory: u64,
    pub available_memory: u64,
    pub boot_time: SystemTime,
    pub uptime: Duration,
}

impl SystemInfo {
    /// Collect system information
    #[cfg(feature = "diagnostics")]
    pub fn collect() -> WinUtilsResult<Self> {
        use sysinfo::{System, SystemExt};

        let mut system = System::new_all();
        system.refresh_all();

        let boot_time_secs = System::boot_time();
        let boot_time = SystemTime::UNIX_EPOCH + Duration::from_secs(boot_time_secs);
        let uptime = SystemTime::now()
            .duration_since(boot_time)
            .unwrap_or(Duration::new(0, 0));

        Ok(Self {
            hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
            os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
            os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
            kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
            architecture: std::env::consts::ARCH.to_string(),
            cpu_count: SystemExt::cpus(&system).len(),
            total_memory: SystemExt::total_memory(&system),
            available_memory: SystemExt::available_memory(&system),
            boot_time,
            uptime,
        })
    }

    #[cfg(not(feature = "diagnostics"))]
    pub fn collect() -> WinUtilsResult<Self> {
        use std::env;

        Ok(Self {
            hostname: env::var("COMPUTERNAME")
                .or_else(|_| env::var("HOSTNAME"))
                .unwrap_or_else(|_| "Unknown".to_string()),
            os_name: env::consts::OS.to_string(),
            os_version: "Unknown".to_string(),
            kernel_version: "Unknown".to_string(),
            architecture: env::consts::ARCH.to_string(),
            cpu_count: std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(1),
            total_memory: 0,
            available_memory: 0,
            boot_time: SystemTime::UNIX_EPOCH,
            uptime: Duration::new(0, 0),
        })
    }

    /// Get memory usage percentage
    pub fn memory_usage_percent(&self) -> f64 {
        if self.total_memory == 0 {
            0.0
        } else {
            ((self.total_memory - self.available_memory) as f64 / self.total_memory as f64) * 100.0
        }
    }

    /// Format memory size in human-readable format
    pub fn format_memory(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }
}

/// Performance metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub timestamp: SystemTime,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub disk_usage: HashMap<String, DiskMetrics>,
    pub network_usage: NetworkMetrics,
    pub process_count: usize,
    pub load_average: Option<[f64; 3]>, // 1min, 5min, 15min (Unix-like systems)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMetrics {
    pub total_space: u64,
    pub available_space: u64,
    pub used_space: u64,
    pub usage_percent: f64,
    pub file_system: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub errors_sent: u64,
    pub errors_received: u64,
}

impl PerformanceMetrics {
    /// Collect current performance metrics
    #[cfg(feature = "diagnostics")]
    pub fn collect() -> WinUtilsResult<Self> {
        use sysinfo::{System, SystemExt, CpuExt, DiskExt, NetworkExt};

        let mut system = System::new_all();
        system.refresh_all();

        // CPU usage (average across all cores)
        let cpus = SystemExt::cpus(&system);
        let cpu_usage = cpus.iter()
            .map(|cpu| CpuExt::cpu_usage(cpu))
            .sum::<f32>() / cpus.len() as f32;

        // Memory usage
        let memory_usage = SystemExt::total_memory(&system) - SystemExt::available_memory(&system);

        // Disk usage
        let mut disk_usage = HashMap::new();
        for disk in SystemExt::disks(&system) {
            let total = DiskExt::total_space(disk);
            let available = DiskExt::available_space(disk);
            let used = total - available;
            let usage_percent = if total > 0 {
                (used as f64 / total as f64) * 100.0
            } else {
                0.0
            };

            disk_usage.insert(
                DiskExt::name(disk).to_string_lossy().to_string(),
                DiskMetrics {
                    total_space: total,
                    available_space: available,
                    used_space: used,
                    usage_percent,
                    file_system: String::from_utf8_lossy(DiskExt::file_system(disk)).to_string(),
                },
            );
        }

        // Network usage (aggregate all interfaces)
        let mut network_usage = NetworkMetrics {
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
            errors_sent: 0,
            errors_received: 0,
        };

        for (_, data) in SystemExt::networks(&system) {
            network_usage.bytes_sent += NetworkExt::transmitted(data);
            network_usage.bytes_received += NetworkExt::received(data);
            network_usage.packets_sent += NetworkExt::packets_transmitted(data);
            network_usage.packets_received += NetworkExt::packets_received(data);
            network_usage.errors_sent += NetworkExt::errors_on_transmitted(data);
            network_usage.errors_received += NetworkExt::errors_on_received(data);
        }

        // Process count
        let process_count = SystemExt::processes(&system).len();

        // Load average (Unix-like systems only)
        let load_average = SystemExt::load_average(&system);
        let load_avg = if load_average.one >= 0.0 {
            Some([load_average.one, load_average.five, load_average.fifteen])
        } else {
            None
        };

        Ok(Self {
            timestamp: SystemTime::now(),
            cpu_usage,
            memory_usage,
            disk_usage,
            network_usage,
            process_count,
            load_average: load_avg,
        })
    }

    #[cfg(not(feature = "diagnostics"))]
    pub fn collect() -> WinUtilsResult<Self> {
        Ok(Self {
            timestamp: SystemTime::now(),
            cpu_usage: 0.0,
            memory_usage: 0,
            disk_usage: HashMap::new(),
            network_usage: NetworkMetrics {
                bytes_sent: 0,
                bytes_received: 0,
                packets_sent: 0,
                packets_received: 0,
                errors_sent: 0,
                errors_received: 0,
            },
            process_count: 0,
            load_average: None,
        })
    }
}

/// Performance monitor for tracking metrics over time
pub struct PerformanceMonitor {
    pub utility_name: String,
    metrics_history: Vec<PerformanceMetrics>,
    start_time: Instant,
    max_history: usize,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new<S: Into<String>>(utility_name: S) -> Self {
        Self {
            utility_name: utility_name.into(),
            metrics_history: Vec::new(),
            start_time: Instant::now(),
            max_history: 100, // Keep last 100 measurements
        }
    }

    /// Start monitoring (collect initial metrics)
    pub fn start(&mut self) -> WinUtilsResult<()> {
        self.start_time = Instant::now();
        self.collect_metrics()
    }

    /// Collect current metrics and add to history
    pub fn collect_metrics(&mut self) -> WinUtilsResult<()> {
        let metrics = PerformanceMetrics::collect()?;

        self.metrics_history.push(metrics);

        // Keep only the last max_history measurements
        if self.metrics_history.len() > self.max_history {
            self.metrics_history.remove(0);
        }

        Ok(())
    }

    /// Get the latest metrics
    pub fn latest_metrics(&self) -> Option<&PerformanceMetrics> {
        self.metrics_history.last()
    }

    /// Get average CPU usage over the monitoring period
    pub fn average_cpu_usage(&self) -> f32 {
        if self.metrics_history.is_empty() {
            return 0.0;
        }

        let sum: f32 = self.metrics_history.iter()
            .map(|m| m.cpu_usage)
            .sum();

        sum / self.metrics_history.len() as f32
    }

    /// Get peak memory usage
    pub fn peak_memory_usage(&self) -> u64 {
        self.metrics_history.iter()
            .map(|m| m.memory_usage)
            .max()
            .unwrap_or(0)
    }

    /// Get monitoring duration
    pub fn duration(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Generate a performance report
    pub fn generate_report(&self) -> PerformanceReport {
        let duration = self.duration();
        let latest = self.latest_metrics();

        PerformanceReport {
            utility_name: self.utility_name.clone(),
            monitoring_duration: duration,
            measurements_count: self.metrics_history.len(),
            average_cpu_usage: self.average_cpu_usage(),
            peak_memory_usage: self.peak_memory_usage(),
            current_metrics: latest.cloned(),
            summary: self.generate_summary(),
        }
    }

    fn generate_summary(&self) -> String {
        if self.metrics_history.is_empty() {
            return "No performance data collected".to_string();
        }

        let avg_cpu = self.average_cpu_usage();
        let peak_mem = self.peak_memory_usage();
        let duration = self.duration();

        format!(
            "Monitored for {:.1}s: Avg CPU {:.1}%, Peak Memory {}",
            duration.as_secs_f64(),
            avg_cpu,
            SystemInfo::format_memory(peak_mem)
        )
    }
}

/// Performance monitoring report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub utility_name: String,
    pub monitoring_duration: Duration,
    pub measurements_count: usize,
    pub average_cpu_usage: f32,
    pub peak_memory_usage: u64,
    pub current_metrics: Option<PerformanceMetrics>,
    pub summary: String,
}

impl PerformanceReport {
    /// Display the performance report
    pub fn display(&self) -> WinUtilsResult<()> {
        self.display_with_color(ColorChoice::Auto)
    }

    /// Display the performance report with specified color choice
    pub fn display_with_color(&self, color_choice: ColorChoice) -> WinUtilsResult<()> {
        let mut stdout = StandardStream::stdout(color_choice);

        // Header
        stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Blue)))?;
        writeln!(stdout, "PERFORMANCE REPORT: {}", self.utility_name.to_uppercase())?;
        stdout.reset()?;
        writeln!(stdout)?;

        // Summary
        stdout.set_color(ColorSpec::new().set_bold(true))?;
        writeln!(stdout, "{}", self.summary)?;
        stdout.reset()?;
        writeln!(stdout)?;

        // Detailed metrics
        writeln!(stdout, "Monitoring Duration: {:.2}s", self.monitoring_duration.as_secs_f64())?;
        writeln!(stdout, "Measurements Taken: {}", self.measurements_count)?;
        writeln!(stdout, "Average CPU Usage: {:.1}%", self.average_cpu_usage)?;
        writeln!(stdout, "Peak Memory Usage: {}", SystemInfo::format_memory(self.peak_memory_usage))?;
        writeln!(stdout)?;

        // Current metrics (if available)
        if let Some(ref metrics) = self.current_metrics {
            stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Green)))?;
            writeln!(stdout, "CURRENT SYSTEM STATUS")?;
            stdout.reset()?;

            writeln!(stdout, "CPU Usage: {:.1}%", metrics.cpu_usage)?;
            writeln!(stdout, "Memory Usage: {}", SystemInfo::format_memory(metrics.memory_usage))?;
            writeln!(stdout, "Process Count: {}", metrics.process_count)?;

            if let Some(load_avg) = metrics.load_average {
                writeln!(stdout, "Load Average: {:.2}, {:.2}, {:.2}",
                    load_avg[0], load_avg[1], load_avg[2])?;
            }

            // Disk usage
            if !metrics.disk_usage.is_empty() {
                writeln!(stdout)?;
                stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Cyan)))?;
                writeln!(stdout, "DISK USAGE")?;
                stdout.reset()?;

                for (name, disk) in &metrics.disk_usage {
                    writeln!(stdout, "  {}: {:.1}% ({} / {})",
                        name,
                        disk.usage_percent,
                        SystemInfo::format_memory(disk.used_space),
                        SystemInfo::format_memory(disk.total_space)
                    )?;
                }
            }

            // Network usage
            writeln!(stdout)?;
            stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Magenta)))?;
            writeln!(stdout, "NETWORK USAGE")?;
            stdout.reset()?;
            writeln!(stdout, "  Sent: {} ({} packets)",
                SystemInfo::format_memory(metrics.network_usage.bytes_sent),
                metrics.network_usage.packets_sent
            )?;
            writeln!(stdout, "  Received: {} ({} packets)",
                SystemInfo::format_memory(metrics.network_usage.bytes_received),
                metrics.network_usage.packets_received
            )?;

            if metrics.network_usage.errors_sent > 0 || metrics.network_usage.errors_received > 0 {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
                writeln!(stdout, "  Errors: {} sent, {} received",
                    metrics.network_usage.errors_sent,
                    metrics.network_usage.errors_received
                )?;
                stdout.reset()?;
            }
        }

        writeln!(stdout)?;
        Ok(())
    }
}

/// System diagnostics coordinator
pub struct SystemDiagnostics {
    pub utility_name: String,
    system_info: Option<SystemInfo>,
    performance_monitor: Option<PerformanceMonitor>,
}

impl SystemDiagnostics {
    /// Create a new system diagnostics instance
    pub fn new<S: Into<String>>(utility_name: S) -> Self {
        Self {
            utility_name: utility_name.into(),
            system_info: None,
            performance_monitor: None,
        }
    }

    /// Initialize diagnostics (collect system info)
    pub fn initialize(&mut self) -> WinUtilsResult<()> {
        self.system_info = Some(SystemInfo::collect()?);
        self.performance_monitor = Some(PerformanceMonitor::new(&self.utility_name));
        Ok(())
    }

    /// Start performance monitoring
    pub fn start_monitoring(&mut self) -> WinUtilsResult<()> {
        if let Some(ref mut monitor) = self.performance_monitor {
            monitor.start()?;
        }
        Ok(())
    }

    /// Collect current performance metrics
    pub fn collect_metrics(&mut self) -> WinUtilsResult<()> {
        if let Some(ref mut monitor) = self.performance_monitor {
            monitor.collect_metrics()?;
        }
        Ok(())
    }

    /// Run comprehensive system diagnostics
    pub fn run_diagnostics(&mut self) -> WinUtilsResult<DiagnosticResults> {
        use crate::testing::{DiagnosticResult, DiagnosticStatus};

        let mut results = DiagnosticResults::new(&self.utility_name);

        // System information check
        if let Some(ref info) = self.system_info {
            let result = DiagnosticResult::info(
                "System Information",
                format!("{} {} on {}", info.os_name, info.os_version, info.architecture),
            )
            .with_detail("Hostname", info.hostname.clone())
            .with_detail("CPU Count", info.cpu_count.to_string())
            .with_detail("Total Memory", SystemInfo::format_memory(info.total_memory))
            .with_detail("Uptime", format!("{:.1} hours", info.uptime.as_secs_f64() / 3600.0));

            results.add_result(result);
        }

        // Memory usage check
        if let Some(ref info) = self.system_info {
            let usage_percent = info.memory_usage_percent();
            let result = if usage_percent > 90.0 {
                DiagnosticResult::error(
                    "Memory Usage",
                    format!("High memory usage: {:.1}%", usage_percent),
                )
                .with_recommendation("Close unnecessary applications")
                .with_recommendation("Consider adding more RAM")
            } else if usage_percent > 70.0 {
                DiagnosticResult::warning(
                    "Memory Usage",
                    format!("Moderate memory usage: {:.1}%", usage_percent),
                )
                .with_recommendation("Monitor memory usage during intensive operations")
            } else {
                DiagnosticResult::ok(
                    "Memory Usage",
                    format!("Memory usage is normal: {:.1}%", usage_percent),
                )
            };

            results.add_result(
                result.with_detail("Available Memory", SystemInfo::format_memory(info.available_memory))
            );
        }

        // Performance monitoring check
        if let Some(ref monitor) = self.performance_monitor {
            if let Some(metrics) = monitor.latest_metrics() {
                let cpu_usage = metrics.cpu_usage;
                let result = if cpu_usage > 90.0 {
                    DiagnosticResult::warning(
                        "CPU Usage".to_string(),
                        format!("High CPU usage: {:.1}%", cpu_usage),
                    )
                    .with_recommendation("Check for resource-intensive processes")
                } else {
                    DiagnosticResult::ok(
                        "CPU Usage".to_string(),
                        format!("CPU usage is normal: {:.1}%", cpu_usage),
                    )
                };

                results.add_result(
                    result
                        .with_detail("Process Count", metrics.process_count.to_string())
                        .with_detail("Monitoring Duration", format!("{:.1}s", monitor.duration().as_secs_f64()))
                );
            }
        }

        // Disk space checks
        if let Some(ref monitor) = self.performance_monitor {
            if let Some(metrics) = monitor.latest_metrics() {
                for (name, disk) in &metrics.disk_usage {
                    let result = if disk.usage_percent > 95.0 {
                        DiagnosticResult::error(
                            format!("Disk Space ({})", name),
                            format!("Critically low disk space: {:.1}%", disk.usage_percent),
                        )
                        .with_recommendation("Free up disk space immediately")
                        .with_recommendation("Move large files to another drive")
                    } else if disk.usage_percent > 80.0 {
                        DiagnosticResult::warning(
                            format!("Disk Space ({})", name),
                            format!("Low disk space: {:.1}%", disk.usage_percent),
                        )
                        .with_recommendation("Consider cleaning up temporary files")
                    } else {
                        DiagnosticResult::ok(
                            format!("Disk Space ({})", name),
                            format!("Disk space is adequate: {:.1}%", disk.usage_percent),
                        )
                    };

                    results.add_result(
                        result
                            .with_detail("Total Space", SystemInfo::format_memory(disk.total_space))
                            .with_detail("Available Space", SystemInfo::format_memory(disk.available_space))
                            .with_detail("File System", disk.file_system.clone())
                    );
                }
            }
        }

        // Run common diagnostics
        results.add_result(crate::testing::common_diagnostics::check_winpath()?);
        results.add_result(crate::testing::common_diagnostics::check_path_environment()?);
        results.add_result(crate::testing::common_diagnostics::check_file_permissions()?);

        Ok(results)
    }

    /// Generate performance report
    pub fn generate_performance_report(&self) -> Option<PerformanceReport> {
        self.performance_monitor.as_ref().map(|m| m.generate_report())
    }

    /// Get system information
    pub fn system_info(&self) -> Option<&SystemInfo> {
        self.system_info.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_info_collection() {
        let info = SystemInfo::collect().unwrap();
        assert!(!info.hostname.is_empty());
        assert!(!info.os_name.is_empty());
        assert!(!info.architecture.is_empty());
        assert!(info.cpu_count > 0);
    }

    #[test]
    fn test_memory_formatting() {
        assert_eq!(SystemInfo::format_memory(512), "512 B");
        assert_eq!(SystemInfo::format_memory(1024), "1.0 KB");
        assert_eq!(SystemInfo::format_memory(1536), "1.5 KB");
        assert_eq!(SystemInfo::format_memory(1048576), "1.0 MB");
        assert_eq!(SystemInfo::format_memory(1073741824), "1.0 GB");
    }

    #[test]
    fn test_performance_monitor() {
        let mut monitor = PerformanceMonitor::new("test-util");
        assert_eq!(monitor.utility_name, "test-util");
        assert_eq!(monitor.metrics_history.len(), 0);

        // Test that we can start monitoring (may fail if sysinfo feature is not available)
        let _ = monitor.start();

        // Test report generation
        let report = monitor.generate_report();
        assert_eq!(report.utility_name, "test-util");
    }

    #[test]
    fn test_performance_report_display() {
        let report = PerformanceReport {
            utility_name: "test-util".to_string(),
            monitoring_duration: Duration::from_secs(30),
            measurements_count: 10,
            average_cpu_usage: 25.5,
            peak_memory_usage: 1024 * 1024, // 1 MB
            current_metrics: None,
            summary: "Test report".to_string(),
        };

        // Test that display doesn't panic
        let result = report.display_with_color(ColorChoice::Never);
        assert!(result.is_ok());
    }

    #[test]
    fn test_system_diagnostics() {
        let mut diagnostics = SystemDiagnostics::new("test-util");
        assert_eq!(diagnostics.utility_name, "test-util");

        // Test initialization
        let result = diagnostics.initialize();
        // May fail if sysinfo feature is not available, but shouldn't panic
        let _ = result;

        // Test that we can run diagnostics
        let diag_results = diagnostics.run_diagnostics();
        // Should always succeed even if individual checks fail
        assert!(diag_results.is_ok());
    }
}
