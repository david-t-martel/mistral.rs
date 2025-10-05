# Winutils Workspace Migration Script
# This script automates the reorganization of the winutils project structure
# to eliminate duplication and follow Rust workspace best practices

param(
    [switch]$DryRun = $false,
    [switch]$Backup = $true,
    [switch]$Force = $false
)

$ErrorActionPreference = "Stop"
$ProgressPreference = 'SilentlyContinue'

# Colors for output
function Write-Info { param($Message) Write-Host "[INFO] $Message" -ForegroundColor Cyan }
function Write-Success { param($Message) Write-Host "[SUCCESS] $Message" -ForegroundColor Green }
function Write-Warning { param($Message) Write-Host "[WARNING] $Message" -ForegroundColor Yellow }
function Write-Error { param($Message) Write-Host "[ERROR] $Message" -ForegroundColor Red }
function Write-Step { param($Message) Write-Host "`n==> $Message" -ForegroundColor Magenta }

$ProjectRoot = $PSScriptRoot
$BackupDir = Join-Path $ProjectRoot "backup_$(Get-Date -Format 'yyyyMMdd_HHmmss')"

Write-Host @"
╔════════════════════════════════════════════════════════════════╗
║          Winutils Workspace Migration Tool v1.0               ║
║                                                                ║
║  This tool will reorganize the project structure to:          ║
║  • Eliminate all duplications                                 ║
║  • Follow Rust workspace best practices                       ║
║  • Consolidate dependencies                                   ║
║  • Standardize build configurations                           ║
╚════════════════════════════════════════════════════════════════╝
"@ -ForegroundColor Cyan

if ($DryRun) {
    Write-Warning "DRY RUN MODE - No changes will be made"
}

# Step 1: Backup current structure
if ($Backup -and -not $DryRun) {
    Write-Step "Creating backup of current structure..."

    $itemsToBackup = @(
        "Cargo.toml",
        "Cargo.lock",
        "shared",
        "derive-utils",
        "coreutils",
        "where",
        "which",
        "Makefile*",
        "*.md"
    )

    New-Item -ItemType Directory -Path $BackupDir -Force | Out-Null

    foreach ($item in $itemsToBackup) {
        $sourcePath = Join-Path $ProjectRoot $item
        if (Test-Path $sourcePath) {
            Write-Info "Backing up $item..."
            Copy-Item -Path $sourcePath -Destination $BackupDir -Recurse -Force
        }
    }

    Write-Success "Backup created at: $BackupDir"
}

# Step 2: Create new directory structure
Write-Step "Creating optimized directory structure..."

$newDirectories = @(
    "crates\libs\winpath",
    "crates\libs\winutils-core",
    "crates\libs\common",
    "crates\utils\standard",
    "crates\utils\extended",
    "crates\utils\extended\wrappers",
    "crates\tools",
    "docs\architecture",
    "docs\api",
    "docs\guides",
    "scripts\ci",
    "tests\integration",
    ".cargo"
)

foreach ($dir in $newDirectories) {
    $fullPath = Join-Path $ProjectRoot $dir
    if (-not $DryRun) {
        New-Item -ItemType Directory -Path $fullPath -Force | Out-Null
    }
    Write-Info "Created: $dir"
}

# Step 3: Migrate shared libraries
Write-Step "Migrating shared libraries..."

$libraryMappings = @{
    "shared\winpath" = "crates\libs\winpath"
    "shared\winutils-core" = "crates\libs\winutils-core"
}

foreach ($mapping in $libraryMappings.GetEnumerator()) {
    $source = Join-Path $ProjectRoot $mapping.Key
    $dest = Join-Path $ProjectRoot $mapping.Value

    if (Test-Path $source) {
        Write-Info "Moving $($mapping.Key) -> $($mapping.Value)"
        if (-not $DryRun) {
            Copy-Item -Path "$source\*" -Destination $dest -Recurse -Force
        }
    }
}

# Step 4: Identify and consolidate duplicate utilities
Write-Step "Identifying and consolidating duplicate utilities..."

# Check for duplicate 'where' implementations
$wherePaths = @(
    "derive-utils\where",
    "where"
)

$duplicates = @()
foreach ($path in $wherePaths) {
    if (Test-Path (Join-Path $ProjectRoot $path)) {
        $duplicates += $path
    }
}

if ($duplicates.Count -gt 1) {
    Write-Warning "Found duplicate 'where' implementations:"
    $duplicates | ForEach-Object { Write-Host "  - $_" -ForegroundColor Yellow }

    # Use the derive-utils version as canonical
    if (-not $DryRun) {
        $canonical = Join-Path $ProjectRoot "derive-utils\where"
        $destination = Join-Path $ProjectRoot "crates\utils\extended\where"
        Copy-Item -Path "$canonical\*" -Destination $destination -Recurse -Force
        Write-Success "Consolidated 'where' utility to: crates\utils\extended\where"
    }
}

# Step 5: Migrate standard coreutils
Write-Step "Migrating standard coreutils..."

$coreutilsPath = Join-Path $ProjectRoot "coreutils\src"
if (Test-Path $coreutilsPath) {
    $utils = Get-ChildItem -Path $coreutilsPath -Directory

    foreach ($util in $utils) {
        $destPath = Join-Path $ProjectRoot "crates\utils\standard\$($util.Name)"
        Write-Info "Moving $($util.Name) to standard utilities..."

        if (-not $DryRun) {
            Copy-Item -Path $util.FullName -Destination $destPath -Recurse -Force
        }
    }

    Write-Success "Migrated $($utils.Count) standard utilities"
}

# Step 6: Migrate extended utilities
Write-Step "Migrating extended utilities and wrappers..."

$extendedMappings = @{
    "derive-utils\which" = "crates\utils\extended\which"
    "derive-utils\tree" = "crates\utils\extended\tree"
    "derive-utils\find-wrapper" = "crates\utils\extended\wrappers\find"
    "derive-utils\grep-wrapper" = "crates\utils\extended\wrappers\grep"
    "derive-utils\cmd-wrapper" = "crates\utils\extended\wrappers\cmd"
    "derive-utils\pwsh-wrapper" = "crates\utils\extended\wrappers\pwsh"
    "derive-utils\bash-wrapper" = "crates\utils\extended\wrappers\bash"
}

foreach ($mapping in $extendedMappings.GetEnumerator()) {
    $source = Join-Path $ProjectRoot $mapping.Key
    $dest = Join-Path $ProjectRoot $mapping.Value

    if (Test-Path $source) {
        Write-Info "Moving $($mapping.Key) -> $($mapping.Value)"
        if (-not $DryRun) {
            New-Item -ItemType Directory -Path $dest -Force | Out-Null
            Copy-Item -Path "$source\*" -Destination $dest -Recurse -Force
        }
    }
}

# Step 7: Create new workspace Cargo.toml
Write-Step "Creating optimized workspace Cargo.toml..."

$workspaceToml = @'
[workspace]
resolver = "2"
members = [
    # Libraries (build order matters)
    "crates/libs/winpath",
    "crates/libs/common",
    "crates/libs/winutils-core",

    # Standard utilities
    "crates/utils/standard/*",

    # Extended utilities
    "crates/utils/extended/*",
    "crates/utils/extended/wrappers/*",

    # Tools
    "crates/tools/*",
]

exclude = ["target", "tests/fixtures", "backup_*"]

[workspace.package]
version = "0.2.0"
authors = ["David Martel <david.martel@auricleinc.com>"]
edition = "2021"
rust-version = "1.85.0"
license = "MIT OR Apache-2.0"
repository = "https://github.com/david-t-martel/winutils"
keywords = ["windows", "utilities", "coreutils", "rust"]
categories = ["command-line-utilities", "os::windows-apis"]

[workspace.dependencies]
# Core dependencies
anyhow = "1.0.95"
clap = { version = "4.5.32", features = ["derive", "env", "wrap_help", "color"] }
serde = { version = "1.0.217", features = ["derive"] }
serde_json = "1.0.134"
thiserror = "2.0.9"
tokio = { version = "1.42.0", features = ["full"] }

# Internal libraries
winpath = { path = "crates/libs/winpath" }
winutils-core = { path = "crates/libs/winutils-core" }
common = { path = "crates/libs/common" }

# Windows-specific
windows = { version = "0.60.0", features = [
    "Win32_Foundation",
    "Win32_Storage_FileSystem",
    "Win32_System_Console",
    "Win32_System_SystemInformation",
    "Win32_Security"
]}
windows-sys = { version = "0.60.0", features = [
    "Win32_Foundation",
    "Win32_Storage_FileSystem",
    "Win32_System_Console",
    "Win32_System_SystemInformation",
    "Win32_Security"
]}
winapi-util = "0.1.9"

# Path handling
dunce = "1.0.5"
path-slash = "0.2.1"
normalize-path = "0.2.1"

# Performance
rayon = "1.10.0"
crossbeam-channel = "0.5.14"
dashmap = "6.1.0"
lru = "0.12.5"
ahash = "0.8.11"
parking_lot = "0.12.3"
once_cell = "1.20.2"

# Common utilities
regex = "1.11.1"
glob = "0.3.2"
walkdir = "2.5.0"
dirs = "5.0.1"
which = "7.0.1"
termcolor = "1.4.1"
indicatif = "0.17.10"
humansize = "2.1.3"
chrono = "0.4.39"

[workspace.lints.rust]
unsafe_code = "warn"
missing_docs = "warn"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
missing_errors_doc = "allow"
must_use_candidate = "allow"

[profile.release]
lto = "fat"
codegen-units = 1
opt-level = 3
strip = true
panic = "abort"
overflow-checks = false

[profile.release-windows]
inherits = "release"
# Windows-specific optimizations
incremental = false

[profile.dev]
opt-level = 0
debug = true
overflow-checks = true
incremental = true

[profile.test]
inherits = "dev"
opt-level = 1

[profile.bench]
inherits = "release"
debug = true
'@

if (-not $DryRun) {
    $workspaceToml | Out-File -FilePath (Join-Path $ProjectRoot "Cargo.toml") -Encoding UTF8
    Write-Success "Created optimized workspace Cargo.toml"
}

# Step 8: Create cargo config
Write-Step "Creating .cargo/config.toml..."

$cargoConfig = @'
[build]
target-dir = "target"
incremental = true
jobs = 8

[target.x86_64-pc-windows-msvc]
rustflags = [
    "-C", "link-arg=/STACK:8388608",
    "-C", "target-cpu=native"
]

[profile.release]
strip = true

[net]
git-fetch-with-cli = true

[registries.crates-io]
protocol = "sparse"
'@

if (-not $DryRun) {
    $cargoConfig | Out-File -FilePath (Join-Path $ProjectRoot ".cargo\config.toml") -Encoding UTF8
    Write-Success "Created .cargo/config.toml"
}

# Step 9: Update all Cargo.toml files to use workspace dependencies
Write-Step "Updating Cargo.toml files to use workspace dependencies..."

$tomlFiles = Get-ChildItem -Path (Join-Path $ProjectRoot "crates") -Filter "Cargo.toml" -Recurse -ErrorAction SilentlyContinue

foreach ($tomlFile in $tomlFiles) {
    Write-Info "Updating: $($tomlFile.FullName)"

    if (-not $DryRun) {
        $content = Get-Content $tomlFile.FullName -Raw

        # Replace version specifications with workspace references
        $replacements = @(
            ('anyhow = "[\d\.]+"', 'anyhow.workspace = true'),
            ('clap = \{[^}]+\}', 'clap.workspace = true'),
            ('serde = \{[^}]+\}', 'serde.workspace = true'),
            ('tokio = \{[^}]+\}', 'tokio.workspace = true'),
            ('rayon = "[\d\.]+"', 'rayon.workspace = true'),
            ('regex = "[\d\.]+"', 'regex.workspace = true')
        )

        foreach ($replacement in $replacements) {
            $content = $content -replace $replacement[0], $replacement[1]
        }

        # Add workspace package inheritance
        if ($content -notmatch '\[package\][\s\S]*?version\.workspace') {
            $content = $content -replace '\[package\]', "[package]`nversion.workspace = true`nauthors.workspace = true`nedition.workspace = true`nlicense.workspace = true"
        }

        $content | Out-File -FilePath $tomlFile.FullName -Encoding UTF8
    }
}

# Step 10: Consolidate documentation
Write-Step "Consolidating documentation..."

$docFiles = Get-ChildItem -Path $ProjectRoot -Filter "*.md" -File
foreach ($doc in $docFiles) {
    $category = switch -Regex ($doc.Name) {
        "ARCHITECTURE|BUILD" { "architecture" }
        "README|GUIDE|INTEGRATION" { "guides" }
        "API|REFERENCE" { "api" }
        default { "" }
    }

    if ($category) {
        $destPath = Join-Path $ProjectRoot "docs\$category\$($doc.Name)"
        Write-Info "Moving $($doc.Name) to docs\$category"

        if (-not $DryRun) {
            Copy-Item -Path $doc.FullName -Destination $destPath -Force
        }
    }
}

# Step 11: Create build scripts
Write-Step "Creating standardized build scripts..."

$buildScript = @'
# Winutils Build Script
param(
    [string]$Profile = "release",
    [switch]$Clean = $false,
    [switch]$Test = $false,
    [switch]$Doc = $false,
    [string]$Package = ""
)

$ErrorActionPreference = "Stop"

if ($Clean) {
    Write-Host "Cleaning build artifacts..." -ForegroundColor Yellow
    cargo clean
}

if ($Package) {
    Write-Host "Building package: $Package" -ForegroundColor Cyan
    cargo build --package $Package --profile $Profile
} else {
    Write-Host "Building all packages..." -ForegroundColor Cyan
    cargo build --workspace --profile $Profile
}

if ($Test) {
    Write-Host "Running tests..." -ForegroundColor Cyan
    cargo test --workspace
}

if ($Doc) {
    Write-Host "Generating documentation..." -ForegroundColor Cyan
    cargo doc --workspace --no-deps
}

Write-Host "Build completed successfully!" -ForegroundColor Green
'@

if (-not $DryRun) {
    $buildScript | Out-File -FilePath (Join-Path $ProjectRoot "scripts\build.ps1") -Encoding UTF8
    Write-Success "Created scripts\build.ps1"
}

# Step 12: Clean up old structure
if (-not $DryRun -and $Force) {
    Write-Step "Cleaning up old structure..."

    $itemsToRemove = @(
        "Makefile.old",
        "Makefile-optimized",
        "build.log",
        "build-output.log",
        "build-output2.log"
    )

    foreach ($item in $itemsToRemove) {
        $path = Join-Path $ProjectRoot $item
        if (Test-Path $path) {
            Remove-Item -Path $path -Force
            Write-Info "Removed: $item"
        }
    }
}

# Final summary
Write-Host "`n" -NoNewline
Write-Host "╔════════════════════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║                  Migration Complete!                          ║" -ForegroundColor Green
Write-Host "╚════════════════════════════════════════════════════════════════╝" -ForegroundColor Green

Write-Host @"

Next Steps:
1. Review the new structure in crates/
2. Run 'cargo build --workspace' to verify everything compiles
3. Run 'cargo test --workspace' to ensure tests pass
4. Update CI/CD pipelines to use new structure
5. Delete backup directory once verified: $BackupDir

New Build Commands:
  - Build all: cargo build --workspace --release
  - Build specific: cargo build -p <package-name> --release
  - Run tests: cargo test --workspace
  - Generate docs: cargo doc --workspace --no-deps

"@ -ForegroundColor Cyan

if ($DryRun) {
    Write-Warning "`nThis was a DRY RUN - no changes were made. Run without -DryRun to apply changes."
}
