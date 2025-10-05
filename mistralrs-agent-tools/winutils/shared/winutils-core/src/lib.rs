//! # WinUtils Core
//!
//! Enhanced core features for Windows utilities including:
//! - Unified help system with examples and Windows-specific documentation
//! - Version and source identification
//! - Built-in testing framework
//! - Windows-specific enhancements (ACLs, attributes, shortcuts)
//! - Diagnostics and troubleshooting tools
//!
//! ## Features
//!
//! - `help` - Enhanced help system with examples and documentation
//! - `version` - Version and source identification system
//! - `testing` - Built-in testing framework and benchmarks
//! - `windows-enhanced` - Windows-specific functionality (ACLs, attributes, shortcuts)
//! - `diagnostics` - System diagnostics and troubleshooting
//! - `man-pages` - Man page generation support

pub mod help;
pub mod version;
pub mod testing;
pub mod windows;
pub mod diagnostics;
pub mod error;

// Re-export commonly used items
pub use error::{WinUtilsError, WinUtilsResult};
pub use help::{HelpSystem, EnhancedHelp, ExampleSet};
pub use version::{VersionInfo, BuildInfo, FeatureInfo};

#[cfg(feature = "testing")]
pub use testing::{SelfTest, BenchmarkSuite, DiagnosticMode};

#[cfg(feature = "windows-enhanced")]
pub use windows::{WindowsAttributes, WindowsAcl, ShortcutInfo};

#[cfg(feature = "diagnostics")]
pub use diagnostics::{SystemDiagnostics, PerformanceMonitor};

/// Common traits that utilities should implement
pub mod traits {
    use crate::{WinUtilsResult, HelpSystem, VersionInfo};

    /// Core functionality that all enhanced utilities should provide
    pub trait EnhancedUtility {
        /// Get the utility name
        fn name(&self) -> &'static str;

        /// Get the utility description
        fn description(&self) -> &'static str;

        /// Get enhanced help information
        fn help_system(&self) -> &HelpSystem;

        /// Get version information
        fn version_info(&self) -> &VersionInfo;

        /// Run self-tests
        #[cfg(feature = "testing")]
        fn self_test(&self) -> WinUtilsResult<crate::testing::TestResults>;

        /// Run benchmarks
        #[cfg(feature = "testing")]
        fn benchmark(&self) -> WinUtilsResult<crate::testing::BenchmarkResults>;

        /// Run diagnostics
        #[cfg(feature = "diagnostics")]
        fn diagnose(&self) -> WinUtilsResult<crate::testing::DiagnosticResults>;
    }

    /// Trait for utilities that support Windows-specific features
    #[cfg(feature = "windows-enhanced")]
    pub trait WindowsEnhanced {
        /// Handle Windows file attributes
        fn handle_windows_attributes(&self, path: &std::path::Path) -> WinUtilsResult<crate::windows::AttributeInfo>;

        /// Handle Windows ACLs
        fn handle_windows_acl(&self, path: &std::path::Path) -> WinUtilsResult<crate::windows::AclInfo>;

        /// Handle Windows shortcuts
        fn handle_shortcut(&self, path: &std::path::Path) -> WinUtilsResult<Option<crate::windows::ShortcutInfo>>;
    }
}

/// Utility macros for common functionality
pub mod macros {
    /// Create a standard argument parser with enhanced features
    #[macro_export]
    macro_rules! enhanced_app {
        ($name:expr, $description:expr, $version:expr) => {
            clap::Command::new($name)
                .about($description)
                .version($version)
                .arg(clap::Arg::new("help-full")
                    .long("help-full")
                    .help("Show comprehensive help with examples")
                    .action(clap::ArgAction::SetTrue))
                .arg(clap::Arg::new("version-full")
                    .long("version-full")
                    .help("Show detailed version information")
                    .action(clap::ArgAction::SetTrue))
                .arg(clap::Arg::new("source")
                    .long("source")
                    .help("Show source repository information")
                    .action(clap::ArgAction::SetTrue))
                .arg(clap::Arg::new("features")
                    .long("features")
                    .help("List compiled features")
                    .action(clap::ArgAction::SetTrue))
                .arg(clap::Arg::new("self-test")
                    .long("self-test")
                    .help("Run internal validation tests")
                    .action(clap::ArgAction::SetTrue))
                .arg(clap::Arg::new("benchmark")
                    .long("benchmark")
                    .help("Run performance benchmarks")
                    .action(clap::ArgAction::SetTrue))
                .arg(clap::Arg::new("diagnose")
                    .long("diagnose")
                    .help("Run diagnostic checks")
                    .action(clap::ArgAction::SetTrue))
                .arg(clap::Arg::new("check-updates")
                    .long("check-updates")
                    .help("Check for available updates")
                    .action(clap::ArgAction::SetTrue))
        };
    }

    /// Handle enhanced arguments in utility main functions
    #[macro_export]
    macro_rules! handle_enhanced_args {
        ($matches:expr, $utility:expr) => {
            if $matches.get_flag("help-full") {
                $utility.help_system().show_full_help()?;
                return Ok(());
            }

            if $matches.get_flag("version-full") {
                $utility.version_info().show_detailed()?;
                return Ok(());
            }

            if $matches.get_flag("source") {
                $utility.version_info().show_source_info()?;
                return Ok(());
            }

            if $matches.get_flag("features") {
                $utility.version_info().show_features()?;
                return Ok(());
            }

            #[cfg(feature = "testing")]
            {
                if $matches.get_flag("self-test") {
                    let results = $utility.self_test()?;
                    results.display()?;
                    return Ok(());
                }

                if $matches.get_flag("benchmark") {
                    let results = $utility.benchmark()?;
                    results.display()?;
                    return Ok(());
                }
            }

            #[cfg(feature = "diagnostics")]
            {
                if $matches.get_flag("diagnose") {
                    let results = $utility.diagnose()?;
                    results.display()?;
                    return Ok(());
                }
            }

            if $matches.get_flag("check-updates") {
                $utility.version_info().check_updates().await?;
                return Ok(());
            }
        };
    }
}

/// Common constants used across utilities
pub mod constants {
    /// Current version of the winutils enhancement framework
    pub const WINUTILS_CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

    /// Repository URL
    pub const REPOSITORY_URL: &str = "https://github.com/uutils/coreutils";

    /// Issues URL
    pub const ISSUES_URL: &str = "https://github.com/uutils/coreutils/issues";

    /// Documentation URL
    pub const DOCS_URL: &str = "https://docs.rs/winutils-core";

    /// Default cache directory for diagnostics and temporary files
    pub const CACHE_DIR_NAME: &str = ".winutils";

    /// Configuration file name
    pub const CONFIG_FILE_NAME: &str = "winutils.toml";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert!(!constants::WINUTILS_CORE_VERSION.is_empty());
        assert!(constants::REPOSITORY_URL.starts_with("https://"));
        assert!(constants::DOCS_URL.starts_with("https://"));
    }
}
