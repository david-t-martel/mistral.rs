//! Enhanced Utility Demo
//!
//! This example demonstrates how to create a utility that uses all the
//! enhanced features provided by winutils-core:
//!
//! 1. Enhanced Help System with examples and Windows-specific notes
//! 2. Version and Source Identification with detailed build info
//! 3. Built-in Testing Framework with self-tests and benchmarks
//! 4. Windows-specific Enhancements (ACL, attributes, shortcuts)
//! 5. Diagnostics and Performance Monitoring
//!
//! To run this demo:
//! ```
//! cargo run --example enhanced_utility_demo -- --help-full
//! cargo run --example enhanced_utility_demo -- --version-full
//! cargo run --example enhanced_utility_demo -- --self-test
//! cargo run --example enhanced_utility_demo -- --benchmark
//! cargo run --example enhanced_utility_demo -- --diagnose
//! ```

use std::env;
use std::path::Path;
use std::time::Instant;

use winutils_core::{
    enhanced_app, handle_enhanced_args,
    HelpSystem, EnhancedHelp, ExampleSet, Example, WindowsNotes,
    VersionInfo, WinUtilsResult, WinUtilsError,
    traits::{EnhancedUtility, WindowsEnhanced},
};

#[cfg(feature = "testing")]
use winutils_core::testing::{SelfTest, TestResults, TestResult, BenchmarkSuite, BenchmarkResults, BenchmarkResult};

#[cfg(feature = "diagnostics")]
use winutils_core::diagnostics::{SystemDiagnostics, DiagnosticResults};

#[cfg(feature = "windows-enhanced")]
use winutils_core::windows::{WindowsHandler, AttributeInfo, AclInfo, ShortcutInfo};

/// Demo utility that showcases all enhanced features
struct DemoUtility {
    help_system: HelpSystem,
    version_info: VersionInfo,
    #[cfg(feature = "diagnostics")]
    diagnostics: SystemDiagnostics,
}

impl DemoUtility {
    fn new() -> WinUtilsResult<Self> {
        println!("🚀 Initializing Enhanced Demo Utility...");

        let help = create_demo_help();
        let help_system = HelpSystem::new(help);
        let version_info = VersionInfo::new("demo-util", "Demonstration of winutils-core enhanced features");

        #[cfg(feature = "diagnostics")]
        let mut diagnostics = SystemDiagnostics::new("demo-util");
        #[cfg(feature = "diagnostics")]
        {
            println!("🔧 Initializing system diagnostics...");
            diagnostics.initialize()?;
        }

        println!("✅ Demo utility initialized successfully!");

        Ok(Self {
            help_system,
            version_info,
            #[cfg(feature = "diagnostics")]
            diagnostics,
        })
    }

    fn run(&mut self, args: Vec<String>) -> WinUtilsResult<()> {
        let app = enhanced_app!("demo-util", "Enhanced utility demonstration", env!("CARGO_PKG_VERSION"))
            .arg(clap::Arg::new("operation")
                .help("Operation to perform")
                .value_parser(["info", "test-features", "performance-demo"])
                .default_value("info"))
            .arg(clap::Arg::new("path")
                .short('p')
                .long("path")
                .help("Path to analyze (for Windows features demo)")
                .value_name("PATH")
                .default_value("."));

        let matches = app.try_get_matches_from(args)?;

        // Handle enhanced arguments first
        handle_enhanced_args!(matches, self);

        let operation = matches.get_one::<String>("operation").unwrap();
        let path = matches.get_one::<String>("path").unwrap();

        #[cfg(feature = "diagnostics")]
        self.diagnostics.start_monitoring()?;

        match operation.as_str() {
            "info" => self.show_info(),
            "test-features" => self.test_features(path),
            "performance-demo" => self.performance_demo(),
            _ => Err(WinUtilsError::generic("main", format!("Unknown operation: {}", operation))),
        }?;

        #[cfg(feature = "diagnostics")]
        self.diagnostics.collect_metrics()?;

        Ok(())
    }

    fn show_info(&self) -> WinUtilsResult<()> {
        println!("📋 Enhanced Utility Information");
        println!("================================");
        println!();
        println!("This demo utility showcases the enhanced features provided by winutils-core:");
        println!();
        println!("🔹 Enhanced Help System:");
        println!("   • Comprehensive help with examples and use cases");
        println!("   • Windows-specific documentation and notes");
        println!("   • Man page generation support");
        println!();
        println!("🔹 Version and Source Management:");
        println!("   • Detailed build information with git integration");
        println!("   • Feature detection and listing");
        println!("   • Update checking capabilities");
        println!();
        println!("🔹 Built-in Testing Framework:");
        println!("   • Self-validation tests");
        println!("   • Performance benchmarks");
        println!("   • Integration test harness");
        println!();
        println!("🔹 Windows-specific Enhancements:");
        println!("   • File attributes (Hidden, System, Archive, ReadOnly)");
        println!("   • ACL (Access Control List) support");
        println!("   • Windows shortcuts (.lnk files) handling");
        println!("   • Registry integration");
        println!();
        println!("🔹 System Diagnostics:");
        println!("   • Performance monitoring");
        println!("   • System information collection");
        println!("   • Troubleshooting assistance");
        println!();
        println!("Try the following commands to explore these features:");
        println!("  demo-util --help-full          # Comprehensive help");
        println!("  demo-util --version-full       # Detailed version info");
        println!("  demo-util --features           # List compiled features");
        println!("  demo-util --self-test          # Run self-tests");
        println!("  demo-util --benchmark          # Performance benchmarks");
        println!("  demo-util --diagnose           # System diagnostics");
        println!("  demo-util test-features         # Test Windows features");
        println!("  demo-util performance-demo     # Performance monitoring demo");

        Ok(())
    }

    fn test_features(&self, path_str: &str) -> WinUtilsResult<()> {
        println!("🧪 Testing Windows-specific Features");
        println!("====================================");
        println!();

        let path = Path::new(path_str);
        if !path.exists() {
            return Err(WinUtilsError::path(format!("Path does not exist: {}", path_str)));
        }

        println!("Testing path: {}", path.display());
        println!();

        // Test Windows attributes
        #[cfg(feature = "windows-enhanced")]
        {
            println!("🔍 Windows File Attributes:");
            match WindowsHandler::get_attributes(path) {
                Ok(attr_info) => {
                    if attr_info.error.is_some() {
                        println!("  ❌ Error: {}", attr_info.error.as_ref().unwrap());
                    } else {
                        println!("  ✅ Attributes: {}", attr_info.attributes.description());
                        println!("     Short form: {}", attr_info.attributes.short_string());
                        println!("     Size: {} bytes", attr_info.size);
                        println!("     Created: {:?}", attr_info.created);
                        println!("     Modified: {:?}", attr_info.modified);
                        println!("     Accessed: {:?}", attr_info.accessed);
                    }
                }
                Err(e) => println!("  ❌ Failed to get attributes: {}", e),
            }
            println!();

            println!("🔐 Windows ACL Information:");
            match WindowsHandler::get_acl(path) {
                Ok(acl_info) => {
                    if acl_info.error.is_some() {
                        println!("  ❌ Error: {}", acl_info.error.as_ref().unwrap());
                    } else {
                        println!("  ✅ ACL retrieved successfully");
                        println!("     Owner: {:?}", acl_info.acl.owner);
                        println!("     Group: {:?}", acl_info.acl.group);
                        println!("     ACEs: {} entries", acl_info.acl.aces.len());
                        println!("     Permission summary: {}", acl_info.acl.permission_summary());
                    }
                }
                Err(e) => println!("  ❌ Failed to get ACL: {}", e),
            }
            println!();

            println!("🔗 Windows Shortcut Detection:");
            match WindowsHandler::read_shortcut(path) {
                Ok(Some(shortcut_info)) => {
                    println!("  ✅ Shortcut detected!");
                    println!("     Target: {}", shortcut_info.target_path.display());
                    if let Some(ref working_dir) = shortcut_info.working_directory {
                        println!("     Working directory: {}", working_dir.display());
                    }
                    if let Some(ref args) = shortcut_info.arguments {
                        println!("     Arguments: {}", args);
                    }
                    if let Some(ref desc) = shortcut_info.description {
                        println!("     Description: {}", desc);
                    }
                }
                Ok(None) => println!("  ℹ️  Not a Windows shortcut"),
                Err(e) => println!("  ❌ Failed to read shortcut: {}", e),
            }
        }

        #[cfg(not(feature = "windows-enhanced"))]
        {
            println!("  ⚠️  Windows-enhanced features not compiled in");
            println!("     Rebuild with --features windows-enhanced to enable");
        }

        Ok(())
    }

    fn performance_demo(&mut self) -> WinUtilsResult<()> {
        println!("⚡ Performance Monitoring Demonstration");
        println!("======================================");
        println!();

        #[cfg(feature = "diagnostics")]
        {
            println!("🔄 Starting performance monitoring...");

            // Simulate some work
            let start = Instant::now();

            // CPU-intensive task
            let mut sum = 0u64;
            for i in 0..1_000_000 {
                sum = sum.wrapping_add(i);
            }

            // I/O task
            let _ = std::fs::read_dir(".");

            // Memory allocation task
            let _large_vec: Vec<u64> = (0..100_000).collect();

            let work_duration = start.elapsed();
            println!("✅ Simulated work completed in {:.3}s", work_duration.as_secs_f64());

            // Collect metrics
            self.diagnostics.collect_metrics()?;

            // Generate report
            if let Some(report) = self.diagnostics.generate_performance_report() {
                println!();
                report.display()?;
            }
        }

        #[cfg(not(feature = "diagnostics"))]
        {
            println!("  ⚠️  Diagnostics feature not compiled in");
            println!("     Rebuild with --features diagnostics to enable");
        }

        Ok(())
    }
}

impl EnhancedUtility for DemoUtility {
    fn name(&self) -> &'static str {
        "demo-util"
    }

    fn description(&self) -> &'static str {
        "Demonstration utility for winutils-core enhanced features"
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
impl WindowsEnhanced for DemoUtility {
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
impl SelfTest for DemoUtility {
    fn self_test(&self) -> WinUtilsResult<TestResults> {
        let mut results = TestResults::new(self.test_name());

        println!("🧪 Running Self-Tests...");

        // Test basic functionality
        let basic_result = self.test_basic_functionality()?;
        results.add_result(basic_result);

        // Test path handling
        let path_result = self.test_path_handling()?;
        results.add_result(path_result);

        // Test version system
        let start = Instant::now();
        let version_str = self.version_info().build_info.short_version();
        let duration = start.elapsed();
        if !version_str.is_empty() {
            results.add_result(TestResult::success_with_message(
                "Version System",
                duration,
                format!("Version: {}", version_str),
            ));
        } else {
            results.add_result(TestResult::failure(
                "Version System",
                duration,
                "Empty version string",
            ));
        }

        // Test help system
        let start = Instant::now();
        let help_name = &self.help_system().help.utility_name;
        let duration = start.elapsed();
        if help_name == "demo-util" {
            results.add_result(TestResult::success(
                "Help System",
                duration,
            ));
        } else {
            results.add_result(TestResult::failure(
                "Help System",
                duration,
                format!("Incorrect utility name: {}", help_name),
            ));
        }

        // Test Windows features (if available)
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
                            format!("Failed: {}", e),
                        ));
                    }
                }
            }
        }

        println!("✅ Self-tests completed");
        Ok(results)
    }

    fn test_name(&self) -> &str {
        "Enhanced Demo Utility"
    }
}

#[cfg(feature = "testing")]
impl BenchmarkSuite for DemoUtility {
    fn benchmark(&self) -> WinUtilsResult<BenchmarkResults> {
        let mut results = BenchmarkResults::new(self.benchmark_name());

        println!("⚡ Running Performance Benchmarks...");

        // Basic operation benchmark
        let basic_bench = self.benchmark_basic_operation()?;
        results.add_result(basic_bench);

        // Version string generation benchmark
        let iterations = 10_000;
        let mut times = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let start = Instant::now();
            let _ = self.version_info().build_info.short_version();
            times.push(start.elapsed());
        }

        let version_bench = BenchmarkResult::new(
            "Version String Generation",
            iterations as u64,
            &times,
        );
        results.add_result(version_bench);

        // Help system access benchmark
        let iterations = 1_000;
        let mut times = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let start = Instant::now();
            let _ = &self.help_system().help.utility_name;
            times.push(start.elapsed());
        }

        let help_bench = BenchmarkResult::new(
            "Help System Access",
            iterations as u64,
            &times,
        );
        results.add_result(help_bench);

        println!("✅ Benchmarks completed");
        Ok(results)
    }

    fn benchmark_name(&self) -> &str {
        "Enhanced Demo Utility"
    }
}

fn create_demo_help() -> EnhancedHelp {
    let examples = ExampleSet::new()
        .add_basic(Example::new(
            "Show utility information",
            "demo-util info"
        ).with_output("Enhanced Utility Information\n================================"))
        .add_basic(Example::new(
            "Test Windows features on current directory",
            "demo-util test-features"
        ).with_output("Testing Windows-specific Features\n===================================="))
        .add_basic(Example::new(
            "Run performance demonstration",
            "demo-util performance-demo"
        ).with_output("Performance Monitoring Demonstration\n======================================"))
        .add_common_use_case(Example::new(
            "Analyze specific file or directory",
            "demo-util test-features --path C:\\Windows\\System32"
        ).with_notes("Tests Windows features on the specified path"))
        .add_advanced(Example::new(
            "Run comprehensive diagnostics",
            "demo-util --diagnose"
        ).with_output("DIAGNOSTIC RESULTS: DEMO-UTIL\n✓ System Information: Windows 11 (Build 22000)\n✓ Memory Usage: Memory usage is normal: 45.2%")
        .with_notes("Performs system health checks and diagnostics"))
        .add_windows_specific(Example::new(
            "View all enhanced features",
            "demo-util --help-full"
        ).with_notes("Shows comprehensive help including all Windows-specific features")
        .windows_specific())
        .add_troubleshooting(Example::new(
            "Debug feature availability",
            "demo-util --features"
        ).with_output("ENABLED FEATURES\n✓ Enhanced Help System - Comprehensive help with examples\n✓ Windows Enhancements - ACL support, file attributes")
        .with_notes("Shows which features are compiled in and available"));

    let windows_notes = WindowsNotes::new()
        .add_path_note("Supports all Windows path formats including UNC and long paths")
        .add_path_note("Integrates with winpath for Git Bash compatibility")
        .add_permissions_note("Windows ACLs provide detailed permission information")
        .add_permissions_note("Registry integration available for system information")
        .add_file_attributes_note("Full support for Hidden, System, Archive, and ReadOnly attributes")
        .add_file_attributes_note("Windows shortcuts (.lnk files) can be resolved to their targets")
        .add_performance_note("Optimized for Windows performance characteristics")
        .add_performance_note("System diagnostics include Windows-specific metrics")
        .add_compatibility_note("All features are optional and gracefully degrade on other platforms")
        .add_compatibility_note("Enhanced features can be disabled at compile time if not needed");

    EnhancedHelp::new(
        "demo-util",
        "Enhanced utility demonstration",
        "This utility demonstrates the comprehensive enhanced features provided by winutils-core. It showcases the enhanced help system, version management, built-in testing framework, Windows-specific functionality, and system diagnostics capabilities that can be integrated into any Windows utility."
    )
    .with_examples(examples)
    .with_windows_notes(windows_notes)
    .add_see_also("winutils-core documentation")
    .add_see_also("ls(1) - Example enhanced utility")
    .add_custom_section("Integration", vec![
        "To integrate these features into your utility:".to_string(),
        "1. Add winutils-core to your Cargo.toml dependencies".to_string(),
        "2. Implement the EnhancedUtility trait".to_string(),
        "3. Use the enhanced_app! and handle_enhanced_args! macros".to_string(),
        "4. Optionally implement WindowsEnhanced, SelfTest, and BenchmarkSuite traits".to_string(),
    ])
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut utility = match DemoUtility::new() {
        Ok(util) => util,
        Err(e) => {
            eprintln!("❌ Error initializing demo utility: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = utility.run(args) {
        eprintln!("❌ Error: {}", e);
        std::process::exit(1);
    }
}
