//! Build script for derive-utils workspace
//!
//! Handles platform-specific compilation flags and feature detection.

use std::env;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Platform-specific optimizations
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    match target_os.as_str() {
        "windows" => {
            // Windows-specific optimizations
            println!("cargo:rustc-link-arg=/STACK:8388608"); // 8MB stack

            // Enable Windows API features
            println!("cargo:rustc-cfg=feature=\"windows-api\"");

            // Link Windows libraries
            println!("cargo:rustc-link-lib=kernel32");
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=shell32");
        }
        "linux" => {
            // Linux-specific optimizations
            println!("cargo:rustc-cfg=feature=\"linux-optimizations\"");
        }
        _ => {}
    }

    // Architecture-specific optimizations
    match target_arch.as_str() {
        "x86_64" => {
            // Enable x86_64 SIMD instructions
            println!("cargo:rustc-cfg=feature=\"simd\"");
        }
        "aarch64" => {
            // ARM64 optimizations
            println!("cargo:rustc-cfg=feature=\"neon\"");
        }
        _ => {}
    }

    // Check for optional dependencies
    if Path::new("Cargo.lock").exists() {
        println!("cargo:rustc-cfg=feature=\"has-lockfile\"");
    }

    // Build-time configuration
    let profile = env::var("PROFILE").unwrap_or_default();
    if profile == "release" {
        println!("cargo:rustc-cfg=feature=\"release-build\"");
    }

    // Set version information
    if let Ok(git_hash) = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        if git_hash.status.success() {
            let hash = String::from_utf8_lossy(&git_hash.stdout);
            println!("cargo:rustc-env=GIT_HASH={}", hash.trim());
        }
    }

    // Build timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);
}
