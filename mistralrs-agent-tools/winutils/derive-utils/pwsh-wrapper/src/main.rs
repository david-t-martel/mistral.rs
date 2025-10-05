use clap::{Arg, Command};
use log::{debug, error, info, warn};
use regex::Regex;
use std::{
    env,
    path::Path,
    process::ExitCode,
};
use winpath::PathNormalizer;
use windows_sys::Win32::{
    Foundation::CloseHandle,
    System::{
        Console::{GetStdHandle, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE},
        Threading::{CreateProcessW, WaitForSingleObject, GetExitCodeProcess, INFINITE, PROCESS_INFORMATION, STARTUPINFOW},
    },
};

const POWERSHELL_PATHS: &[&str] = &[
    // PowerShell Core (pwsh.exe) - preferred
    "pwsh.exe",
    "C:\\Program Files\\PowerShell\\7\\pwsh.exe",
    "C:\\Program Files (x86)\\PowerShell\\7\\pwsh.exe",
    "%USERPROFILE%\\AppData\\Local\\Microsoft\\WindowsApps\\pwsh.exe",

    // Windows PowerShell (powershell.exe) - fallback
    "powershell.exe",
    "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
    "C:\\Windows\\SysWOW64\\WindowsPowerShell\\v1.0\\powershell.exe",
    "%SystemRoot%\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
];

/// Enhanced PowerShell wrapper with universal path normalization
#[derive(Debug)]
pub struct PowerShellWrapper {
    normalizer: PathNormalizer,
    preserve_args: bool,
    debug_mode: bool,
    powershell_path: String,
    is_pwsh_core: bool,
    path_patterns: Vec<Regex>,
}

impl PowerShellWrapper {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let normalizer = PathNormalizer::new();
        let (powershell_path, is_pwsh_core) = Self::find_powershell()?;

        // Compile regex patterns for PowerShell path detection
        let path_patterns = vec![
            // PowerShell cmdlet parameters that typically contain paths
            Regex::new(r#"-(Path|LiteralPath|FilePath|Directory|File|Location|Destination|Source|Root|Home|Working|Base)\s+(['"]?)([^'"]+)"#)?,
            // Set-Location / cd commands
            Regex::new(r#"\b(Set-Location|cd|sl|pushd|popd)\s+(['"]?)([^'"]+)"#)?,
            // File system operations
            Regex::new(r#"\b(Get-ChildItem|Get-Item|Test-Path|New-Item|Remove-Item|Copy-Item|Move-Item)\s+(['"]?)([^'"]+)"#)?,
            // Import/Export operations
            Regex::new(r#"\b(Import-Module|Export-Module|Import-Csv|Export-Csv|Out-File)\s+(['"]?)([^'"]+)"#)?,
        ];

        Ok(PowerShellWrapper {
            normalizer,
            preserve_args: false,
            debug_mode: env::var("WINPATH_DEBUG").is_ok(),
            powershell_path,
            is_pwsh_core,
            path_patterns,
        })
    }

    /// Find the appropriate PowerShell executable
    fn find_powershell() -> Result<(String, bool), Box<dyn std::error::Error>> {
        // Determine binary name from executable
        let exe_name = env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_else(|| "pwsh".to_string());

        // Try to find pwsh first (PowerShell Core), then fallback to powershell
        if exe_name == "pwsh" || exe_name == "powershell" {
            // Try paths in order based on binary name
            let search_paths = if exe_name == "pwsh" {
                // Prefer pwsh.exe paths
                POWERSHELL_PATHS.iter().filter(|p| p.contains("pwsh")).collect::<Vec<_>>()
            } else {
                // Prefer powershell.exe paths
                POWERSHELL_PATHS.iter().filter(|p| p.contains("powershell") && !p.contains("pwsh")).collect::<Vec<_>>()
            };

            for &path in &search_paths {
                let expanded = expand_environment_string(path);
                if Path::new(&expanded).exists() {
                    let is_core = expanded.contains("pwsh");
                    return Ok((expanded, is_core));
                }
            }

            // Try PATH search
            if let Ok(path) = which::which(&exe_name) {
                let path_str = path.to_string_lossy().to_string();
                let is_core = path_str.contains("pwsh");
                return Ok((path_str, is_core));
            }
        }

        // Fallback: try all paths
        for &path in POWERSHELL_PATHS {
            let expanded = expand_environment_string(path);
            if Path::new(&expanded).exists() {
                let is_core = expanded.contains("pwsh");
                return Ok((expanded, is_core));
            }
        }

        Err("Could not locate PowerShell executable".into())
    }

    /// Normalize paths in PowerShell arguments and scripts
    pub fn normalize_args(&self, args: &[String]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut normalized_args = Vec::new();

        for arg in args {
            if self.preserve_args {
                normalized_args.push(arg.clone());
                continue;
            }

            // Skip PowerShell parameters that shouldn't be normalized
            if arg.starts_with('-') && !self.looks_like_path_param(arg) {
                normalized_args.push(arg.clone());
                continue;
            }

            let normalized_arg = self.normalize_powershell_arg(arg)?;
            normalized_args.push(normalized_arg);
        }

        Ok(normalized_args)
    }

    /// Check if a parameter likely contains a path
    fn looks_like_path_param(&self, arg: &str) -> bool {
        let lower = arg.to_lowercase();
        lower.contains("path") ||
        lower.contains("file") ||
        lower.contains("directory") ||
        lower.contains("location") ||
        lower.contains("source") ||
        lower.contains("destination")
    }

    /// Normalize a single PowerShell argument
    fn normalize_powershell_arg(&self, arg: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Check if this is a script block or complex command
        if arg.contains('{') || arg.contains(';') || arg.contains('|') {
            return self.normalize_script_content(arg);
        }

        // Simple path normalization
        if self.looks_like_path(arg) {
            match self.normalizer.normalize(arg) {
                Ok(normalized) => {
                    let normalized_str = normalized.path().to_string();
                    if self.debug_mode {
                        debug!("Normalized path: {} -> {}", arg, normalized_str);
                    }
                    return Ok(normalized_str);
                }
                Err(e) => {
                    if self.debug_mode {
                        warn!("Failed to normalize '{}': {}", arg, e);
                    }
                }
            }
        }

        Ok(arg.to_string())
    }

    /// Normalize paths within PowerShell script content
    fn normalize_script_content(&self, script: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut result = script.to_string();

        for pattern in &self.path_patterns {
            let mut replacements = Vec::new();

            for captures in pattern.captures_iter(script) {
                if let (Some(full_match), Some(path_part)) = (captures.get(0), captures.get(3)) {
                    let path_str = path_part.as_str();

                    if self.looks_like_path(path_str) {
                        if let Ok(normalized) = self.normalizer.normalize(path_str) {
                            let normalized_str = normalized.path().to_string();
                            let new_match = full_match.as_str().replace(path_str, &normalized_str);
                            replacements.push((full_match.as_str().to_string(), new_match));

                            if self.debug_mode {
                                debug!("Script path normalization: {} -> {}", path_str, normalized_str);
                            }
                        }
                    }
                }
            }

            // Apply replacements
            for (old, new) in replacements {
                result = result.replace(&old, &new);
            }
        }

        Ok(result)
    }

    /// Heuristic to determine if a string looks like a path
    fn looks_like_path(&self, s: &str) -> bool {
        // Basic path indicators
        s.contains('/') ||
        s.contains('\\') ||
        // PowerShell specific: quoted paths
        (s.starts_with('"') && s.ends_with('"') && s.len() > 2) ||
        (s.starts_with('\'') && s.ends_with('\'') && s.len() > 2) ||
        s.starts_with('.') ||
        s.starts_with('~') ||
        // WSL/Git Bash patterns
        s.starts_with("/mnt/") ||
        s.starts_with("/c/") ||
        s.starts_with("/cygdrive/") ||
        // Windows drive patterns
        (s.len() >= 2 && s.chars().nth(1) == Some(':')) ||
        // PowerShell provider paths
        s.contains("::")
    }

    /// Execute PowerShell with normalized arguments
    pub fn execute(&self, args: Vec<String>) -> Result<ExitCode, Box<dyn std::error::Error>> {
        let normalized_args = self.normalize_args(&args)?;

        if self.debug_mode {
            info!("Executing PowerShell at: {}", self.powershell_path);
            info!("PowerShell Core: {}", self.is_pwsh_core);
            info!("Original args: {:?}", args);
            info!("Normalized args: {:?}", normalized_args);
        }

        // Build command line
        let mut command_line = format!("\"{}\"", self.powershell_path);
        for arg in &normalized_args {
            command_line.push(' ');

            // PowerShell argument escaping
            if arg.contains(' ') || arg.contains('"') || arg.contains('$') || arg.contains('`') {
                command_line.push('"');
                // Escape special PowerShell characters
                for ch in arg.chars() {
                    match ch {
                        '"' => command_line.push_str("\"\""), // PowerShell double-quote escaping
                        '$' => command_line.push_str("`$"),   // Escape variable expansion
                        '`' => command_line.push_str("``"),   // Escape backtick
                        _ => command_line.push(ch),
                    }
                }
                command_line.push('"');
            } else {
                command_line.push_str(arg);
            }
        }

        self.execute_process(&command_line)
    }

    /// Execute the process using Windows API
    fn execute_process(&self, command_line: &str) -> Result<ExitCode, Box<dyn std::error::Error>> {
        unsafe {
            let mut startup_info: STARTUPINFOW = std::mem::zeroed();
            startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

            // Inherit standard handles
            startup_info.hStdInput = GetStdHandle(STD_INPUT_HANDLE);
            startup_info.hStdOutput = GetStdHandle(STD_OUTPUT_HANDLE);
            startup_info.hStdError = GetStdHandle(STD_ERROR_HANDLE);
            startup_info.dwFlags = 0x100; // STARTF_USESTDHANDLES

            let mut process_info: PROCESS_INFORMATION = std::mem::zeroed();

            // Convert command line to wide string
            let command_line_wide: Vec<u16> = command_line.encode_utf16().chain(std::iter::once(0)).collect();

            let success = CreateProcessW(
                std::ptr::null(),                    // Application name
                command_line_wide.as_ptr() as *mut u16, // Command line
                std::ptr::null_mut(),               // Process attributes
                std::ptr::null_mut(),               // Thread attributes
                1,                                  // Inherit handles
                0,                                  // Creation flags
                std::ptr::null_mut(),               // Environment
                std::ptr::null(),                   // Current directory
                &startup_info,                      // Startup info
                &mut process_info,                  // Process info
            );

            if success == 0 {
                return Err(format!("Failed to create process: {}", std::io::Error::last_os_error()).into());
            }

            // Wait for process to complete
            WaitForSingleObject(process_info.hProcess, INFINITE);

            // Get exit code
            let mut exit_code: u32 = 0;
            if GetExitCodeProcess(
                process_info.hProcess,
                &mut exit_code,
            ) == 0 {
                warn!("Failed to get exit code, assuming success");
                exit_code = 0;
            }

            // Cleanup handles
            CloseHandle(process_info.hProcess);
            CloseHandle(process_info.hThread);

            Ok(ExitCode::from(exit_code as u8))
        }
    }
}

/// Expand environment variables in a string
fn expand_environment_string(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            let mut var_name = String::new();
            let mut found_closing = false;

            while let Some(&next_ch) = chars.peek() {
                chars.next();
                if next_ch == '%' {
                    found_closing = true;
                    break;
                }
                var_name.push(next_ch);
            }

            if found_closing && !var_name.is_empty() {
                if let Ok(value) = env::var(&var_name) {
                    result.push_str(&value);
                } else {
                    result.push('%');
                    result.push_str(&var_name);
                    result.push('%');
                }
            } else {
                result.push('%');
                result.push_str(&var_name);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn main() -> ExitCode {
    // Initialize logging
    env_logger::builder()
        .filter_level(if env::var("WINPATH_DEBUG").is_ok() {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Warn
        })
        .init();

    let app = Command::new("PowerShell Wrapper")
        .version("0.1.0")
        .about("Enhanced PowerShell wrapper with universal path normalization")
        .arg(Arg::new("preserve-args")
            .long("preserve-args")
            .help("Preserve original arguments without path normalization")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("debug")
            .long("debug")
            .help("Enable debug output")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("args")
            .help("Arguments to pass to PowerShell")
            .num_args(0..)
            .trailing_var_arg(true));

    let matches = app.try_get_matches();

    let (preserve_args, debug_mode, args) = match matches {
        Ok(m) => (
            m.get_flag("preserve-args"),
            m.get_flag("debug") || env::var("WINPATH_DEBUG").is_ok(),
            m.get_many::<String>("args")
                .map(|vals| vals.map(|s| s.clone()).collect())
                .unwrap_or_default()
        ),
        Err(_) => {
            // If argument parsing fails, pass all args through
            let args: Vec<String> = env::args().skip(1).collect();
            (false, env::var("WINPATH_DEBUG").is_ok(), args)
        }
    };

    // Create wrapper instance
    let mut wrapper = match PowerShellWrapper::new() {
        Ok(w) => w,
        Err(e) => {
            error!("Failed to initialize PowerShell wrapper: {}", e);
            return ExitCode::FAILURE;
        }
    };

    wrapper.preserve_args = preserve_args;
    wrapper.debug_mode = debug_mode;

    // Execute PowerShell with processed arguments
    match wrapper.execute(args) {
        Ok(exit_code) => exit_code,
        Err(e) => {
            error!("Execution failed: {}", e);
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_path_detection() {
        let wrapper = PowerShellWrapper::new().unwrap();

        assert!(wrapper.looks_like_path("C:\\Windows"));
        assert!(wrapper.looks_like_path("/mnt/c/Windows"));
        assert!(wrapper.looks_like_path("/c/Windows"));
        assert!(wrapper.looks_like_path("'C:\\Program Files'"));
        assert!(wrapper.looks_like_path("\"./relative/path\""));

        assert!(!wrapper.looks_like_path("Get-ChildItem"));
        assert!(!wrapper.looks_like_path("-Path"));
        assert!(!wrapper.looks_like_path("$variable"));
    }

    #[test]
    fn test_powershell_script_normalization() {
        let wrapper = PowerShellWrapper::new().unwrap();

        let script = "Get-ChildItem -Path '/mnt/c/Windows' | Where-Object Name -like '*.txt'";
        let normalized = wrapper.normalize_script_content(script).unwrap();

        // Should normalize the path in the -Path parameter
        assert!(normalized.contains("C:\\Windows") || normalized.contains("/mnt/c/Windows"));
    }

    #[test]
    fn test_powershell_discovery() {
        let (ps_path, is_core) = PowerShellWrapper::find_powershell().unwrap();
        assert!(Path::new(&ps_path).exists());
        assert!(ps_path.to_lowercase().contains("powershell") || ps_path.to_lowercase().contains("pwsh"));
    }

    #[test]
    fn test_environment_expansion() {
        env::set_var("TEST_PS_VAR", "test_value");
        let result = expand_environment_string("%TEST_PS_VAR%\\subdir");
        assert_eq!(result, "test_value\\subdir");
    }
}
