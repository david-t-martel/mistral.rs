//! Command-line options parsing for find wrapper

use anyhow::{anyhow, Result};
use clap::ArgMatches;
use std::path::PathBuf;

/// Main configuration for find operation
#[derive(Debug, Clone)]
pub struct FindOptions {
    pub paths: Vec<String>,
    pub name_pattern: Option<String>,
    pub iname_pattern: Option<String>,
    pub file_type: Option<String>,
    pub size_filter: Option<String>,
    pub newer_than: Option<PathBuf>,
    pub older_than: Option<PathBuf>,
    pub max_depth: Option<usize>,
    pub min_depth: Option<usize>,
    pub include_hidden: bool,
    pub follow_symlinks: bool,
    pub ignore_case: bool,
    pub parallel_threads: Option<usize>,
    pub color_mode: ColorMode,
    pub null_separator: bool,
    pub count_only: bool,
    pub exec_commands: Vec<String>,
    pub show_windows_attributes: bool,
    pub include_ntfs_streams: bool,
    pub include_junctions: bool,
    pub path_format: PathFormat,
    pub use_ignore_patterns: bool,
    pub ignore_git_ignore: bool,
    pub disable_sort: bool,
}

/// Color output mode
#[derive(Debug, Clone, PartialEq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

/// Path output format
#[derive(Debug, Clone, PartialEq)]
pub enum PathFormat {
    Windows,
    Unix,
    Native,
    Auto,
}

/// File type enumeration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileType {
    File,
    Directory,
    Symlink,
    Pipe,
    Socket,
    BlockDevice,
    CharDevice,
}

impl FindOptions {
    /// Parse options from command-line matches
    pub fn from_matches(matches: &ArgMatches) -> Result<Self> {
        let paths = matches
            .get_many::<String>("paths")
            .map(|values| values.cloned().collect())
            .unwrap_or_default();

        let parallel_threads = matches
            .get_one::<String>("parallel")
            .map(|s| s.parse::<usize>())
            .transpose()
            .map_err(|_| anyhow!("Invalid thread count"))?;

        let color_mode = match matches.get_one::<String>("color").map(|s| s.as_str()) {
            Some("always") => ColorMode::Always,
            Some("never") => ColorMode::Never,
            Some("auto") | None => ColorMode::Auto,
            Some(other) => return Err(anyhow!("Invalid color mode: {}", other)),
        };

        let path_format = match matches.get_one::<String>("path-format").map(|s| s.as_str()) {
            Some("windows") => PathFormat::Windows,
            Some("unix") => PathFormat::Unix,
            Some("native") => PathFormat::Native,
            Some("auto") | None => PathFormat::Auto,
            Some(other) => return Err(anyhow!("Invalid path format: {}", other)),
        };

        let max_depth = matches
            .get_one::<String>("maxdepth")
            .map(|s| s.parse::<usize>())
            .transpose()
            .map_err(|_| anyhow!("Invalid maxdepth value"))?;

        let min_depth = matches
            .get_one::<String>("mindepth")
            .map(|s| s.parse::<usize>())
            .transpose()
            .map_err(|_| anyhow!("Invalid mindepth value"))?;

        let newer_than = matches
            .get_one::<String>("newer")
            .map(PathBuf::from);

        let older_than = matches
            .get_one::<String>("older")
            .map(PathBuf::from);

        let exec_commands = matches
            .get_many::<String>("exec")
            .map(|values| values.cloned().collect())
            .unwrap_or_default();

        Ok(FindOptions {
            paths,
            name_pattern: matches.get_one::<String>("name").cloned(),
            iname_pattern: matches.get_one::<String>("iname").cloned(),
            file_type: matches.get_one::<String>("type").cloned(),
            size_filter: matches.get_one::<String>("size").cloned(),
            newer_than,
            older_than,
            max_depth,
            min_depth,
            include_hidden: matches.get_flag("hidden"),
            follow_symlinks: matches.get_flag("follow"),
            ignore_case: matches.get_flag("ignore-case"),
            parallel_threads,
            color_mode,
            null_separator: matches.get_flag("print0"),
            count_only: matches.get_flag("count"),
            exec_commands,
            show_windows_attributes: matches.get_flag("windows-attributes"),
            include_ntfs_streams: matches.get_flag("ntfs-streams"),
            include_junctions: matches.get_flag("junctions"),
            path_format,
            use_ignore_patterns: true, // Enable by default for performance
            ignore_git_ignore: false,
            disable_sort: false,
        })
    }
}

impl Default for FindOptions {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            name_pattern: None,
            iname_pattern: None,
            file_type: None,
            size_filter: None,
            newer_than: None,
            older_than: None,
            max_depth: None,
            min_depth: None,
            include_hidden: false,
            follow_symlinks: false,
            ignore_case: false,
            parallel_threads: None,
            color_mode: ColorMode::Auto,
            null_separator: false,
            count_only: false,
            exec_commands: Vec::new(),
            show_windows_attributes: false,
            include_ntfs_streams: false,
            include_junctions: false,
            path_format: PathFormat::Auto,
            use_ignore_patterns: true,
            ignore_git_ignore: false,
            disable_sort: false,
        }
    }
}
