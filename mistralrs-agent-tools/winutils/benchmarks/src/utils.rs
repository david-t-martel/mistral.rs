use anyhow::{Result, anyhow};
use colored::*;
use std::path::Path;
use std::process::Command;
use which::which;

pub async fn validate_environment() -> Result<()> {
    println!("{}", "🔧 Validating benchmark environment...".bright_blue());

    // Check Rust toolchain
    validate_rust_toolchain()?;

    // Check winutils binaries
    validate_winutils_binaries().await?;

    // Check native utilities for comparison
    validate_native_utilities()?;

    // Check system resources
    validate_system_resources()?;

    // Check disk space
    validate_disk_space()?;

    println!("{}", "✅ Environment validation completed".bright_green());
    Ok(())
}

fn validate_rust_toolchain() -> Result<()> {
    println!("  📦 Checking Rust toolchain...");

    // Check cargo
    if which("cargo").is_err() {
        return Err(anyhow!("cargo not found in PATH"));
    }

    // Check rustc version
    let output = Command::new("rustc")
        .arg("--version")
        .output()?;

    let version = String::from_utf8_lossy(&output.stdout);
    println!("    Rust version: {}", version.trim());

    // Check for required targets
    let output = Command::new("rustup")
        .args(&["target", "list", "--installed"])
        .output();

    if let Ok(output) = output {
        let targets = String::from_utf8_lossy(&output.stdout);
        println!("    Installed targets: {}", targets.lines().count());
    }

    Ok(())
}

async fn validate_winutils_binaries() -> Result<()> {
    println!("  🔧 Checking WinUtils binaries...");

    let target_dir = Path::new("../target/release");

    if !target_dir.exists() {
        return Err(anyhow!("Release target directory not found: {}. Run 'make release' first.", target_dir.display()));
    }

    let expected_utilities = [
        "ls", "cat", "cp", "mv", "rm", "mkdir", "rmdir", "pwd", "echo",
        "grep", "find", "sort", "wc", "head", "tail", "cut", "tr",
        "chmod", "chown", "du", "df", "ps", "kill", "touch", "which"
    ];

    let mut found_count = 0;
    let mut missing_utilities = Vec::new();

    for utility in &expected_utilities {
        let binary_path = target_dir.join(&format!("{}.exe", utility));
        if binary_path.exists() {
            found_count += 1;
        } else {
            missing_utilities.push(*utility);
        }
    }

    println!("    Found {}/{} utilities", found_count, expected_utilities.len());

    if !missing_utilities.is_empty() && missing_utilities.len() < expected_utilities.len() / 2 {
        println!("    ⚠️  Missing utilities: {}", missing_utilities.join(", "));
    } else if missing_utilities.len() >= expected_utilities.len() / 2 {
        return Err(anyhow!("Too many utilities missing. Build may have failed."));
    }

    Ok(())
}

fn validate_native_utilities() -> Result<()> {
    println!("  🏠 Checking native utilities for comparison...");

    let native_utilities = get_available_native_utilities();

    if native_utilities.is_empty() {
        println!("    ⚠️  No native utilities found for comparison");
    } else {
        println!("    Found {} native utilities", native_utilities.len());
        for utility in &native_utilities {
            println!("      ✓ {}", utility);
        }
    }

    Ok(())
}

fn get_available_native_utilities() -> Vec<String> {
    let mut utilities = Vec::new();

    #[cfg(windows)]
    let potential_utilities = vec![
        ("ls", vec!["ls", "dir"]),
        ("cat", vec!["cat", "type"]),
        ("cp", vec!["cp", "copy"]),
        ("mv", vec!["mv", "move"]),
        ("rm", vec!["rm", "del"]),
        ("grep", vec!["grep", "findstr"]),
        ("sort", vec!["sort"]),
        ("wc", vec!["wc"]),
    ];

    #[cfg(not(windows))]
    let potential_utilities = vec![
        ("ls", vec!["ls"]),
        ("cat", vec!["cat"]),
        ("cp", vec!["cp"]),
        ("mv", vec!["mv"]),
        ("rm", vec!["rm"]),
        ("grep", vec!["grep"]),
        ("sort", vec!["sort"]),
        ("wc", vec!["wc"]),
    ];

    for (utility_name, commands) in potential_utilities {
        for cmd in commands {
            if which(cmd).is_ok() {
                utilities.push(format!("{} ({})", utility_name, cmd));
                break;
            }
        }
    }

    utilities
}

fn validate_system_resources() -> Result<()> {
    println!("  💾 Checking system resources...");

    use sysinfo::{System, SystemExt};

    let mut system = System::new_all();
    system.refresh_all();

    let total_memory_gb = system.total_memory() / 1024 / 1024 / 1024;
    let available_memory_gb = system.available_memory() / 1024 / 1024 / 1024;
    let cpu_count = system.cpus().len();

    println!("    CPU cores: {}", cpu_count);
    println!("    Total memory: {}GB", total_memory_gb);
    println!("    Available memory: {}GB", available_memory_gb);

    if available_memory_gb < 2 {
        return Err(anyhow!("Insufficient available memory: {}GB (minimum 2GB recommended)", available_memory_gb));
    }

    if cpu_count < 2 {
        println!("    ⚠️  Single core system detected - benchmarks may be less reliable");
    }

    Ok(())
}

fn validate_disk_space() -> Result<()> {
    println!("  💿 Checking disk space...");

    use sysinfo::{DiskExt, System, SystemExt};

    let mut system = System::new_all();
    system.refresh_all();

    let current_dir = std::env::current_dir()?;

    // Find the disk containing the current directory
    for disk in system.disks() {
        if current_dir.starts_with(disk.mount_point()) {
            let available_gb = disk.available_space() / 1024 / 1024 / 1024;
            let total_gb = disk.total_space() / 1024 / 1024 / 1024;

            println!("    Disk: {}", disk.name().to_string_lossy());
            println!("    Available space: {}GB / {}GB", available_gb, total_gb);

            if available_gb < 1 {
                return Err(anyhow!("Insufficient disk space: {}GB (minimum 1GB recommended)", available_gb));
            }

            break;
        }
    }

    Ok(())
}

pub fn format_duration(duration: std::time::Duration) -> String {
    let nanos = duration.as_nanos();

    if nanos < 1000 {
        format!("{}ns", nanos)
    } else if nanos < 1_000_000 {
        format!("{:.1}μs", nanos as f64 / 1000.0)
    } else if nanos < 1_000_000_000 {
        format!("{:.1}ms", nanos as f64 / 1_000_000.0)
    } else {
        format!("{:.1}s", nanos as f64 / 1_000_000_000.0)
    }
}

pub fn format_memory(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn format_speedup(speedup: f64) -> String {
    if speedup >= 1.0 {
        format!("{:.1}x faster", speedup)
    } else {
        format!("{:.1}x slower", 1.0 / speedup)
    }
}

/// Generate a progress bar for long-running operations
pub struct ProgressBar {
    current: u64,
    total: u64,
    width: usize,
}

impl ProgressBar {
    pub fn new(total: u64) -> Self {
        Self {
            current: 0,
            total,
            width: 50,
        }
    }

    pub fn update(&mut self, current: u64) {
        self.current = current;
        self.render();
    }

    pub fn increment(&mut self) {
        self.current += 1;
        self.render();
    }

    pub fn finish(&mut self) {
        self.current = self.total;
        self.render();
        println!(); // New line after completion
    }

    fn render(&self) {
        if self.total == 0 {
            return;
        }

        let progress = self.current as f64 / self.total as f64;
        let filled = (progress * self.width as f64) as usize;
        let empty = self.width - filled;

        let bar = format!("{}{}",
            "█".repeat(filled).bright_green(),
            "░".repeat(empty).bright_black()
        );

        print!("\r[{}] {:.1}% ({}/{})",
            bar,
            progress * 100.0,
            self.current,
            self.total
        );

        use std::io::{self, Write};
        io::stdout().flush().unwrap();
    }
}

/// Enhanced error reporting for benchmark failures
pub fn report_benchmark_error(utility: &str, test_case: &str, error: &anyhow::Error) {
    println!("{}", format!("❌ Benchmark failed: {} - {}", utility, test_case).bright_red().bold());
    println!("   Error: {}", error);

    // Print error chain
    let mut source = error.source();
    let mut level = 1;
    while let Some(err) = source {
        println!("   {}: {}", "Caused by".bright_yellow(), err);
        source = err.source();
        level += 1;
        if level > 5 { break; } // Prevent infinite loops
    }
}

/// System information gathering for debugging
pub fn gather_system_info() -> SystemInfo {
    use sysinfo::{System, SystemExt};

    let mut system = System::new_all();
    system.refresh_all();

    SystemInfo {
        hostname: system.host_name().unwrap_or_else(|| "Unknown".to_string()),
        os_name: system.name().unwrap_or_else(|| "Unknown".to_string()),
        os_version: system.os_version().unwrap_or_else(|| "Unknown".to_string()),
        kernel_version: system.kernel_version().unwrap_or_else(|| "Unknown".to_string()),
        cpu_model: system.global_cpu_info().brand().to_string(),
        cpu_cores: system.cpus().len() as u32,
        total_memory: system.total_memory(),
        available_memory: system.available_memory(),
        uptime: system.uptime(),
    }
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub total_memory: u64,
    pub available_memory: u64,
    pub uptime: u64,
}

impl SystemInfo {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "hostname": self.hostname,
            "os_name": self.os_name,
            "os_version": self.os_version,
            "kernel_version": self.kernel_version,
            "cpu_model": self.cpu_model,
            "cpu_cores": self.cpu_cores,
            "total_memory": self.total_memory,
            "available_memory": self.available_memory,
            "uptime": self.uptime,
        })
    }
}
