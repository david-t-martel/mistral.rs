use std::env;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // Enable Windows-specific features only on Windows
    if target_os == "windows" {
        println!("cargo:rustc-cfg=windows_features");

        // Link against Windows APIs
        println!("cargo:rustc-link-lib=kernel32");
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=advapi32");
    }

    // Set version information
    if let Ok(version) = env::var("CARGO_PKG_VERSION") {
        println!("cargo:rustc-env=TREE_VERSION={}", version);
    }

    // Set build timestamp
    let now = chrono::Utc::now();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", now.format("%Y-%m-%d %H:%M:%S UTC"));

    // Rebuild if build script changes
    println!("cargo:rerun-if-changed=build.rs");
}
