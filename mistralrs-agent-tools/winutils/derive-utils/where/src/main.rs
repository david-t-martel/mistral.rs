// Copyright (c) 2024 uutils developers
// SPDX-License-Identifier: MIT OR Apache-2.0

//! High-performance Windows `where` command implementation
//!
//! This implementation provides faster execution than the native Windows where.exe
//! by using efficient PATH caching, parallel directory searching, and optimized
//! pattern matching.

use anyhow::{Context, Result};
use clap::Parser;
use std::process;

mod args;
mod cache;
mod error;
mod pathext;
mod search;
mod output;
mod wildcard;

use args::Args;
use error::WhereError;
use search::SearchEngine;

fn main() {
    let args = Args::parse();

    #[cfg(feature = "tracing")]
    {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    }

    if let Err(e) = run(args.clone()) {
        if args.quiet {
            process::exit(1);
        } else {
            eprintln!("where: {}", e);
            process::exit(1);
        }
    }
}

fn run(args: Args) -> Result<()> {
    let mut search_engine = SearchEngine::new(args)?;
    let results = search_engine.search()
        .context("Failed to search for executables")?;

    if results.is_empty() {
        return Err(WhereError::NotFound.into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_basic_search() {
        let temp = TempDir::new().unwrap();
        let exe_path = temp.path().join("test.exe");
        fs::write(&exe_path, b"").unwrap();

        let args = Args {
            patterns: vec!["test.exe".to_string()],
            recursive_dir: Some(temp.path().to_string_lossy().to_string()),
            quiet: false,
            full_path: false,
            show_time: false,
        };

        let mut engine = SearchEngine::new(args).unwrap();
        let results = engine.search().unwrap();

        assert!(!results.is_empty());
    }

    #[test]
    fn test_wildcard_search() {
        let temp = TempDir::new().unwrap();
        let exe1 = temp.path().join("test1.exe");
        let exe2 = temp.path().join("test2.exe");
        fs::write(&exe1, b"").unwrap();
        fs::write(&exe2, b"").unwrap();

        let args = Args {
            patterns: vec!["test*.exe".to_string()],
            recursive_dir: Some(temp.path().to_string_lossy().to_string()),
            quiet: false,
            full_path: false,
            show_time: false,
        };

        let mut engine = SearchEngine::new(args).unwrap();
        let results = engine.search().unwrap();

        assert_eq!(results.len(), 2);
    }
}
