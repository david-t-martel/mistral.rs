//! Windows-optimized cat utility
//!
//! Concatenate and display files with Windows-specific enhancements
//!
//! This is a Windows-optimized version of the cat utility from uutils/coreutils.
//! It provides enhanced Windows path handling, performance optimizations,
//! BOM detection, line ending conversion, and Windows-specific features while
//! maintaining compatibility with the original GNU cat.

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use thiserror::Error;
use winpath::normalize_path;

mod bom;
mod file_reader;
mod line_endings;
mod windows_fs;

use bom::{BomInfo, BomType};
use file_reader::WindowsFileReader;
use line_endings::LineEndingConverter;

#[derive(Error, Debug)]
pub enum CatError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Path error: {0}")]
    Path(String),
    #[error("Encoding error: {0}")]
    Encoding(String),
}

type Result<T> = std::result::Result<T, CatError>;

#[derive(Debug)]
struct CatConfig {
    files: Vec<PathBuf>,
    show_ends: bool,
    show_tabs: bool,
    show_nonprinting: bool,
    number_lines: bool,
    number_nonblank: bool,
    squeeze_blank: bool,

    // Windows-specific options
    convert_crlf: bool,
    convert_lf: bool,
    strip_bom: bool,
    show_bom: bool,
    use_mmap: bool,
    buffer_size: usize,
}

impl Default for CatConfig {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            show_ends: false,
            show_tabs: false,
            show_nonprinting: false,
            number_lines: false,
            number_nonblank: false,
            squeeze_blank: false,
            convert_crlf: false,
            convert_lf: false,
            strip_bom: false,
            show_bom: false,
            use_mmap: true, // Enable by default for performance
            buffer_size: 65536, // 64KB optimal for NTFS
        }
    }
}

fn main() {
    let matches = build_cli().get_matches();

    let mut config = CatConfig::default();

    // Parse standard cat options
    config.show_ends = matches.get_flag("show-ends");
    config.show_tabs = matches.get_flag("show-tabs");
    config.show_nonprinting = matches.get_flag("show-nonprinting");
    config.number_lines = matches.get_flag("number");
    config.number_nonblank = matches.get_flag("number-nonblank");
    config.squeeze_blank = matches.get_flag("squeeze-blank");

    // Parse Windows-specific options
    config.convert_crlf = matches.get_flag("crlf");
    config.convert_lf = matches.get_flag("lf");
    config.strip_bom = matches.get_flag("strip-bom");
    config.show_bom = matches.get_flag("show-bom");

    if let Some(buffer_size_str) = matches.get_one::<String>("buffer-size") {
        if let Ok(size) = buffer_size_str.parse::<usize>() {
            config.buffer_size = size;
        }
    }

    // Collect and normalize file paths
    if let Some(files) = matches.get_many::<String>("FILE") {
        for file in files {
            match normalize_windows_path(file) {
                Ok(path) => config.files.push(path),
                Err(e) => {
                    eprintln!("cat: {}: {}", file, e);
                    process::exit(1);
                }
            }
        }
    }

    // If no files specified, read from stdin
    if config.files.is_empty() {
        config.files.push(PathBuf::from("-"));
    }

    // Validate conflicting options
    if config.convert_crlf && config.convert_lf {
        eprintln!("cat: cannot specify both --crlf and --lf options");
        process::exit(1);
    }

    // Try Windows-optimized implementation first, fallback to GNU if needed
    match run_windows_cat(&config) {
        Ok(()) => process::exit(0),
        Err(e) => {
            eprintln!("cat: {}", e);

            // Fallback to original uutils implementation for compatibility
            if should_fallback(&e) {
                eprintln!("cat: falling back to GNU implementation");
                let args: Vec<OsString> = std::env::args().map(OsString::from).collect();
                let exit_code = uu_cat::uumain(args.into_iter());
                process::exit(exit_code);
            } else {
                process::exit(1);
            }
        }
    }
}

fn build_cli() -> Command {
    Command::new("cat")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Concatenate and display files - Windows optimized")
        .arg(Arg::new("FILE")
            .help("Files to concatenate (use '-' for stdin)")
            .action(ArgAction::Append)
            .value_name("FILE"))

        // Standard GNU cat options
        .arg(Arg::new("show-ends")
            .short('E')
            .long("show-ends")
            .help("Display $ at end of each line")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("number")
            .short('n')
            .long("number")
            .help("Number all output lines")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("number-nonblank")
            .short('b')
            .long("number-nonblank")
            .help("Number nonempty output lines")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("show-tabs")
            .short('T')
            .long("show-tabs")
            .help("Display TAB characters as ^I")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("show-nonprinting")
            .short('v')
            .long("show-nonprinting")
            .help("Use ^ and M- notation, except for LFD and TAB")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("squeeze-blank")
            .short('s')
            .long("squeeze-blank")
            .help("Suppress repeated empty output lines")
            .action(ArgAction::SetTrue))

        // Windows-specific options
        .arg(Arg::new("crlf")
            .long("crlf")
            .help("Convert LF line endings to CRLF")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("lf")
            .long("lf")
            .help("Convert CRLF line endings to LF")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("strip-bom")
            .long("strip-bom")
            .help("Remove Byte Order Mark (BOM) from output")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("show-bom")
            .long("show-bom")
            .help("Display information about Byte Order Mark (BOM)")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("buffer-size")
            .long("buffer-size")
            .help("Set buffer size for file operations (default: 64KB)")
            .value_name("BYTES"))
}

fn normalize_windows_path(path: &str) -> Result<PathBuf> {
    if path == "-" {
        return Ok(PathBuf::from("-"));
    }

    normalize_path(path)
        .map(|s| PathBuf::from(s))
        .map_err(|e| CatError::Path(format!("failed to normalize path '{}': {}", path, e)))
}

fn run_windows_cat(config: &CatConfig) -> Result<()> {
    let stdout = io::stdout();
    let mut writer = BufWriter::with_capacity(config.buffer_size, stdout.lock());

    let mut line_number = 1;
    let mut last_line_blank = false;

    for file_path in &config.files {
        if config.show_bom {
            if let Ok(bom_info) = detect_bom(file_path) {
                if bom_info.bom_type != BomType::None {
                    writeln!(writer, "BOM detected: {} ({} bytes)",
                           bom_info.bom_type, bom_info.bom_length)?;
                }
            }
        }

        let mut reader = WindowsFileReader::new(file_path, config)?;
        let mut converter = LineEndingConverter::new(config.convert_crlf, config.convert_lf);

        if config.strip_bom {
            reader.skip_bom()?;
        }

        let mut buffer = vec![0u8; config.buffer_size];
        let mut line_buffer = Vec::new();
        let mut in_line = false;

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            let data = &buffer[..bytes_read];
            let converted = converter.convert(data)?;

            // Process line by line for numbering and other options
            if config.number_lines || config.number_nonblank || config.squeeze_blank ||
               config.show_ends || config.show_tabs || config.show_nonprinting {

                for &byte in &converted {
                    if byte == b'\n' {
                        let line = String::from_utf8_lossy(&line_buffer);
                        let is_blank = line.trim().is_empty();

                        if config.squeeze_blank && is_blank && last_line_blank {
                            line_buffer.clear();
                            continue;
                        }

                        if config.number_lines || (config.number_nonblank && !is_blank) {
                            write!(writer, "{:6}\t", line_number)?;
                            line_number += 1;
                        }

                        write_processed_line(&mut writer, &line_buffer, config)?;

                        if config.show_ends {
                            write!(writer, "$")?;
                        }

                        writeln!(writer)?;
                        line_buffer.clear();
                        last_line_blank = is_blank;
                        in_line = false;
                    } else {
                        line_buffer.push(byte);
                        in_line = true;
                    }
                }
            } else {
                // Direct write for performance when no special processing needed
                writer.write_all(&converted)?;
            }
        }

        // Handle final line without newline
        if in_line && !line_buffer.is_empty() {
            let line = String::from_utf8_lossy(&line_buffer);
            let is_blank = line.trim().is_empty();

            if !(config.squeeze_blank && is_blank && last_line_blank) {
                if config.number_lines || (config.number_nonblank && !is_blank) {
                    write!(writer, "{:6}\t", line_number)?;
                }

                write_processed_line(&mut writer, &line_buffer, config)?;

                if config.show_ends {
                    write!(writer, "$")?;
                }
            }
        }
    }

    writer.flush()?;
    Ok(())
}

fn write_processed_line(writer: &mut BufWriter<io::StdoutLock>,
                       line_data: &[u8],
                       config: &CatConfig) -> Result<()> {
    if !config.show_tabs && !config.show_nonprinting {
        writer.write_all(line_data)?;
        return Ok(());
    }

    for &byte in line_data {
        match byte {
            b'\t' if config.show_tabs => {
                writer.write_all(b"^I")?;
            }
            b if config.show_nonprinting => {
                match b {
                    0..=8 | 11..=12 | 14..=31 => {
                        write!(writer, "^{}", (b + 64) as char)?;
                    }
                    127 => {
                        writer.write_all(b"^?")?;
                    }
                    128..=159 => {
                        write!(writer, "M-^{}", (b - 128 + 64) as char)?;
                    }
                    160..=254 => {
                        write!(writer, "M-{}", (b - 128) as char)?;
                    }
                    255 => {
                        writer.write_all(b"M-^?")?;
                    }
                    _ => {
                        writer.write_all(&[b])?;
                    }
                }
            }
            _ => {
                writer.write_all(&[byte])?;
            }
        }
    }

    Ok(())
}

fn detect_bom(file_path: &Path) -> Result<BomInfo> {
    if file_path.to_str() == Some("-") {
        return Ok(BomInfo::new(BomType::None, 0));
    }

    let mut file = File::open(file_path)?;
    let mut buffer = [0u8; 4];
    let bytes_read = file.read(&mut buffer)?;

    Ok(BomInfo::detect(&buffer[..bytes_read]))
}

fn should_fallback(error: &CatError) -> bool {
    match error {
        CatError::Encoding(_) => false, // Our encoding handling should be better
        CatError::Path(_) => true,      // Might be a path issue we can't handle
        CatError::Io(io_err) => {
            match io_err.kind() {
                io::ErrorKind::PermissionDenied => true, // GNU cat might handle this differently
                io::ErrorKind::NotFound => false,        // We can handle this
                _ => true,                               // Other IO errors might need GNU handling
            }
        }
    }
}
