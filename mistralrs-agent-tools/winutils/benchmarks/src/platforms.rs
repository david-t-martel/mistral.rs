use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Platform {
    pub os: String,
    pub arch: String,
    pub version: String,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub memory_gb: u64,
    pub shell: String,
}

pub fn get_current_platform() -> Platform {
    use sysinfo::{System, SystemExt};

    let mut system = System::new_all();
    system.refresh_all();

    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();

    #[cfg(windows)]
    let version = get_windows_version().unwrap_or_else(|_| "Unknown".to_string());

    #[cfg(not(windows))]
    let version = "Unknown".to_string();

    let cpu_model = system.global_cpu_info().brand().to_string();
    let cpu_cores = system.cpus().len() as u32;
    let memory_gb = system.total_memory() / 1024 / 1024 / 1024;

    let shell = detect_shell();

    Platform {
        os,
        arch,
        version,
        cpu_model,
        cpu_cores,
        memory_gb,
        shell,
    }
}

#[cfg(windows)]
fn get_windows_version() -> Result<String> {
    use std::process::Command;

    let output = Command::new("cmd")
        .args(&["/C", "ver"])
        .output()?;

    let version_str = String::from_utf8_lossy(&output.stdout);

    // Parse Windows version from output like "Microsoft Windows [Version 10.0.19045.4780]"
    if let Some(start) = version_str.find("[Version ") {
        if let Some(end) = version_str[start..].find(']') {
            let version = &version_str[start + 9..start + end];
            return Ok(version.to_string());
        }
    }

    Ok("Unknown".to_string())
}

fn detect_shell() -> String {
    // Try to detect the current shell
    if let Ok(shell) = std::env::var("SHELL") {
        if let Some(shell_name) = shell.split('/').last() {
            return shell_name.to_string();
        }
    }

    // Windows-specific detection
    #[cfg(windows)]
    {
        if std::env::var("MSYSTEM").is_ok() {
            return "Git Bash".to_string();
        }

        if let Ok(comspec) = std::env::var("COMSPEC") {
            if comspec.contains("cmd.exe") {
                return "cmd".to_string();
            }
        }

        // Check for PowerShell
        if std::env::var("PSModulePath").is_ok() {
            return "PowerShell".to_string();
        }
    }

    "Unknown".to_string()
}

pub fn get_native_command(utility: &str) -> Result<String> {
    let native_commands = get_native_command_map();

    if let Some(cmd) = native_commands.get(utility) {
        // Verify the command exists
        if which::which(cmd).is_ok() {
            Ok(cmd.clone())
        } else {
            Err(anyhow!("Native command '{}' not found for utility '{}'", cmd, utility))
        }
    } else {
        // Default to the utility name itself
        if which::which(utility).is_ok() {
            Ok(utility.to_string())
        } else {
            Err(anyhow!("No native equivalent found for utility '{}'", utility))
        }
    }
}

fn get_native_command_map() -> HashMap<&'static str, String> {
    let mut map = HashMap::new();

    #[cfg(windows)]
    {
        // Windows native commands
        map.insert("ls", "dir".to_string());
        map.insert("cat", "type".to_string());
        map.insert("cp", "copy".to_string());
        map.insert("mv", "move".to_string());
        map.insert("rm", "del".to_string());
        map.insert("mkdir", "md".to_string());
        map.insert("rmdir", "rd".to_string());
        map.insert("pwd", "cd".to_string());
        map.insert("echo", "echo".to_string());
        map.insert("find", "findstr".to_string());
        map.insert("grep", "findstr".to_string());

        // Try to use Git Bash versions if available
        if std::env::var("MSYSTEM").is_ok() || which::which("bash").is_ok() {
            // Git Bash provides Unix-like utilities
            for utility in &["ls", "cat", "cp", "mv", "rm", "mkdir", "rmdir", "pwd", "echo", "find", "grep", "sort", "wc"] {
                if which::which(utility).is_ok() {
                    map.insert(utility, utility.to_string());
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        // Unix/Linux commands - usually the same name
        for utility in &["ls", "cat", "cp", "mv", "rm", "mkdir", "rmdir", "pwd", "echo", "find", "grep", "sort", "wc"] {
            map.insert(utility, utility.to_string());
        }
    }

    map
}

pub fn get_platform_optimizations() -> PlatformOptimizations {
    let platform = get_current_platform();

    #[cfg(windows)]
    {
        PlatformOptimizations {
            io_buffer_size: 64 * 1024, // 64KB optimal for NTFS
            use_overlapped_io: true,
            enable_large_pages: platform.memory_gb > 8,
            thread_pool_size: platform.cpu_cores.min(16),
            use_memory_mapped_files: true,
            compression_enabled: true,
        }
    }

    #[cfg(not(windows))]
    {
        PlatformOptimizations {
            io_buffer_size: 256 * 1024, // 256KB for Linux filesystems
            use_overlapped_io: false,
            enable_large_pages: platform.memory_gb > 16,
            thread_pool_size: platform.cpu_cores,
            use_memory_mapped_files: true,
            compression_enabled: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlatformOptimizations {
    pub io_buffer_size: usize,
    pub use_overlapped_io: bool,
    pub enable_large_pages: bool,
    pub thread_pool_size: u32,
    pub use_memory_mapped_files: bool,
    pub compression_enabled: bool,
}

/// Platform-specific performance tuning
pub fn apply_platform_tuning() -> Result<()> {
    #[cfg(windows)]
    {
        // Set process priority to high for benchmarking
        unsafe {
            use windows::Win32::System::Threading::*;
            use windows::Win32::Foundation::*;

            let handle = GetCurrentProcess();
            SetPriorityClass(handle, HIGH_PRIORITY_CLASS)?;
        }

        // Enable large page support if available
        enable_large_page_support()?;
    }

    Ok(())
}

#[cfg(windows)]
fn enable_large_page_support() -> Result<()> {
    unsafe {
        use windows::Win32::System::Memory::*;
        use windows::Win32::Foundation::*;

        // Try to enable large page support
        let size = GetLargePageMinimum();
        if size > 0 {
            println!("Large page support available: {} bytes", size);
        }
    }

    Ok(())
}

/// Get system performance characteristics
pub fn get_performance_baseline() -> PerformanceBaseline {
    use std::time::{Duration, Instant};

    // CPU speed test
    let cpu_start = Instant::now();
    let mut sum = 0u64;
    for i in 0..1_000_000 {
        sum = sum.wrapping_add(i);
    }
    let cpu_time = cpu_start.elapsed();

    // Memory bandwidth test (simplified)
    let memory_start = Instant::now();
    let data = vec![0u8; 1024 * 1024]; // 1MB
    let _checksum: u64 = data.iter().map(|&x| x as u64).sum();
    let memory_time = memory_start.elapsed();

    // Disk I/O test
    let io_time = benchmark_disk_io().unwrap_or(Duration::from_millis(100));

    PerformanceBaseline {
        cpu_performance_score: 1000.0 / cpu_time.as_millis() as f64,
        memory_bandwidth_score: 1000.0 / memory_time.as_micros() as f64,
        disk_io_score: 100.0 / io_time.as_millis() as f64,
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceBaseline {
    pub cpu_performance_score: f64,
    pub memory_bandwidth_score: f64,
    pub disk_io_score: f64,
}

fn benchmark_disk_io() -> Result<Duration> {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let start = std::time::Instant::now();

    let mut temp_file = NamedTempFile::new()?;
    let data = vec![0u8; 1024]; // 1KB

    for _ in 0..100 {
        temp_file.write_all(&data)?;
    }

    temp_file.flush()?;

    Ok(start.elapsed())
}
