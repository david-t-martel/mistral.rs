//! Windows-optimized cp utility
//!
//! Copy files and directories with Windows-specific optimizations
//!
//! This is a Windows-optimized version of the cp utility that provides:
//! - Cross-drive optimization using Windows CopyFileEx API
//! - NTFS junction and symbolic link support
//! - Windows file attributes preservation
//! - Performance optimizations for large files and network shares
//! - Parallel copying for multiple files
//! - Progress callbacks for large file operations

mod windows_cp;
mod copy_engine;
mod file_attributes;
mod junction_handler;
mod progress;

use std::env;
use std::ffi::OsString;
use std::process;
use clap::{Arg, ArgAction, Command};
use uucore::error::{UResult, UUsageError};
use uucore::format_usage;
use winpath::normalize_path;
use windows_cp::WindowsCpOptions;

const ABOUT: &str = "Copy files and directories (Windows optimized)";
const USAGE: &str = "cp [OPTION]... [-T] SOURCE DEST\n       cp [OPTION]... SOURCE... DIRECTORY\n       cp [OPTION]... -t DIRECTORY SOURCE...";

fn main() {
    let args: Vec<String> = env::args().collect();
    let processed_args = process_windows_args(args);

    match uu_main(processed_args.into_iter().map(OsString::from)) {
        Ok(()) => {},
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

pub fn uu_main(args: impl Iterator<Item = OsString>) -> UResult<()> {
    let config = uu_app();
    let matches = config.try_get_matches_from(args)?;

    // Try Windows-optimized path first
    if let Ok(options) = WindowsCpOptions::from_matches(&matches) {
        return windows_cp::copy_files(options);
    }

    // Fallback to original uutils implementation
    let exit_code = uu_cp::uumain(std::env::args().map(OsString::from));
    if exit_code != 0 {
        process::exit(exit_code);
    }
    Ok(())
}

fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version("0.1.0")
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        // Standard cp options
        .arg(
            Arg::new("archive")
                .short('a')
                .long("archive")
                .help("same as -dR --preserve=all")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("backup")
                .long("backup")
                .help("make a backup of each existing destination file")
                .value_name("CONTROL")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("copy-contents")
                .long("copy-contents")
                .help("copy contents of special files when recursive")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dereference")
                .short('L')
                .long("dereference")
                .help("always follow symbolic links in SOURCE")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("force")
                .short('f')
                .long("force")
                .help("if an existing destination file cannot be opened, remove it and try again")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("interactive")
                .short('i')
                .long("interactive")
                .help("prompt before overwrite")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("link")
                .short('l')
                .long("link")
                .help("hard link files instead of copying")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-clobber")
                .short('n')
                .long("no-clobber")
                .help("do not overwrite an existing file")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-dereference")
                .short('P')
                .long("no-dereference")
                .help("never follow symbolic links in SOURCE")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("preserve")
                .short('p')
                .long("preserve")
                .help("preserve specified attributes")
                .value_name("ATTR_LIST")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("parents")
                .long("parents")
                .help("use full source file name under DIRECTORY")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("recursive")
                .short('R')
                .short_alias('r')
                .long("recursive")
                .help("copy directories recursively")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("strip-trailing-slashes")
                .long("strip-trailing-slashes")
                .help("remove any trailing slashes from each SOURCE argument")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("symbolic-link")
                .short('s')
                .long("symbolic-link")
                .help("make symbolic links instead of copying")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("target-directory")
                .short('t')
                .long("target-directory")
                .help("copy all SOURCE arguments into DIRECTORY")
                .value_name("DIRECTORY")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("no-target-directory")
                .short('T')
                .long("no-target-directory")
                .help("treat DEST as a normal file")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("update")
                .short('u')
                .long("update")
                .help("copy only when the SOURCE file is newer than the destination file or when the destination file is missing")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("explain what is being done")
                .action(ArgAction::SetTrue),
        )
        // Windows-specific options
        .arg(
            Arg::new("follow-junctions")
                .long("follow-junctions")
                .help("follow NTFS junction points when copying")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("preserve-junctions")
                .long("preserve-junctions")
                .help("preserve NTFS junction points as junction points")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("preserve-security")
                .long("preserve-security")
                .help("preserve Windows security descriptors")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("preserve-streams")
                .long("preserve-streams")
                .help("preserve alternate data streams")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("unbuffered")
                .long("unbuffered")
                .help("use unbuffered I/O for large files")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("parallel")
                .short('j')
                .long("parallel")
                .help("number of parallel copy threads")
                .value_name("THREADS")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("progress")
                .long("progress")
                .help("show progress for large files")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("FILES")
                .help("Source and destination files")
                .action(ArgAction::Append)
                .required(true),
        )
}

/// Windows-optimized argument processing
fn process_windows_args(args: Vec<String>) -> Vec<String> {
    args.into_iter()
        .map(|arg| {
            // Normalize Windows paths in arguments
            if arg.contains('\\') || arg.contains('/') {
                match normalize_path(&arg) {
                    Ok(normalized) => normalized,
                    Err(_) => arg, // Keep original if normalization fails
                }
            } else {
                arg
            }
        })
        .collect()
}
