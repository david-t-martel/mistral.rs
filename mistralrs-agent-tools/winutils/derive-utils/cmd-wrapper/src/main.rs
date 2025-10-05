use clap::{Arg, Command};
use log::{debug, error, info, warn};
use std::{
    env,
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

const CMD_EXE_PATHS: &[&str] = &[
    "C:\\Windows\\System32\\cmd.exe",
    "C:\\Windows\\SysWOW64\\cmd.exe",
    "%SystemRoot%\\System32\\cmd.exe",
];

/// Enhanced Windows cmd.exe wrapper with universal path normalization
#[derive(Debug)]
pub struct CmdWrapper {
    normalizer: PathNormalizer,
    preserve_args: bool,
    debug_mode: bool,
    cmd_path: String,
}

impl CmdWrapper {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let normalizer = PathNormalizer::new();
        let cmd_path = Self::find_cmd_exe()?;

        Ok(CmdWrapper {
            normalizer,
            preserve_args: false,
            debug_mode: env::var("WINPATH_DEBUG").is_ok(),
            cmd_path,
        })
    }

    /// Find the appropriate cmd.exe executable
    fn find_cmd_exe() -> Result<String, Box<dyn std::error::Error>> {
        for &path in CMD_EXE_PATHS {
            let expanded = expand_environment_string(path);
            if std::path::Path::new(&expanded).exists() {
                return Ok(expanded);
            }
        }

        // Fallback to system PATH search
        if let Ok(path) = which::which("cmd") {
            return Ok(path.to_string_lossy().to_string());
        }

        Err("Could not locate cmd.exe".into())
    }

    /// Normalize paths in command arguments
    pub fn normalize_args(&self, args: &[String]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut normalized_args = Vec::new();

        for arg in args {
            if self.preserve_args {
                normalized_args.push(arg.clone());
                continue;
            }

            // Skip cmd.exe flags that shouldn't be normalized
            if arg.starts_with('/') || arg.starts_with('-') {
                normalized_args.push(arg.clone());
                continue;
            }

            // Check if argument looks like a path
            if self.looks_like_path(arg) {
                match self.normalizer.normalize(arg) {
                    Ok(normalized) => {
                        let normalized_str = normalized.path().to_string();
                        if self.debug_mode {
                            debug!("Normalized path: {} -> {}", arg, normalized_str);
                        }
                        normalized_args.push(normalized_str);
                    }
                    Err(e) => {
                        if self.debug_mode {
                            warn!("Failed to normalize '{}': {}", arg, e);
                        }
                        normalized_args.push(arg.clone());
                    }
                }
            } else {
                normalized_args.push(arg.clone());
            }
        }

        Ok(normalized_args)
    }

    /// Heuristic to determine if a string looks like a path
    fn looks_like_path(&self, s: &str) -> bool {
        // Basic path indicators
        s.contains('/') ||
        s.contains('\\') ||
        s.contains(':') ||
        s.starts_with('.') ||
        s.starts_with('~') ||
        // WSL/Git Bash patterns
        s.starts_with("/mnt/") ||
        s.starts_with("/c/") ||
        s.starts_with("/cygdrive/") ||
        // Windows drive patterns
        (s.len() >= 2 && s.chars().nth(1) == Some(':'))
    }

    /// Execute cmd.exe with normalized arguments
    pub fn execute(&self, args: Vec<String>) -> Result<ExitCode, Box<dyn std::error::Error>> {
        let normalized_args = self.normalize_args(&args)?;

        if self.debug_mode {
            info!("Executing cmd.exe at: {}", self.cmd_path);
            info!("Original args: {:?}", args);
            info!("Normalized args: {:?}", normalized_args);
        }

        // Build command line for CreateProcessW
        let mut command_line = format!("\"{}\"", self.cmd_path);
        for arg in &normalized_args {
            command_line.push(' ');
            if arg.contains(' ') || arg.contains('"') {
                command_line.push('"');
                // Escape quotes in the argument
                for ch in arg.chars() {
                    if ch == '"' {
                        command_line.push('\\');
                    }
                    command_line.push(ch);
                }
                command_line.push('"');
            } else {
                command_line.push_str(arg);
            }
        }

        self.execute_process(&command_line)
    }

    /// Execute the process using Windows API for proper shell integration
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

/// Expand environment variables in a string (Windows-style)
fn expand_environment_string(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            // Find the closing %
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
                    // Keep original if variable not found
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

    let app = Command::new("cmd")
        .version("0.1.0")
        .about("Enhanced Windows cmd.exe wrapper with universal path normalization")
        .arg(Arg::new("preserve-args")
            .long("preserve-args")
            .help("Preserve original arguments without path normalization")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("debug")
            .long("debug")
            .help("Enable debug output")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("args")
            .help("Arguments to pass to cmd.exe")
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
    let mut wrapper = match CmdWrapper::new() {
        Ok(w) => w,
        Err(e) => {
            error!("Failed to initialize cmd wrapper: {}", e);
            return ExitCode::FAILURE;
        }
    };

    wrapper.preserve_args = preserve_args;
    wrapper.debug_mode = debug_mode;

    // Execute cmd.exe with processed arguments
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
        let wrapper = CmdWrapper::new().unwrap();

        assert!(wrapper.looks_like_path("C:\\Windows"));
        assert!(wrapper.looks_like_path("/mnt/c/Windows"));
        assert!(wrapper.looks_like_path("/c/Windows"));
        assert!(wrapper.looks_like_path("./relative/path"));
        assert!(wrapper.looks_like_path("../parent"));

        assert!(!wrapper.looks_like_path("/C"));
        assert!(!wrapper.looks_like_path("echo"));
        assert!(!wrapper.looks_like_path("hello world"));
    }

    #[test]
    fn test_arg_normalization() {
        let wrapper = CmdWrapper::new().unwrap();

        let args = vec![
            "/c".to_string(),
            "dir".to_string(),
            "/mnt/c/Windows".to_string(),
        ];

        let normalized = wrapper.normalize_args(&args).unwrap();

        // Should preserve /c flag but normalize the path
        assert_eq!(normalized[0], "/c");
        assert_eq!(normalized[1], "dir");
        // Path should be normalized to Windows format
        assert!(normalized[2].starts_with("C:\\") || normalized[2] == "/mnt/c/Windows");
    }

    #[test]
    fn test_environment_expansion() {
        env::set_var("TEST_VAR", "test_value");
        let result = expand_environment_string("%TEST_VAR%\\subdir");
        assert_eq!(result, "test_value\\subdir");

        let result = expand_environment_string("no_vars_here");
        assert_eq!(result, "no_vars_here");
    }

    #[test]
    fn test_cmd_exe_discovery() {
        let cmd_path = CmdWrapper::find_cmd_exe().unwrap();
        assert!(std::path::Path::new(&cmd_path).exists());
        assert!(cmd_path.to_lowercase().contains("cmd.exe"));
    }
}
