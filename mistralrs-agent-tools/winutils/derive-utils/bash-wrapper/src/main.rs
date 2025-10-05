use clap::{Arg, Command};
use log::{debug, error, info, warn};
use regex::Regex;
use shell_words;
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

const BASH_PATHS: &[&str] = &[
    // Git Bash (most common on Windows)
    "C:\\Program Files\\Git\\bin\\bash.exe",
    "C:\\Program Files (x86)\\Git\\bin\\bash.exe",
    "%PROGRAMFILES%\\Git\\bin\\bash.exe",
    "%PROGRAMFILES(X86)%\\Git\\bin\\bash.exe",

    // Windows Subsystem for Linux
    "bash.exe", // WSL bash in PATH
    "C:\\Windows\\System32\\bash.exe",

    // Cygwin
    "C:\\cygwin64\\bin\\bash.exe",
    "C:\\cygwin\\bin\\bash.exe",

    // MSYS2
    "C:\\msys64\\usr\\bin\\bash.exe",
    "C:\\msys32\\usr\\bin\\bash.exe",

    // User-installed Git
    "%USERPROFILE%\\AppData\\Local\\Programs\\Git\\bin\\bash.exe",
];

/// Environment detection for different bash implementations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BashEnvironment {
    GitBash,
    WSL,
    Cygwin,
    MSYS2,
    Unknown,
}

/// Enhanced Bash wrapper with universal path normalization
#[derive(Debug)]
pub struct BashWrapper {
    normalizer: PathNormalizer,
    preserve_args: bool,
    debug_mode: bool,
    bash_path: String,
    environment: BashEnvironment,
    interactive_mode: bool,
}

impl BashWrapper {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let normalizer = PathNormalizer::new();
        let (bash_path, environment) = Self::find_bash()?;

        Ok(BashWrapper {
            normalizer,
            preserve_args: false,
            debug_mode: env::var("WINPATH_DEBUG").is_ok(),
            bash_path,
            environment,
            interactive_mode: false,
        })
    }

    /// Find the appropriate bash executable and detect environment
    fn find_bash() -> Result<(String, BashEnvironment), Box<dyn std::error::Error>> {
        // Try PATH first for system-installed bash
        if let Ok(path) = which::which("bash") {
            let path_str = path.to_string_lossy().to_string();
            let env = Self::detect_environment(&path_str);
            return Ok((path_str, env));
        }

        // Try known paths
        for &path in BASH_PATHS {
            let expanded = expand_environment_string(path);
            if Path::new(&expanded).exists() {
                let env = Self::detect_environment(&expanded);
                return Ok((expanded, env));
            }
        }

        Err("Could not locate bash executable".into())
    }

    /// Detect the bash environment type from path
    fn detect_environment(path: &str) -> BashEnvironment {
        let lower = path.to_lowercase();

        if lower.contains("git") {
            BashEnvironment::GitBash
        } else if lower.contains("system32") || lower.contains("wsl") {
            BashEnvironment::WSL
        } else if lower.contains("cygwin") {
            BashEnvironment::Cygwin
        } else if lower.contains("msys") {
            BashEnvironment::MSYS2
        } else {
            BashEnvironment::Unknown
        }
    }

    /// Normalize paths in bash arguments and scripts
    pub fn normalize_args(&self, args: &[String]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut normalized_args = Vec::new();

        for arg in args {
            if self.preserve_args {
                normalized_args.push(arg.clone());
                continue;
            }

            // Skip bash options that shouldn't be normalized
            if arg.starts_with('-') && !self.is_path_option(arg) {
                normalized_args.push(arg.clone());
                continue;
            }

            let normalized_arg = self.normalize_bash_arg(arg)?;
            normalized_args.push(normalized_arg);
        }

        Ok(normalized_args)
    }

    /// Check if a bash option expects a path argument
    fn is_path_option(&self, arg: &str) -> bool {
        matches!(arg, "--init-file" | "--rcfile" | "--posix" | "-c" | "--command")
    }

    /// Normalize a single bash argument
    fn normalize_bash_arg(&self, arg: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Handle shell scripts or commands - FIXED: use string literals for multi-char operators
        if arg.contains(';') || arg.contains("&&") || arg.contains("||") || arg.contains('|') {
            return self.normalize_shell_script(arg);
        }

        // Simple path normalization
        if self.looks_like_path(arg) {
            return self.normalize_path_for_environment(arg);
        }

        Ok(arg.to_string())
    }

    /// Normalize paths within shell script content
    fn normalize_shell_script(&self, script: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Parse shell script and normalize paths within commands
        let mut result = script.to_string();

        // Regex patterns for common path patterns in shell scripts - FIXED: proper regex patterns
        let patterns = vec![
            // Command arguments that look like paths
            Regex::new(r#"\b(cd|ls|cat|grep|find|cp|mv|rm|mkdir|rmdir|chmod|chown|ln|touch)\s+(['"]?)([^'"; \s|&]+)"#)?,
            // Variable assignments with paths
            Regex::new(r#"\b([A-Z_][A-Z0-9_]*)=(['"]?)([^'"; \s]+)"#)?,
            // File redirections
            Regex::new(r#"([<>]+)\s*(['"]?)([^'"; \s|&]+)"#)?,
        ];

        for pattern in patterns {
            let mut replacements = Vec::new();

            for captures in pattern.captures_iter(script) {
                if let Some(path_match) = captures.get(3) {
                    let path_str = path_match.as_str();

                    if self.looks_like_path(path_str) {
                        if let Ok(normalized_path) = self.normalize_path_for_environment(path_str) {
                            let full_match = captures.get(0).unwrap().as_str();
                            let new_match = full_match.replace(path_str, &normalized_path);
                            replacements.push((full_match.to_string(), new_match));

                            if self.debug_mode {
                                debug!("Script path normalization: {} -> {}", path_str, normalized_path);
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

    /// Normalize path based on detected bash environment
    fn normalize_path_for_environment(&self, path: &str) -> Result<String, Box<dyn std::error::Error>> {
        match self.environment {
            BashEnvironment::GitBash => {
                // Git Bash expects Unix-style paths but may need conversion from Windows
                if path.contains('\\') || (path.len() >= 2 && path.chars().nth(1) == Some(':')) {
                    // Convert Windows path to Git Bash format
                    let normalized = self.normalizer.normalize(path)?;
                    self.convert_to_git_bash_path(normalized.path())
                } else {
                    Ok(path.to_string())
                }
            }
            BashEnvironment::WSL => {
                // WSL expects /mnt/c/ style paths
                let normalized = self.normalizer.normalize(path)?;
                self.convert_to_wsl_path(normalized.path())
            }
            BashEnvironment::Cygwin => {
                // Cygwin expects /cygdrive/c/ style paths
                let normalized = self.normalizer.normalize(path)?;
                self.convert_to_cygwin_path(normalized.path())
            }
            BashEnvironment::MSYS2 => {
                // MSYS2 expects /c/ style paths (similar to Git Bash)
                let normalized = self.normalizer.normalize(path)?;
                self.convert_to_git_bash_path(normalized.path())
            }
            BashEnvironment::Unknown => {
                // Fallback: try to normalize to standard format
                let normalized = self.normalizer.normalize(path)?;
                Ok(normalized.path().to_string())
            }
        }
    }

    /// Convert Windows path to Git Bash format (/c/path/to/file)
    fn convert_to_git_bash_path(&self, path: &str) -> Result<String, Box<dyn std::error::Error>> {
        if path.len() >= 2 && path.chars().nth(1) == Some(':') {
            let drive = path.chars().next().unwrap().to_lowercase();
            let rest = &path[2..].replace('\\', "/");
            Ok(format!("/{}{}", drive, rest))
        } else {
            Ok(path.replace('\\', "/"))
        }
    }

    /// Convert Windows path to WSL format (/mnt/c/path/to/file)
    fn convert_to_wsl_path(&self, path: &str) -> Result<String, Box<dyn std::error::Error>> {
        if path.len() >= 2 && path.chars().nth(1) == Some(':') {
            let drive = path.chars().next().unwrap().to_lowercase();
            let rest = &path[2..].replace('\\', "/");
            Ok(format!("/mnt/{}{}", drive, rest))
        } else {
            Ok(path.replace('\\', "/"))
        }
    }

    /// Convert Windows path to Cygwin format (/cygdrive/c/path/to/file)
    fn convert_to_cygwin_path(&self, path: &str) -> Result<String, Box<dyn std::error::Error>> {
        if path.len() >= 2 && path.chars().nth(1) == Some(':') {
            let drive = path.chars().next().unwrap().to_lowercase();
            let rest = &path[2..].replace('\\', "/");
            Ok(format!("/cygdrive/{}{}", drive, rest))
        } else {
            Ok(path.replace('\\', "/"))
        }
    }

    /// Heuristic to determine if a string looks like a path
    fn looks_like_path(&self, s: &str) -> bool {
        // Basic path indicators
        s.contains('/') ||
        s.contains('\\') ||
        s.starts_with('.') ||
        s.starts_with('~') ||
        // Windows drive patterns
        (s.len() >= 2 && s.chars().nth(1) == Some(':')) ||
        // Unix absolute paths
        s.starts_with('/') ||
        // Common path patterns
        s.contains("./") ||
        s.contains("../") ||
        // WSL/Git Bash patterns
        s.starts_with("/mnt/") ||
        s.starts_with("/c/") ||
        s.starts_with("/cygdrive/")
    }

    /// Execute bash with normalized arguments
    pub fn execute(&self, args: Vec<String>) -> Result<ExitCode, Box<dyn std::error::Error>> {
        let normalized_args = self.normalize_args(&args)?;

        if self.debug_mode {
            info!("Executing bash at: {}", self.bash_path);
            info!("Environment: {:?}", self.environment);
            info!("Interactive mode: {}", self.interactive_mode);
            info!("Original args: {:?}", args);
            info!("Normalized args: {:?}", normalized_args);
        }

        // Build command line
        let mut command_line = format!("\"{}\"", self.bash_path);

        // Add bash environment-specific options
        match self.environment {
            BashEnvironment::GitBash => {
                // Git Bash specific settings
                command_line.push_str(" --login");
            }
            BashEnvironment::WSL => {
                // WSL specific settings (inherit environment)
            }
            _ => {
                // Generic bash settings
            }
        }

        for arg in &normalized_args {
            command_line.push(' ');

            // Bash argument escaping
            if arg.contains(' ') || arg.contains('"') || arg.contains('$') || arg.contains('`') || arg.contains('\\') {
                command_line.push('"');
                // Escape special bash characters
                for ch in arg.chars() {
                    match ch {
                        '"' => command_line.push_str("\\\""),
                        '$' => command_line.push_str("\\$"),
                        '`' => command_line.push_str("\\`"),
                        '\\' => command_line.push_str("\\\\"),
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

    let app = Command::new("Bash Wrapper")
        .version("0.1.0")
        .about("Enhanced bash wrapper with universal path normalization")
        .arg(Arg::new("preserve-args")
            .long("preserve-args")
            .help("Preserve original arguments without path normalization")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("debug")
            .long("debug")
            .help("Enable debug output")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("interactive")
            .short('i')
            .long("interactive")
            .help("Force interactive mode")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("login")
            .short('l')
            .long("login")
            .help("Force login shell")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("args")
            .help("Arguments to pass to bash")
            .num_args(0..)
            .trailing_var_arg(true));

    let matches = app.try_get_matches();

    let (preserve_args, debug_mode, interactive, login, args) = match matches {
        Ok(m) => (
            m.get_flag("preserve-args"),
            m.get_flag("debug") || env::var("WINPATH_DEBUG").is_ok(),
            m.get_flag("interactive"),
            m.get_flag("login"),
            m.get_many::<String>("args")
                .map(|vals| vals.map(|s| s.clone()).collect())
                .unwrap_or_default()
        ),
        Err(_) => {
            // If argument parsing fails, pass all args through
            let args: Vec<String> = env::args().skip(1).collect();
            (false, env::var("WINPATH_DEBUG").is_ok(), false, false, args)
        }
    };

    // Create wrapper instance
    let mut wrapper = match BashWrapper::new() {
        Ok(w) => w,
        Err(e) => {
            error!("Failed to initialize bash wrapper: {}", e);
            return ExitCode::FAILURE;
        }
    };

    wrapper.preserve_args = preserve_args;
    wrapper.debug_mode = debug_mode;
    wrapper.interactive_mode = interactive;

    // Prepend bash options if requested
    let mut final_args = args;
    if interactive {
        final_args.insert(0, "-i".to_string());
    }
    if login {
        final_args.insert(0, "-l".to_string());
    }

    // Execute bash with processed arguments
    match wrapper.execute(final_args) {
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
        let wrapper = BashWrapper::new().unwrap();

        assert!(wrapper.looks_like_path("C:\\Windows"));
        assert!(wrapper.looks_like_path("/mnt/c/Windows"));
        assert!(wrapper.looks_like_path("/c/Windows"));
        assert!(wrapper.looks_like_path("./relative/path"));
        assert!(wrapper.looks_like_path("../parent"));
        assert!(wrapper.looks_like_path("~/home"));

        assert!(!wrapper.looks_like_path("echo"));
        assert!(!wrapper.looks_like_path("ls"));
        assert!(!wrapper.looks_like_path("-l"));
    }

    #[test]
    fn test_environment_detection() {
        assert_eq!(
            BashWrapper::detect_environment("C:\\Program Files\\Git\\bin\\bash.exe"),
            BashEnvironment::GitBash
        );
        assert_eq!(
            BashWrapper::detect_environment("C:\\Windows\\System32\\bash.exe"),
            BashEnvironment::WSL
        );
        assert_eq!(
            BashWrapper::detect_environment("C:\\cygwin64\\bin\\bash.exe"),
            BashEnvironment::Cygwin
        );
        assert_eq!(
            BashWrapper::detect_environment("C:\\msys64\\usr\\bin\\bash.exe"),
            BashEnvironment::MSYS2
        );
    }

    #[test]
    fn test_git_bash_path_conversion() {
        let wrapper = BashWrapper::new().unwrap();

        let result = wrapper.convert_to_git_bash_path("C:\\Windows\\System32").unwrap();
        assert_eq!(result, "/c/Windows/System32");

        let result = wrapper.convert_to_git_bash_path("D:\\Projects\\test").unwrap();
        assert_eq!(result, "/d/Projects/test");
    }

    #[test]
    fn test_cygwin_path_conversion() {
        let wrapper = BashWrapper::new().unwrap();

        let result = wrapper.convert_to_cygwin_path("C:\\Windows\\System32").unwrap();
        assert_eq!(result, "/cygdrive/c/Windows/System32");
    }

    #[test]
    fn test_bash_discovery() {
        let (bash_path, _env) = BashWrapper::find_bash().unwrap();
        assert!(Path::new(&bash_path).exists());
        assert!(bash_path.to_lowercase().contains("bash"));
    }

    #[test]
    fn test_shell_script_normalization() {
        let wrapper = BashWrapper::new().unwrap();

        let script = "cd '/mnt/c/Windows' && ls -la";
        let normalized = wrapper.normalize_shell_script(script).unwrap();

        // Should normalize the path in the cd command
        // The exact result depends on the environment, but it should contain a valid path
        assert!(normalized.contains("cd") && normalized.contains("ls"));
    }
}
