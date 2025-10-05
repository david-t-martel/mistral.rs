use anyhow::Result;
use clap::Parser;
use std::process;

mod cli;
mod tree;
mod windows;
mod output;
mod utils;

use cli::Args;
use tree::TreeWalker;

/// Windows-optimized tree command with enhanced features
fn main() {
    // Set up high-DPI awareness for Windows console
    #[cfg(windows)]
    windows::setup_console();

    let args = Args::parse();

    if let Err(e) = run(args) {
        eprintln!("tree: {}", e);
        process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    let walker = TreeWalker::new(args)?;
    walker.walk()
}
