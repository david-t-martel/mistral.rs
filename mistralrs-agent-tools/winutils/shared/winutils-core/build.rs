//! Build script for winutils-core
//!
//! This build script sets up environment variables for version information
//! and git integration that can be used at runtime.

use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");

    // Set build date
    let build_date = chrono::Utc::now().to_rfc3339();
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);

    // Set build profile
    let profile = env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=PROFILE={}", profile);

    // Set Rust version
    if let Some(rustc_version) = get_rustc_version() {
        println!("cargo:rustc-env=RUSTC_VERSION={}", rustc_version);
    } else {
        println!("cargo:rustc-env=RUSTC_VERSION=unknown");
    }

    // Git information (if available)
    if let Some(commit) = get_git_commit() {
        println!("cargo:rustc-env=GIT_COMMIT={}", commit);
    }

    if let Some(branch) = get_git_branch() {
        println!("cargo:rustc-env=GIT_BRANCH={}", branch);
    }

    if is_git_dirty() {
        println!("cargo:rustc-env=GIT_DIRTY=true");
    } else {
        println!("cargo:rustc-env=GIT_DIRTY=false");
    }

    // Target information
    println!("cargo:rustc-env=TARGET={}", env::var("TARGET").unwrap_or_default());

    // Feature detection
    set_feature_flags();
}

fn get_rustc_version() -> Option<String> {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
}

fn get_git_commit() -> Option<String> {
    Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
}

fn get_git_branch() -> Option<String> {
    Command::new("git")
        .args(&["branch", "--show-current"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !branch.is_empty() {
                    Some(branch)
                } else {
                    // Fallback for detached HEAD
                    Command::new("git")
                        .args(&["describe", "--tags", "--exact-match", "HEAD"])
                        .output()
                        .ok()
                        .and_then(|output| {
                            if output.status.success() {
                                Some(format!("tag:{}", String::from_utf8_lossy(&output.stdout).trim()))
                            } else {
                                Some("detached".to_string())
                            }
                        })
                }
            } else {
                None
            }
        })
}

fn is_git_dirty() -> bool {
    Command::new("git")
        .args(&["diff", "--quiet"])
        .status()
        .map(|status| !status.success())
        .unwrap_or(false)
        ||
    Command::new("git")
        .args(&["diff", "--cached", "--quiet"])
        .status()
        .map(|status| !status.success())
        .unwrap_or(false)
}

fn set_feature_flags() {
    // Set compile-time feature flags that can be checked at runtime
    if cfg!(feature = "help") {
        println!("cargo:rustc-cfg=has_help_feature");
    }

    if cfg!(feature = "version") {
        println!("cargo:rustc-cfg=has_version_feature");
    }

    if cfg!(feature = "testing") {
        println!("cargo:rustc-cfg=has_testing_feature");
    }

    if cfg!(feature = "windows-enhanced") {
        println!("cargo:rustc-cfg=has_windows_enhanced_feature");
    }

    if cfg!(feature = "diagnostics") {
        println!("cargo:rustc-cfg=has_diagnostics_feature");
    }

    if cfg!(feature = "man-pages") {
        println!("cargo:rustc-cfg=has_man_pages_feature");
    }

    // Platform detection
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-cfg=is_windows");
    }

    if cfg!(target_family = "unix") {
        println!("cargo:rustc-cfg=is_unix");
    }

    // Architecture detection
    if cfg!(target_arch = "x86_64") {
        println!("cargo:rustc-cfg=is_x86_64");
    }

    if cfg!(target_arch = "aarch64") {
        println!("cargo:rustc-cfg=is_aarch64");
    }
}
