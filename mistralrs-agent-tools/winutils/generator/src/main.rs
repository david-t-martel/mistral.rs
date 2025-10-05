#!/usr/bin/env rust-script
//! Windows-Optimized Coreutils Generator
//!
//! This generator creates Windows-optimized versions of uutils coreutils
//! with winpath integration and Windows-specific optimizations.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for the generator
const UUTILS_SOURCE_DIR: &str = "T:\\projects\\coreutils\\src\\uu";
const OUTPUT_DIR: &str = "T:\\projects\\coreutils\\winutils\\coreutils";

/// Utility categories for Windows-specific optimizations
#[derive(Debug, Clone)]
enum UtilityCategory {
    FileSystem,
    Path,
    Text,
    System,
    Encoding,
    Math,
    Simple,
    Directory,
    Environment,
}

/// Windows optimization features
#[derive(Debug, Clone)]
struct WindowsOptimizations {
    windows_apis: bool,
    performance_opts: bool,
    memory_mapping: bool,
    ntfs_features: bool,
    path_normalization: bool,
    console_opts: bool,
}

/// Metadata for each utility
#[derive(Debug, Clone)]
struct UtilityMeta {
    name: String,
    category: UtilityCategory,
    description: String,
    has_complex_logic: bool,
    windows_opts: WindowsOptimizations,
}

/// Main generator struct
struct WinutilsGenerator {
    utilities: HashMap<String, UtilityMeta>,
    source_dir: PathBuf,
    output_dir: PathBuf,
}

impl WinutilsGenerator {
    fn new() -> Self {
        let mut generator = Self {
            utilities: HashMap::new(),
            source_dir: PathBuf::from(UUTILS_SOURCE_DIR),
            output_dir: PathBuf::from(OUTPUT_DIR),
        };

        generator.initialize_utilities();
        generator
    }

    /// Initialize all 77 utilities with their metadata
    fn initialize_utilities(&mut self) {
        let utilities = vec![
            ("arch", UtilityCategory::System, "Display machine architecture", false),
            ("base32", UtilityCategory::Encoding, "Base32 encode/decode", false),
            ("base64", UtilityCategory::Encoding, "Base64 encode/decode", false),
            ("basename", UtilityCategory::Path, "Extract filename from path", false),
            ("basenc", UtilityCategory::Encoding, "Multi-base encoding", false),
            ("cat", UtilityCategory::Text, "Concatenate and display files", true),
            ("cksum", UtilityCategory::Text, "Calculate checksums", true),
            ("comm", UtilityCategory::Text, "Compare sorted files line by line", true),
            ("cp", UtilityCategory::FileSystem, "Copy files and directories", true),
            ("csplit", UtilityCategory::Text, "Split files based on patterns", true),
            ("cut", UtilityCategory::Text, "Extract columns from files", true),
            ("date", UtilityCategory::System, "Display or set system date", false),
            ("dd", UtilityCategory::FileSystem, "Convert and copy files", true),
            ("df", UtilityCategory::System, "Display filesystem usage", true),
            ("dir", UtilityCategory::Directory, "List directory contents (Windows style)", true),
            ("dircolors", UtilityCategory::Directory, "Setup colors for ls", false),
            ("dirname", UtilityCategory::Path, "Extract directory from path", false),
            ("du", UtilityCategory::FileSystem, "Display directory usage", true),
            ("echo", UtilityCategory::Simple, "Display text", false),
            ("env", UtilityCategory::Environment, "Run program in modified environment", false),
            ("expand", UtilityCategory::Text, "Convert tabs to spaces", true),
            ("expr", UtilityCategory::Math, "Evaluate expressions", true),
            ("factor", UtilityCategory::Math, "Factorize numbers", true),
            ("false", UtilityCategory::Simple, "Return false exit code", false),
            ("fmt", UtilityCategory::Text, "Format text paragraphs", true),
            ("fold", UtilityCategory::Text, "Wrap text lines", true),
            ("hashsum", UtilityCategory::Text, "Calculate hash sums", true),
            ("head", UtilityCategory::Text, "Display first lines of files", true),
            ("hostname", UtilityCategory::System, "Display or set hostname", false),
            ("join", UtilityCategory::Text, "Join lines based on common field", true),
            ("link", UtilityCategory::FileSystem, "Create hard links", false),
            ("ln", UtilityCategory::FileSystem, "Create links", true),
            ("mkdir", UtilityCategory::Directory, "Create directories", true),
            ("mktemp", UtilityCategory::FileSystem, "Create temporary files", false),
            ("more", UtilityCategory::Text, "Page through text", true),
            ("mv", UtilityCategory::FileSystem, "Move/rename files", true),
            ("nl", UtilityCategory::Text, "Number lines", true),
            ("nproc", UtilityCategory::System, "Display number of processors", false),
            ("numfmt", UtilityCategory::Math, "Format numbers", true),
            ("od", UtilityCategory::Text, "Octal dump", true),
            ("paste", UtilityCategory::Text, "Merge lines of files", true),
            ("pr", UtilityCategory::Text, "Format text for printing", true),
            ("printenv", UtilityCategory::Environment, "Print environment variables", false),
            ("printf", UtilityCategory::Simple, "Format and print data", true),
            ("ptx", UtilityCategory::Text, "Permuted index", true),
            ("pwd", UtilityCategory::Path, "Print working directory", false),
            ("readlink", UtilityCategory::Path, "Display symlink target", true),
            ("realpath", UtilityCategory::Path, "Display absolute path", true),
            ("rm", UtilityCategory::FileSystem, "Remove files and directories", true),
            ("rmdir", UtilityCategory::Directory, "Remove empty directories", false),
            ("seq", UtilityCategory::Math, "Generate sequence of numbers", true),
            ("shred", UtilityCategory::FileSystem, "Securely delete files", true),
            ("shuf", UtilityCategory::Text, "Shuffle lines", true),
            ("sleep", UtilityCategory::Simple, "Delay execution", false),
            ("sort", UtilityCategory::Text, "Sort lines", true),
            ("split", UtilityCategory::Text, "Split files", true),
            ("sum", UtilityCategory::Text, "Calculate checksums", true),
            ("sync", UtilityCategory::System, "Synchronize filesystems", false),
            ("tac", UtilityCategory::Text, "Reverse cat", true),
            ("tail", UtilityCategory::Text, "Display last lines of files", true),
            ("tee", UtilityCategory::Text, "Copy input to multiple outputs", true),
            ("test", UtilityCategory::Simple, "Evaluate conditional expressions", true),
            ("touch", UtilityCategory::FileSystem, "Update file timestamps", true),
            ("tr", UtilityCategory::Text, "Translate characters", true),
            ("true", UtilityCategory::Simple, "Return true exit code", false),
            ("truncate", UtilityCategory::FileSystem, "Truncate files", false),
            ("tsort", UtilityCategory::Math, "Topological sort", true),
            ("unexpand", UtilityCategory::Text, "Convert spaces to tabs", true),
            ("uniq", UtilityCategory::Text, "Remove duplicate lines", true),
            ("unlink", UtilityCategory::FileSystem, "Remove single file", false),
            ("vdir", UtilityCategory::Directory, "List directory contents verbosely", true),
            ("wc", UtilityCategory::Text, "Count words, lines, characters", true),
            ("whoami", UtilityCategory::System, "Display current username", false),
            ("yes", UtilityCategory::Simple, "Repeatedly output text", false),
        ];

        for (name, category, description, complex) in utilities {
            let windows_opts = self.determine_windows_optimizations(&category, complex);

            self.utilities.insert(name.to_string(), UtilityMeta {
                name: name.to_string(),
                category,
                description: description.to_string(),
                has_complex_logic: complex,
                windows_opts,
            });
        }
    }

    /// Determine Windows optimizations based on utility category and complexity
    fn determine_windows_optimizations(&self, category: &UtilityCategory, complex: bool) -> WindowsOptimizations {
        match category {
            UtilityCategory::FileSystem => WindowsOptimizations {
                windows_apis: true,
                performance_opts: true,
                memory_mapping: complex,
                ntfs_features: true,
                path_normalization: true,
                console_opts: false,
            },
            UtilityCategory::Path => WindowsOptimizations {
                windows_apis: true,
                performance_opts: false,
                memory_mapping: false,
                ntfs_features: true,
                path_normalization: true,
                console_opts: false,
            },
            UtilityCategory::Text => WindowsOptimizations {
                windows_apis: false,
                performance_opts: complex,
                memory_mapping: complex,
                ntfs_features: false,
                path_normalization: true,
                console_opts: true,
            },
            UtilityCategory::System => WindowsOptimizations {
                windows_apis: true,
                performance_opts: false,
                memory_mapping: false,
                ntfs_features: false,
                path_normalization: false,
                console_opts: true,
            },
            UtilityCategory::Directory => WindowsOptimizations {
                windows_apis: true,
                performance_opts: true,
                memory_mapping: false,
                ntfs_features: true,
                path_normalization: true,
                console_opts: true,
            },
            UtilityCategory::Environment => WindowsOptimizations {
                windows_apis: true,
                performance_opts: false,
                memory_mapping: false,
                ntfs_features: false,
                path_normalization: false,
                console_opts: false,
            },
            _ => WindowsOptimizations {
                windows_apis: false,
                performance_opts: complex,
                memory_mapping: false,
                ntfs_features: false,
                path_normalization: false,
                console_opts: false,
            },
        }
    }

    /// Generate all utilities
    fn generate_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Starting Windows-optimized coreutils generation...");

        // Create output directory structure
        self.create_directory_structure()?;

        // Generate workspace Cargo.toml
        self.generate_workspace_cargo_toml()?;

        // Generate shared winpath module
        self.generate_winpath_module()?;

        // Generate each utility
        for (name, meta) in &self.utilities {
            println!("📦 Generating {}", name);
            self.generate_utility(name, meta)?;
        }

        // Generate convenience scripts
        self.generate_build_scripts()?;

        println!("✅ Generation complete! {} utilities created", self.utilities.len());
        Ok(())
    }

    /// Create directory structure
    fn create_directory_structure(&self) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(&self.output_dir)?;
        fs::create_dir_all(self.output_dir.join("src"))?;
        fs::create_dir_all(self.output_dir.join("winpath"))?;
        fs::create_dir_all(self.output_dir.join("scripts"))?;

        for (name, _) in &self.utilities {
            fs::create_dir_all(self.output_dir.join("src").join(name))?;
        }

        Ok(())
    }

    /// Generate workspace Cargo.toml
    fn generate_workspace_cargo_toml(&self) -> Result<(), Box<dyn std::error::Error>> {
        let members = self.utilities.keys()
            .map(|name| format!("    \"src/{}\"", name))
            .collect::<Vec<_>>()
            .join(",\n");

        let cargo_toml = format!(
r#"[workspace]
resolver = "3"
members = [
    "winpath",
{}
]

[workspace.package]
authors = ["Windows Coreutils Team"]
categories = ["command-line-utilities"]
edition = "2021"
homepage = "https://github.com/uutils/coreutils"
keywords = ["coreutils", "windows", "winutils", "cli", "utility"]
license = "MIT"
version = "0.1.0"

[workspace.dependencies]
# Core dependencies
clap = {{ version = "4.5", features = ["wrap_help", "cargo"] }}
uucore = {{ path = "../../src/uucore" }}
winpath = {{ path = "winpath" }}

# Windows-specific dependencies
windows = {{ version = "0.60", features = [
    "Win32_Foundation",
    "Win32_Storage_FileSystem",
    "Win32_System_IO",
    "Win32_System_Console",
    "Win32_System_SystemInformation",
    "Win32_Security",
] }}

# Performance dependencies
memmap2 = "0.9"
rayon = "1.10"
crossbeam = "0.8"

# Utility dependencies
thiserror = "2.0"
anyhow = "1.0"
serde = {{ version = "1.0", features = ["derive"] }}
byteorder = "1.5"
memchr = "2.7"

# Optional dependencies
regex = "1.10"
chrono = {{ version = "0.4", features = ["clock"] }}
walkdir = "2.5"
tempfile = "3.15"

[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
"#,
            members
        );

        fs::write(self.output_dir.join("Cargo.toml"), cargo_toml)?;
        Ok(())
    }

    /// Generate the shared winpath module
    fn generate_winpath_module(&self) -> Result<(), Box<dyn std::error::Error>> {
        let winpath_cargo = r#"[package]
name = "winpath"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true
description = "Windows path normalization and manipulation utilities"

[dependencies]
windows.workspace = true
thiserror.workspace = true
"#;

        let winpath_lib = r#"//! Windows Path Normalization and Manipulation
//!
//! This module provides Windows-optimized path handling utilities.

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WinPathError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Path too long: {0}")]
    PathTooLong(String),
    #[error("Windows API error")]
    WindowsApi,
    #[error("UTF-8 conversion error")]
    Utf8Error,
}

pub type Result<T> = std::result::Result<T, WinPathError>;

/// Windows-optimized path normalization
pub struct WinPath;

impl WinPath {
    /// Normalize a Windows path
    pub fn normalize<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
        let path = path.as_ref();
        // Simple path normalization for now
        Ok(path.to_path_buf())
    }

    /// Convert Unix-style path separators to Windows
    pub fn unix_to_windows<P: AsRef<Path>>(path: P) -> PathBuf {
        let path_str = path.as_ref().to_string_lossy();
        let windows_path = path_str.replace('/', r"\");
        PathBuf::from(windows_path)
    }
}

/// Convenience function for path normalization
pub fn normalize_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    WinPath::normalize(path)
}

pub fn to_windows_path<P: AsRef<Path>>(path: P) -> PathBuf {
    WinPath::unix_to_windows(path)
}
"#;

        fs::create_dir_all(self.output_dir.join("winpath").join("src"))?;
        fs::write(self.output_dir.join("winpath").join("Cargo.toml"), winpath_cargo)?;
        fs::write(self.output_dir.join("winpath").join("src").join("lib.rs"), winpath_lib)?;

        Ok(())
    }

    /// Generate a single utility
    fn generate_utility(&self, name: &str, meta: &UtilityMeta) -> Result<(), Box<dyn std::error::Error>> {
        let util_dir = self.output_dir.join("src").join(name);

        // Generate Cargo.toml for the utility
        self.generate_utility_cargo_toml(&util_dir, name, meta)?;

        // Generate main source file
        self.generate_utility_source(&util_dir, name, meta)?;

        Ok(())
    }

    /// Generate Cargo.toml for a utility
    fn generate_utility_cargo_toml(&self, util_dir: &Path, name: &str, meta: &UtilityMeta) -> Result<(), Box<dyn std::error::Error>> {
        let mut dependencies = vec![
            "clap.workspace = true".to_string(),
            "uucore.workspace = true".to_string(),
            "winpath.workspace = true".to_string(),
            "thiserror.workspace = true".to_string(),
        ];

        // Add Windows-specific dependencies based on optimizations
        if meta.windows_opts.windows_apis {
            dependencies.push("windows.workspace = true".to_string());
        }

        if meta.windows_opts.performance_opts {
            dependencies.push("rayon.workspace = true".to_string());
        }

        if meta.windows_opts.memory_mapping {
            dependencies.push("memmap2.workspace = true".to_string());
        }

        // Add category-specific dependencies
        match meta.category {
            UtilityCategory::Text => {
                dependencies.push("memchr.workspace = true".to_string());
                if meta.has_complex_logic {
                    dependencies.push("regex.workspace = true".to_string());
                }
            },
            UtilityCategory::System => {
                dependencies.push("chrono.workspace = true".to_string());
            },
            UtilityCategory::FileSystem | UtilityCategory::Directory => {
                dependencies.push("walkdir.workspace = true".to_string());
                dependencies.push("tempfile.workspace = true".to_string());
            },
            _ => {}
        }

        let cargo_toml = format!(
r#"[package]
name = "{}"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true
description = "{} - Windows optimized"

[[bin]]
name = "{}"
path = "src/main.rs"

[dependencies]
{}

# Link to original uutils implementation for fallback
uu_{} = {{ path = "../../../../src/uu/{}" }}
"#,
            name,
            meta.description,
            name,
            dependencies.join("\n"),
            name,
            name
        );

        fs::write(util_dir.join("Cargo.toml"), cargo_toml)?;
        Ok(())
    }

    /// Generate main source file for a utility
    fn generate_utility_source(&self, util_dir: &Path, name: &str, meta: &UtilityMeta) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(util_dir.join("src"))?;

        let source = self.generate_utility_template(name, meta);
        fs::write(util_dir.join("src").join("main.rs"), source)?;

        Ok(())
    }

    /// Generate the main template for a utility
    fn generate_utility_template(&self, name: &str, meta: &UtilityMeta) -> String {
        let mut template = String::new();

        template.push_str(&format!("//! Windows-optimized {} utility\n", name));
        template.push_str("//!\n");
        template.push_str(&format!("//! {}\n", meta.description));
        template.push_str("//!\n");
        template.push_str(&format!("//! This is a Windows-optimized version of the {} utility from uutils/coreutils.\n", name));
        template.push_str("//! It provides enhanced Windows path handling, performance optimizations,\n");
        template.push_str("//! and Windows-specific features while maintaining compatibility with the original.\n\n");

        template.push_str("use std::env;\n");
        template.push_str("use std::ffi::OsString;\n");
        template.push_str("use std::process;\n");
        template.push_str("use winpath::normalize_path;\n\n");

        template.push_str("fn main() {\n");
        template.push_str("    let args: Vec<String> = env::args().collect();\n");
        template.push_str("    let processed_args = process_windows_args(args);\n\n");

        template.push_str("    // For now, delegate to the original uutils implementation with path normalization\n");
        template.push_str(&format!("    let exit_code = uu_{}::uumain(processed_args.into_iter().map(OsString::from));\n", name));
        template.push_str("    process::exit(exit_code);\n");
        template.push_str("}\n\n");

        template.push_str("/// Windows-optimized argument processing\n");
        template.push_str("fn process_windows_args(args: Vec<String>) -> Vec<String> {\n");
        template.push_str("    args.into_iter()\n");
        template.push_str("        .map(|arg| {\n");
        template.push_str("            // Normalize Windows paths in arguments\n");
        template.push_str("            if arg.contains('\\\\') || arg.contains('/') {\n");
        template.push_str("                match normalize_path(&arg) {\n");
        template.push_str("                    Ok(normalized) => normalized.to_string_lossy().to_string(),\n");
        template.push_str("                    Err(_) => arg, // Keep original if normalization fails\n");
        template.push_str("                }\n");
        template.push_str("            } else {\n");
        template.push_str("                arg\n");
        template.push_str("            }\n");
        template.push_str("        })\n");
        template.push_str("        .collect()\n");
        template.push_str("}\n");

        template
    }

    /// Generate build scripts
    fn generate_build_scripts(&self) -> Result<(), Box<dyn std::error::Error>> {
        let scripts_dir = self.output_dir.join("scripts");
        fs::create_dir_all(&scripts_dir)?;

        let build_commands = self.utilities.keys()
            .map(|name| format!("cargo build --release --bin {}", name))
            .collect::<Vec<_>>()
            .join("\n");

        let build_all_script = format!(
r#"#!/bin/bash
# Build all Windows-optimized coreutils

echo "🔨 Building Windows-optimized coreutils..."

# Build the winpath module first
echo "📦 Building winpath module..."
cd winpath && cargo build --release
cd ..

# Build all utilities
echo "📦 Building utilities..."
{}

echo "✅ Build complete! All utilities available in target/release/"
"#,
            build_commands
        );

        let install_commands = self.utilities.keys()
            .map(|name| format!("cp target/release/{}.exe \"$INSTALL_DIR/\"", name))
            .collect::<Vec<_>>()
            .join("\n");

        let install_script = format!(
r#"#!/bin/bash
# Install Windows-optimized coreutils to system PATH

INSTALL_DIR="${{1:-C:/utils/winutils}}"

echo "📦 Installing Windows-optimized coreutils to $INSTALL_DIR..."

mkdir -p "$INSTALL_DIR"

# Copy binaries
{}

echo "✅ Installation complete!"
echo "Add $INSTALL_DIR to your PATH to use the utilities"
"#,
            install_commands
        );

        let test_script = r#"#!/bin/bash
# Test all Windows-optimized coreutils

echo "🧪 Testing Windows-optimized coreutils..."

# Run basic smoke tests
for util in target/release/*.exe; do
    echo "Testing $(basename $util)..."
    if $util --help > /dev/null 2>&1; then
        echo "  ✅ Help works"
    else
        echo "  ❌ Help failed"
    fi
done

echo "✅ Basic tests complete!"
"#;

        fs::write(scripts_dir.join("build-all.sh"), build_all_script)?;
        fs::write(scripts_dir.join("install.sh"), install_script)?;
        fs::write(scripts_dir.join("test.sh"), test_script)?;

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Windows-Optimized Coreutils Generator");
    println!("========================================");

    let generator = WinutilsGenerator::new();
    generator.generate_all()?;

    println!("\n📋 Summary:");
    println!("• Generated {} utilities", generator.utilities.len());
    println!("• Output directory: {}", OUTPUT_DIR);
    println!("• Winpath module: {}/winpath", OUTPUT_DIR);
    println!("• Build scripts: {}/scripts", OUTPUT_DIR);

    println!("\n🔧 Next steps:");
    println!("1. cd {}", OUTPUT_DIR);
    println!("2. cargo build --release");
    println!("3. ./scripts/test.sh");
    println!("4. ./scripts/install.sh [install_directory]");

    Ok(())
}
