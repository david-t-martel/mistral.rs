// Copyright (c) 2024 uutils developers
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Build script for the where utility

use std::env;
use std::path::Path;

fn main() {
    // Enable link-time optimization for release builds
    if env::var("PROFILE").unwrap() == "release" {
        println!("cargo:rustc-link-arg=-fuse-ld=lld"); // Use LLD linker if available
    }

    // Windows-specific build configuration
    if cfg!(target_os = "windows") {
        // Increase stack size for Windows
        println!("cargo:rustc-link-arg=/STACK:8388608");

        // Enable Windows subsystem for console applications
        println!("cargo:rustc-link-arg=/SUBSYSTEM:CONSOLE");

        // Link against Windows libraries
        println!("cargo:rustc-link-lib=kernel32");
        println!("cargo:rustc-link-lib=shell32");
    }

    // Set version information from Cargo.toml
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    println!("cargo:rustc-env=VERSION={}", version);

    // Set build timestamp
    let build_time = chrono::Utc::now().to_rfc3339();
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);

    // Git commit hash (if available)
    if let Ok(output) = std::process::Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
    {
        if output.status.success() {
            let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-env=GIT_COMMIT={}", commit);
        }
    }

    // Rerun if Cargo.toml changes
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=build.rs");
}
