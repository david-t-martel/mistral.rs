//! Windows-optimized ls utility
//!
//! A high-performance directory listing tool that leverages Windows APIs
//! for maximum speed and Windows-specific features.

mod windows_attrs;

use anyhow::{Context, Result};
use bytesize::ByteSize;
use chrono::{DateTime, Local};
use clap::{Arg, Command};
use crossbeam_channel::{unbounded, Receiver, Sender};
use dashmap::DashMap;
use rayon::prelude::*;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs::{self, Metadata};
use std::io::Write;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use unicode_width::UnicodeWidthStr;
use winpath::{normalize_path, PathNormalizer};
use windows_sys::Win32::Storage::FileSystem::*;

use crate::windows_attrs::WindowsFileAttributes;

/// Configuration for the ls command
#[derive(Debug, Clone)]
struct LsConfig {
    /// Paths to list
    paths: Vec<PathBuf>,
    /// Show hidden files
    all: bool,
    /// Long format
    long: bool,
    /// Human readable sizes
    human_readable: bool,
    /// Sort by modification time
    sort_time: bool,
    /// Reverse sort order
    reverse: bool,
    /// Recursive listing
    recursive: bool,
    /// JSON output format
    json: bool,
    /// Unix-compatible mode
    unix_compat: bool,
    /// Show Windows attributes
    windows_attrs: bool,
    /// Show alternate data streams
    show_ads: bool,
    /// Show file owner
    show_owner: bool,
    /// Use colors
    color: bool,
    /// One file per line
    one_per_line: bool,
    /// Directory first
    directories_first: bool,
    /// Maximum parallel workers
    max_workers: usize,
    /// Show detailed performance stats
    show_stats: bool,
}

impl Default for LsConfig {
    fn default() -> Self {
        Self {
            paths: vec![PathBuf::from(".")],
            all: false,
            long: false,
            human_readable: false,
            sort_time: false,
            reverse: false,
            recursive: false,
            json: false,
            unix_compat: false,
            windows_attrs: true,
            show_ads: false,
            show_owner: false,
            color: true,
            one_per_line: false,
            directories_first: false,
            max_workers: num_cpus::get().max(2),
            show_stats: false,
        }
    }
}

/// File entry with metadata and Windows-specific attributes
#[derive(Debug, Clone, serde::Serialize)]
struct FileEntry {
    /// File name
    name: String,
    /// Full path
    path: PathBuf,
    /// File size in bytes
    size: u64,
    /// Is directory
    is_dir: bool,
    /// Is symlink
    is_symlink: bool,
    /// Modified time (Unix timestamp)
    modified: u64,
    /// Created time (Unix timestamp)
    created: Option<u64>,
    /// Accessed time (Unix timestamp)
    accessed: Option<u64>,
    /// Unix-style permissions (if unix_compat mode)
    permissions: Option<String>,
    /// Windows attributes
    #[serde(skip_serializing_if = "Option::is_none")]
    windows_attrs: Option<WindowsFileAttributes>,
}

/// Performance statistics
#[derive(Debug)]
struct PerformanceStats {
    /// Total files processed
    files_processed: AtomicUsize,
    /// Total directories processed
    dirs_processed: AtomicUsize,
    /// Total bytes processed
    bytes_processed: AtomicUsize,
    /// Start time
    start_time: std::time::Instant,
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self {
            files_processed: AtomicUsize::new(0),
            dirs_processed: AtomicUsize::new(0),
            bytes_processed: AtomicUsize::new(0),
            start_time: std::time::Instant::now(),
        }
    }
}

impl PerformanceStats {
    fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            ..Default::default()
        }
    }

    fn add_file(&self, size: u64) {
        self.files_processed.fetch_add(1, Ordering::Relaxed);
        self.bytes_processed.fetch_add(size as usize, Ordering::Relaxed);
    }

    fn add_dir(&self) {
        self.dirs_processed.fetch_add(1, Ordering::Relaxed);
    }

    fn print_stats(&self) {
        let elapsed = self.start_time.elapsed();
        let files = self.files_processed.load(Ordering::Relaxed);
        let dirs = self.dirs_processed.load(Ordering::Relaxed);
        let bytes = self.bytes_processed.load(Ordering::Relaxed);

        eprintln!("\nPerformance Statistics:");
        eprintln!("  Files processed: {}", files);
        eprintln!("  Directories processed: {}", dirs);
        eprintln!("  Total bytes: {}", ByteSize(bytes as u64));
        eprintln!("  Time elapsed: {:.2}s", elapsed.as_secs_f64());
        eprintln!("  Files/sec: {:.0}", files as f64 / elapsed.as_secs_f64());
    }
}

fn main() -> Result<()> {
    let app = Command::new("ls")
        .version("1.0.0")
        .author("Windows Coreutils")
        .about("Windows-optimized ls utility with native API integration")
        .arg(Arg::new("paths")
            .help("Paths to list")
            .value_name("PATH")
            .num_args(0..)
            .default_value("."))
        .arg(Arg::new("all")
            .short('a')
            .long("all")
            .help("Show hidden files and directories")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("long")
            .short('l')
            .long("long")
            .help("Use long format")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("human-readable")
            .long("human-readable")
            .help("Show sizes in human readable format")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("sort-time")
            .short('t')
            .long("sort-time")
            .help("Sort by modification time")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("reverse")
            .short('r')
            .long("reverse")
            .help("Reverse sort order")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("recursive")
            .short('R')
            .long("recursive")
            .help("List directories recursively")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("json")
            .short('j')
            .long("json")
            .help("Output in JSON format")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("unix-compat")
            .short('u')
            .long("unix-compat")
            .help("Unix-compatible output mode")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("windows-attrs")
            .short('w')
            .long("windows-attrs")
            .help("Show Windows-specific attributes")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("ads")
            .long("ads")
            .help("Show alternate data streams")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("owner")
            .short('o')
            .long("owner")
            .help("Show file owner")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("no-color")
            .long("no-color")
            .help("Disable colored output")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("one")
            .short('1')
            .help("List one file per line")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("directories-first")
            .long("group-directories-first")
            .help("List directories before files")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("workers")
            .long("workers")
            .help("Number of parallel workers")
            .value_name("N")
            .value_parser(clap::value_parser!(usize)))
        .arg(Arg::new("stats")
            .long("stats")
            .help("Show performance statistics")
            .action(clap::ArgAction::SetTrue));

    let matches = app.get_matches();

    let paths: Vec<PathBuf> = matches
        .get_many::<String>("paths")
        .unwrap_or_default()
        .map(PathBuf::from)
        .collect();

    let config = LsConfig {
        paths: if paths.is_empty() { vec![PathBuf::from(".")] } else { paths },
        all: matches.get_flag("all"),
        long: matches.get_flag("long"),
        human_readable: matches.get_flag("human-readable"),
        sort_time: matches.get_flag("sort-time"),
        reverse: matches.get_flag("reverse"),
        recursive: matches.get_flag("recursive"),
        json: matches.get_flag("json"),
        unix_compat: matches.get_flag("unix-compat"),
        windows_attrs: matches.get_flag("windows-attrs") || !matches.get_flag("unix-compat"),
        show_ads: matches.get_flag("ads"),
        show_owner: matches.get_flag("owner"),
        color: !matches.get_flag("no-color"),
        one_per_line: matches.get_flag("one"),
        directories_first: matches.get_flag("directories-first"),
        max_workers: matches.get_one::<usize>("workers").copied()
            .unwrap_or_else(|| num_cpus::get().max(2)),
        show_stats: matches.get_flag("stats"),
    };

    // Initialize path normalizer for cross-format support
    let _normalizer = PathNormalizer::new();
    let stats = Arc::new(PerformanceStats::new());

    // Normalize all input paths (skip normalization for relative paths)
    let normalized_paths: Result<Vec<PathBuf>> = config.paths
        .iter()
        .map(|p| {
            let path_str = p.to_string_lossy();

            // If it's a relative path or already a valid Windows path, use as-is
            if p.is_relative() || path_str.starts_with(char::is_alphabetic) {
                Ok(p.clone())
            } else {
                // Try to normalize non-standard paths (WSL, Cygwin, etc.)
                let normalized = normalize_path(&path_str)
                    .with_context(|| format!("Failed to normalize path: {}", path_str))?;
                Ok(PathBuf::from(normalized))
            }
        })
        .collect();

    let normalized_paths = normalized_paths?;

    // Set up thread pool for parallel processing
    rayon::ThreadPoolBuilder::new()
        .num_threads(config.max_workers)
        .build_global()
        .context("Failed to initialize thread pool")?;

    // Collect all entries
    let all_entries = collect_entries(&normalized_paths, &config, &stats)?;

    // Output results
    if config.json {
        output_json(&all_entries, &config)?;
    } else {
        output_formatted(&all_entries, &config)?;
    }

    if config.show_stats {
        stats.print_stats();
    }

    Ok(())
}

/// Collect file entries from all specified paths
fn collect_entries(
    paths: &[PathBuf],
    config: &LsConfig,
    stats: &Arc<PerformanceStats>,
) -> Result<BTreeMap<PathBuf, Vec<FileEntry>>> {
    let results = DashMap::new();

    // Process paths in parallel
    paths.par_iter().try_for_each(|path| -> Result<()> {
        let entries = if config.recursive {
            collect_entries_recursive(path, config, stats)?
        } else {
            collect_entries_single(path, config, stats)?
        };

        results.insert(path.clone(), entries);
        Ok(())
    })?;

    // Convert to sorted BTreeMap
    let mut sorted_results = BTreeMap::new();
    for entry in results.into_iter() {
        sorted_results.insert(entry.0, entry.1);
    }

    Ok(sorted_results)
}

/// Collect entries from a single directory
fn collect_entries_single(
    path: &Path,
    config: &LsConfig,
    stats: &Arc<PerformanceStats>,
) -> Result<Vec<FileEntry>> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;

    if metadata.is_file() {
        // Single file
        let entry = create_file_entry(path, &metadata, config, stats)?;
        return Ok(vec![entry]);
    }

    // Directory listing
    let dir_entries = fs::read_dir(path)
        .with_context(|| format!("Failed to read directory: {}", path.display()))?;

    let (sender, receiver): (Sender<Result<FileEntry>>, Receiver<Result<FileEntry>>) = unbounded();

    // Process directory entries in parallel
    dir_entries
        .par_bridge()
        .try_for_each_with(sender, |s, entry| -> Result<()> {
            let entry = entry.with_context(|| format!("Failed to read directory entry in: {}", path.display()))?;
            let entry_path = entry.path();

            // Skip hidden files unless requested
            if !config.all && is_hidden(&entry_path)? {
                return Ok(());
            }

            let metadata = entry.metadata()
                .with_context(|| format!("Failed to get metadata for: {}", entry_path.display()))?;

            let file_entry = create_file_entry(&entry_path, &metadata, config, stats)?;
            s.send(Ok(file_entry)).map_err(|_| anyhow::anyhow!("Failed to send file entry"))?;

            Ok(())
        })?;

    // Collect results
    let mut entries: Vec<FileEntry> = receiver.into_iter().collect::<Result<Vec<_>>>()?;

    // Sort entries
    sort_entries(&mut entries, config);

    stats.add_dir();
    Ok(entries)
}

/// Collect entries recursively
fn collect_entries_recursive(
    path: &Path,
    config: &LsConfig,
    stats: &Arc<PerformanceStats>,
) -> Result<Vec<FileEntry>> {
    let mut all_entries = Vec::new();

    // Use walkdir for efficient recursive traversal
    let walker = walkdir::WalkDir::new(path)
        .follow_links(false)
        .same_file_system(true);

    // Collect entries in parallel batches
    let batch_size = 1000;
    let mut batch = Vec::with_capacity(batch_size);

    for entry in walker {
        let entry = entry.with_context(|| format!("Failed to walk directory: {}", path.display()))?;
        let entry_path = entry.path();

        // Skip hidden files unless requested
        if !config.all && is_hidden(entry_path)? {
            continue;
        }

        batch.push(entry_path.to_path_buf());

        if batch.len() >= batch_size {
            let batch_entries = process_batch(&batch, config, stats)?;
            all_entries.extend(batch_entries);
            batch.clear();
        }
    }

    // Process remaining entries
    if !batch.is_empty() {
        let batch_entries = process_batch(&batch, config, stats)?;
        all_entries.extend(batch_entries);
    }

    // Sort all entries
    sort_entries(&mut all_entries, config);

    Ok(all_entries)
}

/// Process a batch of paths in parallel
fn process_batch(
    paths: &[PathBuf],
    config: &LsConfig,
    stats: &Arc<PerformanceStats>,
) -> Result<Vec<FileEntry>> {
    let entries: Result<Vec<FileEntry>> = paths
        .par_iter()
        .map(|path| {
            let metadata = fs::metadata(path)
                .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;
            create_file_entry(path, &metadata, config, stats)
        })
        .collect();

    entries
}

/// Create a FileEntry from path and metadata
fn create_file_entry(
    path: &Path,
    metadata: &Metadata,
    config: &LsConfig,
    stats: &Arc<PerformanceStats>,
) -> Result<FileEntry> {
    let name = path.file_name()
        .unwrap_or_else(|| path.as_os_str())
        .to_string_lossy()
        .to_string();

    let size = metadata.len();
    let is_dir = metadata.is_dir();
    let is_symlink = metadata.is_symlink();

    // Update stats
    if is_dir {
        stats.add_dir();
    } else {
        stats.add_file(size);
    }

    let modified = metadata.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let created = metadata.created()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    let accessed = metadata.accessed()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    // Get Windows-specific attributes if requested
    let windows_attrs = if config.windows_attrs || config.show_ads || config.show_owner {
        WindowsFileAttributes::from_path(path).ok()
    } else {
        None
    };

    // Generate Unix-style permissions if in compatibility mode
    let permissions = if config.unix_compat {
        Some(format_unix_permissions(metadata, &windows_attrs))
    } else {
        None
    };

    Ok(FileEntry {
        name,
        path: path.to_path_buf(),
        size,
        is_dir,
        is_symlink,
        modified,
        created,
        accessed,
        permissions,
        windows_attrs,
    })
}

/// Check if a file is hidden (starts with . or has HIDDEN attribute)
fn is_hidden(path: &Path) -> Result<bool> {
    // Check for dot files (Unix convention)
    if let Some(name) = path.file_name() {
        if name.to_string_lossy().starts_with('.') {
            return Ok(true);
        }
    }

    // Check Windows HIDDEN attribute
    let attrs = fs::metadata(path)?.file_attributes();
    Ok(attrs & 0x2 != 0) // FILE_ATTRIBUTE_HIDDEN
}

/// Sort entries according to configuration
fn sort_entries(entries: &mut [FileEntry], config: &LsConfig) {
    entries.sort_by(|a, b| {
        use std::cmp::Ordering;

        // Directories first if requested
        if config.directories_first {
            match (a.is_dir, b.is_dir) {
                (true, false) => return Ordering::Less,
                (false, true) => return Ordering::Greater,
                _ => {}
            }
        }

        // Sort by time or name
        let order = if config.sort_time {
            b.modified.cmp(&a.modified) // Newest first for time sort
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        };

        if config.reverse {
            order.reverse()
        } else {
            order
        }
    });
}

/// Format Unix-style permissions
fn format_unix_permissions(metadata: &Metadata, windows_attrs: &Option<WindowsFileAttributes>) -> String {
    let mut perms = String::with_capacity(10);

    // File type
    if metadata.is_dir() {
        perms.push('d');
    } else if metadata.is_symlink() {
        perms.push('l');
    } else {
        perms.push('-');
    }

    // Owner permissions (simplified for Windows)
    perms.push_str("rwx");

    // Group permissions
    perms.push_str("r--");

    // Other permissions
    if let Some(attrs) = windows_attrs {
        if attrs.attributes & FILE_ATTRIBUTE_READONLY != 0 {
            perms.push_str("r--");
        } else {
            perms.push_str("rw-");
        }
    } else {
        perms.push_str("rw-");
    }

    perms
}

/// Output entries in JSON format
fn output_json(entries: &BTreeMap<PathBuf, Vec<FileEntry>>, _config: &LsConfig) -> Result<()> {
    let json_output = json!({
        "directories": entries.iter().map(|(path, files)| {
            json!({
                "path": path.display().to_string(),
                "files": files
            })
        }).collect::<Vec<_>>()
    });

    println!("{}", serde_json::to_string_pretty(&json_output)?);
    Ok(())
}

/// Output entries in formatted text
fn output_formatted(entries: &BTreeMap<PathBuf, Vec<FileEntry>>, config: &LsConfig) -> Result<()> {
    let mut stdout = StandardStream::stdout(if config.color {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    });

    for (path, files) in entries {
        // Print directory header for multiple directories
        if entries.len() > 1 {
            if config.color {
                stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Blue)))?;
            }
            writeln!(stdout, "{}:", path.display())?;
            if config.color {
                stdout.reset()?;
            }
        }

        if config.long {
            output_long_format(&mut stdout, files, config)?;
        } else {
            output_short_format(&mut stdout, files, config)?;
        }

        if entries.len() > 1 {
            writeln!(stdout)?;
        }
    }

    Ok(())
}

/// Output in long format (-l)
fn output_long_format(
    stdout: &mut StandardStream,
    entries: &[FileEntry],
    config: &LsConfig,
) -> Result<()> {
    for entry in entries {
        // Permissions/attributes
        if config.unix_compat {
            if let Some(ref perms) = entry.permissions {
                write!(stdout, "{} ", perms)?;
            }
        } else if let Some(ref attrs) = entry.windows_attrs {
            if config.color {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
            }
            write!(stdout, "{} ", attrs.format_attributes())?;
            if config.color {
                stdout.reset()?;
            }
        }

        // Hard links
        if let Some(ref attrs) = entry.windows_attrs {
            write!(stdout, "{:3} ", attrs.hard_links)?;
        } else {
            write!(stdout, "  1 ")?;
        }

        // Owner
        if config.show_owner {
            if let Some(ref attrs) = entry.windows_attrs {
                if let Some(ref owner) = attrs.owner {
                    write!(stdout, "{:12} ", owner)?;
                } else {
                    write!(stdout, "{:12} ", "unknown")?;
                }
            } else {
                write!(stdout, "{:12} ", "unknown")?;
            }
        }

        // Size
        if config.human_readable {
            write!(stdout, "{:>8} ", ByteSize(entry.size))?;
        } else {
            write!(stdout, "{:>12} ", entry.size)?;
        }

        // Modified time
        let datetime = DateTime::from_timestamp(entry.modified as i64, 0)
            .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());
        let local_dt: DateTime<Local> = datetime.into();
        write!(stdout, "{} ", local_dt.format("%b %d %H:%M"))?;

        // File name with colors
        write_colored_name(stdout, entry, config)?;

        // Alternate data streams
        if config.show_ads {
            if let Some(ref attrs) = entry.windows_attrs {
                if !attrs.ads_list.is_empty() {
                    if config.color {
                        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
                    }
                    write!(stdout, " [ADS: {}]", attrs.ads_list.join(", "))?;
                    if config.color {
                        stdout.reset()?;
                    }
                }
            }
        }

        writeln!(stdout)?;
    }

    Ok(())
}

/// Output in short format (default)
fn output_short_format(
    stdout: &mut StandardStream,
    entries: &[FileEntry],
    config: &LsConfig,
) -> Result<()> {
    if config.one_per_line {
        for entry in entries {
            write_colored_name(stdout, entry, config)?;
            writeln!(stdout)?;
        }
    } else {
        // Calculate column layout
        let term_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);
        let max_name_len = entries.iter()
            .map(|e| e.name.width())
            .max()
            .unwrap_or(0);
        let col_width = (max_name_len + 2).max(8);
        let cols = (term_width / col_width).max(1);

        for (i, entry) in entries.iter().enumerate() {
            write_colored_name(stdout, entry, config)?;

            if (i + 1) % cols == 0 || i == entries.len() - 1 {
                writeln!(stdout)?;
            } else {
                let padding = col_width - entry.name.width();
                write!(stdout, "{}", " ".repeat(padding))?;
            }
        }
    }

    Ok(())
}

/// Write file name with appropriate colors
fn write_colored_name(
    stdout: &mut StandardStream,
    entry: &FileEntry,
    config: &LsConfig,
) -> Result<()> {
    if !config.color {
        write!(stdout, "{}", entry.name)?;
        return Ok(());
    }

    let mut color_spec = ColorSpec::new();

    if entry.is_dir {
        color_spec.set_fg(Some(Color::Blue)).set_bold(true);
    } else if entry.is_symlink {
        color_spec.set_fg(Some(Color::Cyan));
    } else if entry.name.ends_with(".exe") || entry.name.ends_with(".bat") || entry.name.ends_with(".cmd") {
        color_spec.set_fg(Some(Color::Green)).set_bold(true);
    } else if let Some(ref attrs) = entry.windows_attrs {
        if attrs.attributes & FILE_ATTRIBUTE_HIDDEN != 0 {
            color_spec.set_fg(Some(Color::Black)).set_intense(true);
        } else if attrs.attributes & FILE_ATTRIBUTE_SYSTEM != 0 {
            color_spec.set_fg(Some(Color::Red));
        }
    }

    stdout.set_color(&color_spec)?;
    write!(stdout, "{}", entry.name)?;
    stdout.reset()?;

    // Add indicators
    if entry.is_dir {
        write!(stdout, "/")?;
    } else if entry.is_symlink {
        write!(stdout, "@")?;
    } else if let Some(ref attrs) = entry.windows_attrs {
        if attrs.junction_target.is_some() {
            write!(stdout, "@")?;
        }
    }

    Ok(())
}

// External crate for terminal size detection
mod term_size {
    use windows_sys::Win32::System::Console::*;
    use windows_sys::Win32::Foundation::*;

    pub fn dimensions() -> Option<(usize, usize)> {
        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            if handle == INVALID_HANDLE_VALUE {
                return None;
            }

            let mut info = CONSOLE_SCREEN_BUFFER_INFO {
                dwSize: COORD { X: 0, Y: 0 },
                dwCursorPosition: COORD { X: 0, Y: 0 },
                wAttributes: 0,
                srWindow: SMALL_RECT { Left: 0, Top: 0, Right: 0, Bottom: 0 },
                dwMaximumWindowSize: COORD { X: 0, Y: 0 },
            };

            if GetConsoleScreenBufferInfo(handle, &mut info) != 0 {
                let width = (info.srWindow.Right - info.srWindow.Left + 1) as usize;
                let height = (info.srWindow.Bottom - info.srWindow.Top + 1) as usize;
                Some((width, height))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_basic_listing() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let config = LsConfig::default();
        let stats = Arc::new(PerformanceStats::new());

        let entries = collect_entries_single(temp_dir.path(), &config, &stats).unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "test.txt"));
    }

    #[test]
    fn test_hidden_files() {
        let temp_dir = TempDir::new().unwrap();
        let visible_file = temp_dir.path().join("visible.txt");
        let hidden_file = temp_dir.path().join(".hidden.txt");

        fs::write(&visible_file, "visible").unwrap();
        fs::write(&hidden_file, "hidden").unwrap();

        let config = LsConfig { all: false, ..Default::default() };
        let stats = Arc::new(PerformanceStats::new());

        let entries = collect_entries_single(temp_dir.path(), &config, &stats).unwrap();
        assert!(entries.iter().any(|e| e.name == "visible.txt"));
        assert!(!entries.iter().any(|e| e.name == ".hidden.txt"));

        let config_all = LsConfig { all: true, ..Default::default() };
        let entries_all = collect_entries_single(temp_dir.path(), &config_all, &stats).unwrap();
        assert!(entries_all.iter().any(|e| e.name == ".hidden.txt"));
    }

    #[test]
    fn test_sorting() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("b.txt"), "b").unwrap();
        fs::write(temp_dir.path().join("a.txt"), "a").unwrap();
        fs::write(temp_dir.path().join("c.txt"), "c").unwrap();

        let config = LsConfig::default();
        let stats = Arc::new(PerformanceStats::new());

        let mut entries = collect_entries_single(temp_dir.path(), &config, &stats).unwrap();
        sort_entries(&mut entries, &config);

        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["a.txt", "b.txt", "c.txt"]);
    }

    #[test]
    fn test_path_normalization() {
        let normalizer = PathNormalizer::new();

        // Test various path formats
        let test_paths = vec![
            "/mnt/c/temp",
            "C:/temp",
            r"C:\temp",
            "/cygdrive/c/temp",
        ];

        for path in test_paths {
            if let Ok(normalized) = normalize_path(path) {
                assert!(normalized.contains("temp"));
            }
        }
    }
}
