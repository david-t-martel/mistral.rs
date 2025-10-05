# Temporary build script to work around sccache issues
$ErrorActionPreference = "Stop"

# Disable sccache
$env:RUSTC_WRAPPER = ""

# Set optimization flags
$env:RUSTFLAGS = "-C target-cpu=native -C opt-level=3 -C lto=fat"
$env:CARGO_PROFILE_RELEASE_LTO = "true"
$env:CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "1"

Write-Host "Building winpath (Phase 1 - Critical)..."
Push-Location "shared/winpath"
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Pop-Location
    Write-Host "ERROR: Failed to build winpath" -ForegroundColor Red
    exit 1
}
Pop-Location
Write-Host "SUCCESS: winpath built" -ForegroundColor Green

Write-Host "`nBuilding derive-utils (Phase 2)..."
$utils = @("where", "which", "tree")
foreach ($util in $utils) {
    Write-Host "  Building $util..."
    Push-Location "derive-utils/$util"
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  WARNING: $util failed to build" -ForegroundColor Yellow
    } else {
        Write-Host "  SUCCESS: $util built" -ForegroundColor Green
    }
    Pop-Location
}

Write-Host "`nBuilding coreutils (Phase 3 - this will take 2-3 minutes)..."
cargo build --release --workspace
if ($LASTEXITCODE -ne 0) {
    Write-Host "WARNING: Some utilities failed to build" -ForegroundColor Yellow
} else {
    Write-Host "SUCCESS: All coreutils built" -ForegroundColor Green
}

Write-Host "`nBuild complete!"
Write-Host "Binaries location: target\x86_64-pc-windows-msvc\release\"
