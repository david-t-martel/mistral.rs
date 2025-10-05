use clap::Parser;
use std::path::PathBuf;
use winpath::normalize_path;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "tree",
    version,
    about = "Windows-optimized directory tree viewer",
    long_about = "Display directory structures with Windows-specific enhancements including \
                  file attributes, junction points, and long path support."
)]
pub struct Args {
    /// Directory to start tree from
    #[arg(value_name = "DIRECTORY", default_value = ".")]
    pub directory: PathBuf,

    /// Show hidden and system files
    #[arg(short = 'a', long = "all")]
    pub show_all: bool,

    /// Show directories only
    #[arg(short = 'd', long = "dirs-only")]
    pub dirs_only: bool,

    /// Print full path prefix for each file
    #[arg(short = 'f', long = "full-path")]
    pub full_path: bool,

    /// Maximum depth to traverse (0 = unlimited)
    #[arg(short = 'L', long = "level", value_name = "DEPTH")]
    pub max_depth: Option<usize>,

    /// Show file sizes
    #[arg(long = "size")]
    pub show_size: bool,

    /// Show last modification time
    #[arg(long = "time")]
    pub show_time: bool,

    /// Output in JSON format
    #[arg(long = "json")]
    pub json_output: bool,

    /// Use ASCII characters only (no Unicode box drawing)
    #[arg(long = "ascii")]
    pub ascii_only: bool,

    /// Show Windows file attributes (H=Hidden, S=System, A=Archive, etc.)
    #[arg(long = "attrs")]
    pub show_attributes: bool,

    /// Show junction points and symbolic links
    #[arg(long = "links")]
    pub show_links: bool,

    /// Show alternate data streams (Windows NTFS feature)
    #[arg(long = "streams")]
    pub show_streams: bool,

    /// Follow symbolic links
    #[arg(long = "follow-links")]
    pub follow_links: bool,

    /// Sort files alphabetically
    #[arg(short = 's', long = "sort")]
    pub sort: bool,

    /// Sort by modification time (newest first)
    #[arg(short = 't', long = "sort-time")]
    pub sort_time: bool,

    /// Reverse sort order
    #[arg(short = 'r', long = "reverse")]
    pub reverse: bool,

    /// Pattern to match files (supports wildcards)
    #[arg(short = 'P', long = "pattern", value_name = "PATTERN")]
    pub pattern: Option<String>,

    /// Ignore pattern (supports wildcards)
    #[arg(short = 'I', long = "ignore", value_name = "PATTERN")]
    pub ignore_pattern: Option<String>,

    /// Enable color output
    #[arg(long = "color", default_value = "auto")]
    pub color: ColorOption,

    /// Number of threads for parallel processing
    #[arg(long = "threads", value_name = "N")]
    pub threads: Option<usize>,

    /// Disable parallel processing
    #[arg(long = "no-parallel")]
    pub no_parallel: bool,

    /// Show file count and size summary
    #[arg(long = "summary")]
    pub show_summary: bool,

    /// Include file extensions in output
    #[arg(long = "extensions")]
    pub show_extensions: bool,

    /// Show only files matching extension
    #[arg(long = "ext", value_name = "EXT")]
    pub filter_extension: Option<String>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ColorOption {
    Auto,
    Always,
    Never,
}

impl Args {
    /// Get the actual max depth to use (None means unlimited)
    pub fn effective_max_depth(&self) -> Option<usize> {
        self.max_depth.filter(|&depth| depth > 0)
    }

    /// Check if colors should be used
    pub fn use_colors(&self) -> bool {
        match self.color {
            ColorOption::Always => true,
            ColorOption::Never => false,
            ColorOption::Auto => atty::is(atty::Stream::Stdout),
        }
    }

    /// Get number of threads to use
    pub fn get_thread_count(&self) -> usize {
        if self.no_parallel {
            1
        } else {
            self.threads.unwrap_or_else(num_cpus::get)
        }
    }

    /// Get normalized directory path
    pub fn get_normalized_directory(&self) -> Result<PathBuf, winpath::PathError> {
        let dir_str = self.directory.to_string_lossy();
        let normalized = normalize_path(&dir_str)?;
        Ok(PathBuf::from(normalized))
    }
}
