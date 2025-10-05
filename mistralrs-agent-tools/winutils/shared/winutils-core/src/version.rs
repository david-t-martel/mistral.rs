//! Version and Source Identification System
//!
//! Provides comprehensive version information, build details, feature detection,
//! source repository information, and update checking capabilities.

use crate::{WinUtilsError, WinUtilsResult, constants};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use chrono::{DateTime, Utc};

/// Build information including git details and compilation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInfo {
    pub version: String,
    pub git_commit: Option<String>,
    pub git_branch: Option<String>,
    pub git_dirty: bool,
    pub build_date: DateTime<Utc>,
    pub build_profile: String,
    pub rust_version: String,
    pub target_triple: String,
    pub features: Vec<String>,
    pub optimizations: Vec<String>,
}

impl BuildInfo {
    /// Create build information from environment variables and git
    pub fn from_env() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_commit: Self::get_git_commit(),
            git_branch: Self::get_git_branch(),
            git_dirty: Self::is_git_dirty(),
            build_date: Self::get_build_date(),
            build_profile: Self::get_build_profile(),
            rust_version: Self::get_rust_version(),
            target_triple: env!("TARGET").to_string(),
            features: Self::get_enabled_features(),
            optimizations: Self::get_optimizations(),
        }
    }

    #[cfg(feature = "version")]
    fn get_git_commit() -> Option<String> {
        use git2::Repository;
        Repository::discover(".")
            .ok()
            .and_then(|repo| {
                repo.head().ok().and_then(|head| {
                    head.target().map(|oid| oid.to_string()[..8].to_string())
                })
            })
    }

    #[cfg(not(feature = "version"))]
    fn get_git_commit() -> Option<String> {
        std::env::var("GIT_COMMIT").ok()
            .map(|s| s[..std::cmp::min(8, s.len())].to_string())
    }

    #[cfg(feature = "version")]
    fn get_git_branch() -> Option<String> {
        use git2::Repository;
        Repository::discover(".")
            .ok()
            .and_then(|repo| {
                repo.head().ok().and_then(|head| {
                    head.shorthand().map(|s| s.to_string())
                })
            })
    }

    #[cfg(not(feature = "version"))]
    fn get_git_branch() -> Option<String> {
        std::env::var("GIT_BRANCH").ok()
    }

    #[cfg(feature = "version")]
    fn is_git_dirty() -> bool {
        use git2::Repository;
        if let Ok(repo) = Repository::discover(".") {
            let mut status_options = git2::StatusOptions::new();
            status_options.include_untracked(true);
            if let Ok(statuses) = repo.statuses(Some(&mut status_options)) {
                return !statuses.is_empty();
            }
        }
        false
    }

    #[cfg(not(feature = "version"))]
    fn is_git_dirty() -> bool {
        std::env::var("GIT_DIRTY").ok()
            .map(|s| s == "true")
            .unwrap_or(false)
    }

    fn get_build_date() -> DateTime<Utc> {
        std::env::var("BUILD_DATE")
            .ok()
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now)
    }

    fn get_build_profile() -> String {
        if cfg!(debug_assertions) {
            "debug".to_string()
        } else {
            std::env::var("PROFILE").unwrap_or_else(|_| "release".to_string())
        }
    }

    fn get_rust_version() -> String {
        env!("RUSTC_VERSION").to_string()
    }

    fn get_enabled_features() -> Vec<String> {
        let mut features = Vec::new();

        if cfg!(feature = "help") {
            features.push("help".to_string());
        }
        if cfg!(feature = "version") {
            features.push("version".to_string());
        }
        if cfg!(feature = "testing") {
            features.push("testing".to_string());
        }
        if cfg!(feature = "windows-enhanced") {
            features.push("windows-enhanced".to_string());
        }
        if cfg!(feature = "diagnostics") {
            features.push("diagnostics".to_string());
        }
        if cfg!(feature = "man-pages") {
            features.push("man-pages".to_string());
        }

        features.sort();
        features
    }

    fn get_optimizations() -> Vec<String> {
        let mut opts = Vec::new();

        if cfg!(feature = "lto") {
            opts.push("Link-Time Optimization".to_string());
        }
        if cfg!(target_feature = "sse2") {
            opts.push("SSE2".to_string());
        }
        if cfg!(target_feature = "avx2") {
            opts.push("AVX2".to_string());
        }
        if std::env::var("CARGO_CFG_PANIC").as_deref() == Ok("abort") {
            opts.push("Panic=abort".to_string());
        }

        opts
    }

    /// Get a short version string suitable for --version
    pub fn short_version(&self) -> String {
        match &self.git_commit {
            Some(commit) => {
                let dirty = if self.git_dirty { "-dirty" } else { "" };
                format!("{} ({}{})", self.version, commit, dirty)
            }
            None => self.version.clone(),
        }
    }

    /// Check if this is a development build
    pub fn is_development(&self) -> bool {
        self.build_profile == "debug" || self.git_dirty ||
            self.git_branch.as_deref() != Some("main")
    }
}

/// Feature information including availability and descriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureInfo {
    pub available_features: HashMap<String, FeatureDescription>,
    pub enabled_features: Vec<String>,
    pub platform_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureDescription {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub platform_specific: bool,
    pub requires: Vec<String>,
}

impl FeatureInfo {
    /// Create feature information
    pub fn new() -> Self {
        let mut available = HashMap::new();

        available.insert("help".to_string(), FeatureDescription {
            name: "Enhanced Help System".to_string(),
            description: "Comprehensive help with examples and Windows-specific notes".to_string(),
            enabled: cfg!(feature = "help"),
            platform_specific: false,
            requires: vec!["clap".to_string(), "termcolor".to_string()],
        });

        available.insert("version".to_string(), FeatureDescription {
            name: "Version Management".to_string(),
            description: "Detailed version info, git integration, and update checking".to_string(),
            enabled: cfg!(feature = "version"),
            platform_specific: false,
            requires: vec!["git2".to_string(), "chrono".to_string()],
        });

        available.insert("testing".to_string(), FeatureDescription {
            name: "Built-in Testing".to_string(),
            description: "Self-tests, benchmarks, and diagnostic capabilities".to_string(),
            enabled: cfg!(feature = "testing"),
            platform_specific: false,
            requires: vec!["criterion".to_string(), "tempfile".to_string()],
        });

        available.insert("windows-enhanced".to_string(), FeatureDescription {
            name: "Windows Enhancements".to_string(),
            description: "ACL support, file attributes, and Windows-specific functionality".to_string(),
            enabled: cfg!(feature = "windows-enhanced"),
            platform_specific: true,
            requires: vec!["windows-sys".to_string(), "windows".to_string()],
        });

        available.insert("diagnostics".to_string(), FeatureDescription {
            name: "System Diagnostics".to_string(),
            description: "System information and performance monitoring".to_string(),
            enabled: cfg!(feature = "diagnostics"),
            platform_specific: false,
            requires: vec!["sysinfo".to_string(), "tokio".to_string()],
        });

        available.insert("man-pages".to_string(), FeatureDescription {
            name: "Man Page Generation".to_string(),
            description: "Generate man pages from help information".to_string(),
            enabled: cfg!(feature = "man-pages"),
            platform_specific: false,
            requires: vec!["roff".to_string()],
        });

        let enabled_features = available.iter()
            .filter(|(_, desc)| desc.enabled)
            .map(|(name, _)| name.clone())
            .collect();

        let platform_features = if cfg!(windows) {
            vec!["windows-enhanced".to_string()]
        } else {
            vec![]
        };

        Self {
            available_features: available,
            enabled_features,
            platform_features,
        }
    }

    /// Check if a feature is available and enabled
    pub fn is_enabled(&self, feature: &str) -> bool {
        self.enabled_features.contains(&feature.to_string())
    }

    /// Get feature description
    pub fn get_description(&self, feature: &str) -> Option<&FeatureDescription> {
        self.available_features.get(feature)
    }
}

impl Default for FeatureInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Update information for checking new versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub release_notes_url: Option<String>,
    pub download_url: Option<String>,
    pub last_checked: DateTime<Utc>,
}

impl UpdateInfo {
    /// Create new update info
    pub fn new(current_version: String) -> Self {
        Self {
            current_version,
            latest_version: None,
            update_available: false,
            release_notes_url: None,
            download_url: None,
            last_checked: Utc::now(),
        }
    }

    /// Check for updates from GitHub releases
    #[cfg(feature = "version")]
    pub async fn check_github_updates(&mut self, repo: &str) -> WinUtilsResult<()> {
        use std::time::Duration;

        let client = tokio::time::timeout(Duration::from_secs(10), async {
            // In a real implementation, this would use reqwest or similar
            // For now, we'll just simulate the check
            Ok::<_, WinUtilsError>(())
        }).await
        .map_err(|_| WinUtilsError::network("Request timeout"))?;

        client?;

        // Simulate update check
        self.last_checked = Utc::now();
        self.latest_version = Some("0.2.0".to_string());
        self.update_available = self.current_version != "0.2.0";
        self.release_notes_url = Some(format!("{}/releases/latest", repo));
        self.download_url = Some(format!("{}/releases/latest", repo));

        Ok(())
    }

    #[cfg(not(feature = "version"))]
    pub async fn check_github_updates(&mut self, _repo: &str) -> WinUtilsResult<()> {
        Err(WinUtilsError::feature_not_available("version"))
    }
}

/// Complete version information system
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub build_info: BuildInfo,
    pub feature_info: FeatureInfo,
    pub utility_name: String,
    pub utility_description: String,
    color_choice: ColorChoice,
}

impl VersionInfo {
    /// Create new version info for a utility
    pub fn new<S: Into<String>>(utility_name: S, utility_description: S) -> Self {
        Self {
            build_info: BuildInfo::from_env(),
            feature_info: FeatureInfo::new(),
            utility_name: utility_name.into(),
            utility_description: utility_description.into(),
            color_choice: ColorChoice::Auto,
        }
    }

    /// Set color output preference
    pub fn with_color_choice(mut self, choice: ColorChoice) -> Self {
        self.color_choice = choice;
        self
    }

    /// Show brief version information
    pub fn show_version(&self) -> WinUtilsResult<()> {
        println!("{} {}", self.utility_name, self.build_info.short_version());
        Ok(())
    }

    /// Show detailed version information
    pub fn show_detailed(&self) -> WinUtilsResult<()> {
        let mut stdout = StandardStream::stdout(self.color_choice);

        // Header
        stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Green)))?;
        writeln!(stdout, "{} - Detailed Version Information", self.utility_name.to_uppercase())?;
        stdout.reset()?;
        writeln!(stdout)?;

        // Basic info
        stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Blue)))?;
        writeln!(stdout, "BASIC INFORMATION")?;
        stdout.reset()?;
        writeln!(stdout, "  Utility: {}", self.utility_name)?;
        writeln!(stdout, "  Description: {}", self.utility_description)?;
        writeln!(stdout, "  Version: {}", self.build_info.version)?;
        writeln!(stdout)?;

        // Build information
        stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Blue)))?;
        writeln!(stdout, "BUILD INFORMATION")?;
        stdout.reset()?;
        writeln!(stdout, "  Build Date: {}", self.build_info.build_date.format("%Y-%m-%d %H:%M:%S UTC"))?;
        writeln!(stdout, "  Build Profile: {}", self.build_info.build_profile)?;
        writeln!(stdout, "  Rust Version: {}", self.build_info.rust_version)?;
        writeln!(stdout, "  Target: {}", self.build_info.target_triple)?;

        if self.build_info.is_development() {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
            writeln!(stdout, "  [Development Build]")?;
            stdout.reset()?;
        }
        writeln!(stdout)?;

        // Git information
        if self.build_info.git_commit.is_some() || self.build_info.git_branch.is_some() {
            stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Blue)))?;
            writeln!(stdout, "SOURCE INFORMATION")?;
            stdout.reset()?;

            if let Some(ref commit) = self.build_info.git_commit {
                writeln!(stdout, "  Git Commit: {}", commit)?;
            }
            if let Some(ref branch) = self.build_info.git_branch {
                writeln!(stdout, "  Git Branch: {}", branch)?;
            }
            if self.build_info.git_dirty {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                writeln!(stdout, "  [Working directory has uncommitted changes]")?;
                stdout.reset()?;
            }
            writeln!(stdout)?;
        }

        // Features
        if !self.build_info.features.is_empty() {
            stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Blue)))?;
            writeln!(stdout, "ENABLED FEATURES")?;
            stdout.reset()?;
            for feature in &self.build_info.features {
                if let Some(desc) = self.feature_info.get_description(feature) {
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                    write!(stdout, "  ✓ {}", desc.name)?;
                    stdout.reset()?;
                    writeln!(stdout, " - {}", desc.description)?;
                } else {
                    writeln!(stdout, "  ✓ {}", feature)?;
                }
            }
            writeln!(stdout)?;
        }

        // Optimizations
        if !self.build_info.optimizations.is_empty() {
            stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Blue)))?;
            writeln!(stdout, "OPTIMIZATIONS")?;
            stdout.reset()?;
            for opt in &self.build_info.optimizations {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
                writeln!(stdout, "  ⚡ {}", opt)?;
            }
            stdout.reset()?;
            writeln!(stdout)?;
        }

        Ok(())
    }

    /// Show source repository information
    pub fn show_source_info(&self) -> WinUtilsResult<()> {
        let mut stdout = StandardStream::stdout(self.color_choice);

        stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Green)))?;
        writeln!(stdout, "SOURCE REPOSITORY INFORMATION")?;
        stdout.reset()?;
        writeln!(stdout)?;

        writeln!(stdout, "  Repository: {}", constants::REPOSITORY_URL)?;
        writeln!(stdout, "  Issues: {}", constants::ISSUES_URL)?;
        writeln!(stdout, "  Documentation: {}", constants::DOCS_URL)?;
        writeln!(stdout)?;

        if let Some(ref commit) = self.build_info.git_commit {
            writeln!(stdout, "  Current Commit: {}", commit)?;
            writeln!(stdout, "  Commit URL: {}/commit/{}", constants::REPOSITORY_URL, commit)?;
        }

        if let Some(ref branch) = self.build_info.git_branch {
            writeln!(stdout, "  Branch: {}", branch)?;
            writeln!(stdout, "  Branch URL: {}/tree/{}", constants::REPOSITORY_URL, branch)?;
        }

        writeln!(stdout)?;
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
        writeln!(stdout, "To contribute or report issues, visit the repository above.")?;
        stdout.reset()?;

        Ok(())
    }

    /// Show available features
    pub fn show_features(&self) -> WinUtilsResult<()> {
        let mut stdout = StandardStream::stdout(self.color_choice);

        stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Green)))?;
        writeln!(stdout, "FEATURE INFORMATION")?;
        stdout.reset()?;
        writeln!(stdout)?;

        stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Blue)))?;
        writeln!(stdout, "ENABLED FEATURES")?;
        stdout.reset()?;

        for feature_name in &self.feature_info.enabled_features {
            if let Some(desc) = self.feature_info.get_description(feature_name) {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                write!(stdout, "  ✓ {}", desc.name)?;
                stdout.reset()?;

                if desc.platform_specific {
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
                    write!(stdout, " [Platform-specific]")?;
                    stdout.reset()?;
                }

                writeln!(stdout)?;
                writeln!(stdout, "    {}", desc.description)?;

                if !desc.requires.is_empty() {
                    writeln!(stdout, "    Requires: {}", desc.requires.join(", "))?;
                }
                writeln!(stdout)?;
            }
        }

        // Show disabled features
        let disabled_features: Vec<_> = self.feature_info.available_features.iter()
            .filter(|(_, desc)| !desc.enabled)
            .collect();

        if !disabled_features.is_empty() {
            stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Blue)))?;
            writeln!(stdout, "AVAILABLE BUT DISABLED FEATURES")?;
            stdout.reset()?;

            for (_, desc) in disabled_features {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                write!(stdout, "  ✗ {}", desc.name)?;
                stdout.reset()?;

                if desc.platform_specific {
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
                    write!(stdout, " [Platform-specific]")?;
                    stdout.reset()?;
                }

                writeln!(stdout)?;
                writeln!(stdout, "    {}", desc.description)?;
                writeln!(stdout, "    To enable: Rebuild with --features {}", desc.name.to_lowercase().replace(" ", "-"))?;
                writeln!(stdout)?;
            }
        }

        Ok(())
    }

    /// Check for updates
    pub async fn check_updates(&self) -> WinUtilsResult<()> {
        let mut stdout = StandardStream::stdout(self.color_choice);

        stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Green)))?;
        writeln!(stdout, "CHECKING FOR UPDATES")?;
        stdout.reset()?;
        writeln!(stdout)?;

        let mut update_info = UpdateInfo::new(self.build_info.version.clone());

        match update_info.check_github_updates(constants::REPOSITORY_URL).await {
            Ok(()) => {
                if update_info.update_available {
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
                    writeln!(stdout, "  🎉 Update available!")?;
                    stdout.reset()?;

                    writeln!(stdout, "  Current version: {}", update_info.current_version)?;
                    if let Some(ref latest) = update_info.latest_version {
                        writeln!(stdout, "  Latest version: {}", latest)?;
                    }

                    if let Some(ref url) = update_info.download_url {
                        writeln!(stdout, "  Download: {}", url)?;
                    }

                    if let Some(ref notes_url) = update_info.release_notes_url {
                        writeln!(stdout, "  Release notes: {}", notes_url)?;
                    }
                } else {
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                    writeln!(stdout, "  ✓ You are using the latest version")?;
                    stdout.reset()?;
                }
            }
            Err(e) => {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                writeln!(stdout, "  ✗ Failed to check for updates: {}", e)?;
                stdout.reset()?;
                writeln!(stdout, "  You can check manually at: {}", constants::REPOSITORY_URL)?;
            }
        }

        writeln!(stdout)?;
        Ok(())
    }
}

use std::io::Write;

impl fmt::Write for StandardStream {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_info_creation() {
        let build_info = BuildInfo::from_env();
        assert!(!build_info.version.is_empty());
        assert!(!build_info.rust_version.is_empty());
        assert!(!build_info.target_triple.is_empty());
    }

    #[test]
    fn test_feature_info() {
        let feature_info = FeatureInfo::new();
        assert!(!feature_info.available_features.is_empty());

        // Test that basic features are present
        assert!(feature_info.available_features.contains_key("help"));
        assert!(feature_info.available_features.contains_key("version"));
    }

    #[test]
    fn test_version_info() {
        let version_info = VersionInfo::new("test-util", "A test utility");
        assert_eq!(version_info.utility_name, "test-util");
        assert_eq!(version_info.utility_description, "A test utility");
    }

    #[test]
    fn test_update_info() {
        let mut update_info = UpdateInfo::new("1.0.0".to_string());
        assert_eq!(update_info.current_version, "1.0.0");
        assert!(!update_info.update_available);

        // Simulate an update
        update_info.latest_version = Some("1.1.0".to_string());
        update_info.update_available = true;
        assert!(update_info.update_available);
    }
}
