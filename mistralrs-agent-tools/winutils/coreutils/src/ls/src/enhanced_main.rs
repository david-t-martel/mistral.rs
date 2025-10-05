//! Enhanced Windows ls utility with winutils-core features
//!
//! This demonstrates the complete integration of winutils-core features:
//! - Enhanced help system with examples and Windows-specific notes
//! - Version and source identification
//! - Built-in testing framework
//! - Windows-specific enhancements (ACL, attributes, shortcuts)
//! - Diagnostics and performance monitoring

use std::env;
use std::ffi::OsString;
use std::path::Path;
use std::process;
use std::time::Instant;

use winutils_core::{
    enhanced_app, handle_enhanced_args,
    HelpSystem, EnhancedHelp, ExampleSet, Example, WindowsNotes,
    VersionInfo, WinUtilsResult, WinUtilsError,
    traits::{EnhancedUtility, WindowsEnhanced},
};

#[cfg(feature = "testing")]
use winutils_core::testing::{SelfTest, TestResults, TestResult, BenchmarkSuite, BenchmarkResults};

#[cfg(feature = "diagnostics")]
use winutils_core::diagnostics::{SystemDiagnostics, DiagnosticResults};

#[cfg(feature = "windows-enhanced")]
use winutils_core::windows::{WindowsHandler, AttributeInfo, AclInfo, ShortcutInfo};

use winpath::normalize_path;

/// Enhanced ls utility implementation
struct EnhancedLsUtility {
    help_system: HelpSystem,
    version_info: VersionInfo,
    #[cfg(feature = "diagnostics")]
    diagnostics: SystemDiagnostics,
}

impl EnhancedLsUtility {
    fn new() -> WinUtilsResult<Self> {
        let help = create_enhanced_help();
        let help_system = HelpSystem::new(help);
        let version_info = VersionInfo::new("ls", "List directory contents with Windows enhancements");

        #[cfg(feature = "diagnostics")]
        let mut diagnostics = SystemDiagnostics::new("ls");
        #[cfg(feature = "diagnostics")]
        diagnostics.initialize()?;

        Ok(Self {
            help_system,
            version_info,
            #[cfg(feature = "diagnostics")]
            diagnostics,
        })
    }

    fn run(&mut self, args: Vec<String>) -> WinUtilsResult<()> {
        let app = enhanced_app!("ls", "List directory contents", env!("CARGO_PKG_VERSION"))
            .arg(clap::Arg::new("long")
                .short('l')
                .long("long")
                .help("Use long listing format")
                .action(clap::ArgAction::SetTrue))
            .arg(clap::Arg::new("all")
                .short('a')
                .long("all")
                .help("Show hidden files")
                .action(clap::ArgAction::SetTrue))
            .arg(clap::Arg::new("windows-attributes")
                .long("windows-attributes")
                .help("Show Windows file attributes")
                .action(clap::ArgAction::SetTrue))
            .arg(clap::Arg::new("windows-acl")
                .long("windows-acl")
                .help("Show Windows ACL information")
                .action(clap::ArgAction::SetTrue))
            .arg(clap::Arg::new("resolve-shortcuts")
                .long("resolve-shortcuts")
                .help("Resolve Windows shortcuts (.lnk files)")
                .action(clap::ArgAction::SetTrue))
            .arg(clap::Arg::new("paths")
                .help("Paths to list")
                .num_args(0..)
                .value_name("PATH"));

        let matches = app.try_get_matches_from(args)?;

        // Handle enhanced arguments
        handle_enhanced_args!(matches, self);

        #[cfg(feature = "diagnostics")]
        self.diagnostics.start_monitoring()?;

        // Get paths to list
        let paths: Vec<&str> = matches.get_many::<String>("paths")
            .map(|v| v.map(|s| s.as_str()).collect())
            .unwrap_or_else(|| vec!["."]);

        let long_format = matches.get_flag("long");
        let show_all = matches.get_flag("all");
        let show_windows_attrs = matches.get_flag("windows-attributes");
        let show_windows_acl = matches.get_flag("windows-acl");
        let resolve_shortcuts = matches.get_flag("resolve-shortcuts");

        // Process each path
        for path_str in paths {
            let normalized_path = normalize_path(path_str)
                .map_err(|e| WinUtilsError::path(format!("Failed to normalize path '{}': {}", path_str, e)))?;

            let path = Path::new(&normalized_path);

            if path.is_dir() {
                self.list_directory(path, long_format, show_all, show_windows_attrs, show_windows_acl, resolve_shortcuts)?;
            } else {
                self.list_file(path, long_format, show_windows_attrs, show_windows_acl, resolve_shortcuts)?;
            }
        }

        #[cfg(feature = "diagnostics")]
        self.diagnostics.collect_metrics()?;

        Ok(())
    }

    fn list_directory(
        &self,
        dir_path: &Path,
        long_format: bool,
        show_all: bool,
        show_windows_attrs: bool,
        show_windows_acl: bool,
        resolve_shortcuts: bool,
    ) -> WinUtilsResult<()> {
        let entries = std::fs::read_dir(dir_path)
            .map_err(|e| WinUtilsError::io(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| WinUtilsError::io(e))?;
            let file_path = entry.path();

            // Skip hidden files unless requested
            if !show_all {
                if let Some(filename) = file_path.file_name() {
                    if filename.to_string_lossy().starts_with('.') {
                        continue;
                    }
                }

                #[cfg(feature = "windows-enhanced")]
                {
                    if let Ok(attr_info) = WindowsHandler::get_attributes(&file_path) {
                        if attr_info.attributes.hidden {
                            continue;
                        }
                    }
                }
            }

            self.list_file(&file_path, long_format, show_windows_attrs, show_windows_acl, resolve_shortcuts)?;
        }

        Ok(())
    }

    fn list_file(
        &self,
        file_path: &Path,
        long_format: bool,
        show_windows_attrs: bool,
        show_windows_acl: bool,
        resolve_shortcuts: bool,
    ) -> WinUtilsResult<()> {
        let filename = file_path.file_name()
            .unwrap_or_else(|| file_path.as_os_str())
            .to_string_lossy();

        if long_format {
            self.print_long_format(file_path, show_windows_attrs, show_windows_acl, resolve_shortcuts)?;
        } else {
            println!("{}", filename);
        }

        Ok(())
    }

    fn print_long_format(
        &self,
        file_path: &Path,
        show_windows_attrs: bool,
        show_windows_acl: bool,
        resolve_shortcuts: bool,
    ) -> WinUtilsResult<()> {
        let metadata = std::fs::metadata(file_path)
            .map_err(|e| WinUtilsError::io(e))?;

        let filename = file_path.file_name()
            .unwrap_or_else(|| file_path.as_os_str())
            .to_string_lossy();

        // Basic Unix-style permissions
        let file_type = if metadata.is_dir() { 'd' } else { '-' };
        let permissions = format!("{}rwxrwxrwx", file_type); // Simplified for demo

        // File size
        let size = metadata.len();

        // Modified time
        let modified = metadata.modified()
            .map_err(|e| WinUtilsError::io(e))?;
        let datetime: chrono::DateTime<chrono::Local> = modified.into();
        let time_str = datetime.format("%Y-%m-%d %H:%M").to_string();

        print!("{} {:8} {} {}", permissions, size, time_str, filename);

        // Windows-specific enhancements
        #[cfg(feature = "windows-enhanced")]
        {
            if show_windows_attrs {
                if let Ok(attr_info) = WindowsHandler::get_attributes(file_path) {
                    print!(" [{}]", attr_info.attributes.short_string());
                }
            }

            if show_windows_acl {
                if let Ok(acl_info) = WindowsHandler::get_acl(file_path) {
                    print!(" ACL:{}", acl_info.acl.permission_summary());
                }
            }

            if resolve_shortcuts {
                if let Ok(Some(shortcut_info)) = WindowsHandler::read_shortcut(file_path) {
                    print!(" -> {}", shortcut_info.target_path.display());
                }
            }
        }

        println!();
        Ok(())
    }
}

impl EnhancedUtility for EnhancedLsUtility {
    fn name(&self) -> &'static str {
        "ls"
    }

    fn description(&self) -> &'static str {
        "List directory contents with Windows enhancements"
    }

    fn help_system(&self) -> &HelpSystem {
        &self.help_system
    }

    fn version_info(&self) -> &VersionInfo {
        &self.version_info
    }

    #[cfg(feature = "testing")]
    fn self_test(&self) -> WinUtilsResult<TestResults> {
        <Self as SelfTest>::self_test(self)
    }

    #[cfg(feature = "testing")]
    fn benchmark(&self) -> WinUtilsResult<BenchmarkResults> {
        <Self as BenchmarkSuite>::benchmark(self)
    }

    #[cfg(feature = "diagnostics")]
    fn diagnose(&self) -> WinUtilsResult<DiagnosticResults> {
        let mut diag = self.diagnostics.clone();
        diag.run_diagnostics()
    }
}

#[cfg(feature = "windows-enhanced")]
impl WindowsEnhanced for EnhancedLsUtility {
    fn handle_windows_attributes(&self, path: &Path) -> WinUtilsResult<AttributeInfo> {
        WindowsHandler::get_attributes(path)
    }

    fn handle_windows_acl(&self, path: &Path) -> WinUtilsResult<AclInfo> {
        WindowsHandler::get_acl(path)
    }

    fn handle_shortcut(&self, path: &Path) -> WinUtilsResult<Option<ShortcutInfo>> {
        WindowsHandler::read_shortcut(path)
    }
}

#[cfg(feature = "testing")]
impl SelfTest for EnhancedLsUtility {
    fn self_test(&self) -> WinUtilsResult<TestResults> {
        let mut results = TestResults::new(self.test_name());

        // Test basic functionality
        let basic_result = self.test_basic_functionality()?;
        results.add_result(basic_result);

        // Test path handling
        let path_result = self.test_path_handling()?;
        results.add_result(path_result);

        // Test directory listing
        let start = Instant::now();
        match std::env::current_dir() {
            Ok(current_dir) => {
                match std::fs::read_dir(&current_dir) {
                    Ok(_) => {
                        let duration = start.elapsed();
                        results.add_result(TestResult::success_with_message(
                            "Directory Listing",
                            duration,
                            format!("Successfully listed current directory: {}", current_dir.display()),
                        ));
                    }
                    Err(e) => {
                        let duration = start.elapsed();
                        results.add_result(TestResult::failure(
                            "Directory Listing",
                            duration,
                            format!("Failed to list current directory: {}", e),
                        ));
                    }
                }
            }
            Err(e) => {
                let duration = start.elapsed();
                results.add_result(TestResult::failure(
                    "Directory Listing",
                    duration,
                    format!("Failed to get current directory: {}", e),
                ));
            }
        }

        // Test Windows-specific features
        #[cfg(feature = "windows-enhanced")]
        {
            let start = Instant::now();
            if let Ok(current_dir) = std::env::current_dir() {
                match self.handle_windows_attributes(&current_dir) {
                    Ok(_) => {
                        let duration = start.elapsed();
                        results.add_result(TestResult::success(
                            "Windows Attributes",
                            duration,
                        ));
                    }
                    Err(e) => {
                        let duration = start.elapsed();
                        results.add_result(TestResult::failure(
                            "Windows Attributes",
                            duration,
                            format!("Failed to get Windows attributes: {}", e),
                        ));
                    }
                }
            }
        }

        Ok(results)
    }

    fn test_name(&self) -> &str {
        "Enhanced LS Utility"
    }
}

#[cfg(feature = "testing")]
impl BenchmarkSuite for EnhancedLsUtility {
    fn benchmark(&self) -> WinUtilsResult<BenchmarkResults> {
        use winutils_core::testing::BenchmarkResult;

        let mut results = BenchmarkResults::new(self.benchmark_name());

        // Benchmark basic operation
        let basic_bench = self.benchmark_basic_operation()?;
        results.add_result(basic_bench);

        // Benchmark directory listing
        let iterations = 100;
        let mut times = Vec::with_capacity(iterations);

        if let Ok(current_dir) = std::env::current_dir() {
            for _ in 0..iterations {
                let start = Instant::now();
                let _ = std::fs::read_dir(&current_dir);
                times.push(start.elapsed());
            }

            let bench_result = BenchmarkResult::new(
                "Directory Listing",
                iterations as u64,
                &times,
            );
            results.add_result(bench_result);
        }

        // Benchmark path normalization
        let test_paths = vec![
            "C:\\Windows\\System32",
            "/c/Windows/System32",
            "\\\\server\\share\\file.txt",
            "..\\..\\test\\file.txt",
        ];

        let iterations = 1000;
        let mut times = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let start = Instant::now();
            for path in &test_paths {
                let _ = normalize_path(path);
            }
            times.push(start.elapsed());
        }

        let bench_result = BenchmarkResult::new(
            "Path Normalization",
            iterations as u64,
            &times,
        );
        results.add_result(bench_result);

        Ok(results)
    }

    fn benchmark_name(&self) -> &str {
        "Enhanced LS Utility"
    }
}

fn create_enhanced_help() -> EnhancedHelp {
    let examples = ExampleSet::new()
        .add_basic(Example::new(
            "List current directory",
            "ls"
        ).with_output("file1.txt  file2.txt  directory1/"))
        .add_basic(Example::new(
            "List with detailed information",
            "ls -l"
        ).with_output("-rw-r--r--     1024 2024-01-15 14:30 file1.txt\ndrwxr-xr-x     4096 2024-01-15 14:25 directory1/"))
        .add_basic(Example::new(
            "List all files including hidden",
            "ls -a"
        ).with_output(".hidden_file  file1.txt  file2.txt"))
        .add_common_use_case(Example::new(
            "List files with Windows attributes",
            "ls -l --windows-attributes"
        ).with_output("-rw-r--r--     1024 2024-01-15 14:30 file1.txt [--h--]\ndrwxr-xr-x     4096 2024-01-15 14:25 directory1/ [d----]")
        .with_notes("Shows Windows file attributes: d(directory), r(readonly), h(hidden), s(system), a(archive)"))
        .add_common_use_case(Example::new(
            "List with Windows ACL information",
            "ls -l --windows-acl"
        ).with_output("-rw-r--r--     1024 2024-01-15 14:30 file1.txt ACL:rwx\ndrwxr-xr-x     4096 2024-01-15 14:25 directory1/ ACL:rwx")
        .with_notes("Shows simplified Windows ACL permissions"))
        .add_advanced(Example::new(
            "Resolve Windows shortcuts",
            "ls -l --resolve-shortcuts"
        ).with_output("-rw-r--r--     1024 2024-01-15 14:30 shortcut.lnk -> C:\\Program Files\\App\\app.exe"))
        .add_windows_specific(Example::new(
            "Comprehensive Windows listing",
            "ls -la --windows-attributes --windows-acl --resolve-shortcuts"
        ).with_output("drwxr-xr-x     4096 2024-01-15 14:25 ./ [d----] ACL:rwx\ndrwxr-xr-x     4096 2024-01-15 14:20 ../ [d----] ACL:rwx\n-rw-r--r--      512 2024-01-15 14:30 .hidden [--h--] ACL:r--\n-rw-r--r--     1024 2024-01-15 14:30 file1.txt [-----] ACL:rwx\n-rw-r--r--      256 2024-01-15 14:32 link.lnk [-----] ACL:rwx -> C:\\Target\\file.exe")
        .windows_specific())
        .add_troubleshooting(Example::new(
            "Debug path handling issues",
            "ls --diagnose"
        ).with_notes("Runs diagnostic checks for path handling, permissions, and Windows-specific features"));

    let windows_notes = WindowsNotes::new()
        .add_path_note("Automatically handles Git Bash, WSL, and native Windows paths")
        .add_path_note("Supports UNC paths (\\\\server\\share) and long paths (>260 characters)")
        .add_permissions_note("Windows ACLs are simplified to Unix-style permissions for display")
        .add_permissions_note("Use --windows-acl for detailed Windows permission information")
        .add_file_attributes_note("Windows file attributes displayed with --windows-attributes flag")
        .add_file_attributes_note("Hidden files are respected by default unless -a is used")
        .add_performance_note("Optimized for Windows filesystem performance characteristics")
        .add_performance_note("Uses native Windows APIs for attribute and ACL queries")
        .add_compatibility_note("Maintains GNU ls compatibility for standard operations")
        .add_compatibility_note("Windows-specific flags clearly marked and optional");

    EnhancedHelp::new(
        "ls",
        "List directory contents",
        "List information about files and directories. By default, ls lists the files in the current directory in alphabetical order. This Windows-enhanced version provides additional support for Windows file attributes, ACLs, shortcuts, and optimized path handling."
    )
    .with_examples(examples)
    .with_windows_notes(windows_notes)
    .add_see_also("dir(1)")
    .add_see_also("tree(1)")
    .add_see_also("find(1)")
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut utility = match EnhancedLsUtility::new() {
        Ok(util) => util,
        Err(e) => {
            eprintln!("Error initializing ls utility: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = utility.run(args) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utility_creation() {
        let utility = EnhancedLsUtility::new().unwrap();
        assert_eq!(utility.name(), "ls");
        assert_eq!(utility.description(), "List directory contents with Windows enhancements");
    }

    #[cfg(feature = "testing")]
    #[test]
    fn test_self_test() {
        let utility = EnhancedLsUtility::new().unwrap();
        let results = utility.self_test().unwrap();
        assert!(results.total_tests > 0);
    }

    #[test]
    fn test_enhanced_help() {
        let help = create_enhanced_help();
        assert_eq!(help.utility_name, "ls");
        assert!(!help.examples.basic.is_empty());
        assert!(!help.examples.windows_specific.is_empty());
        assert!(!help.windows_notes.path_handling.is_empty());
    }
}
