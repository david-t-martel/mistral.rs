@echo off
REM Simple build script to bypass all PATH and sccache issues

cd /d T:\projects\coreutils\winutils

echo ========================================
echo Starting WinUtils Build
echo ========================================
echo.

REM Disable sccache completely
set RUSTC_WRAPPER=

REM Set optimization flags
set RUSTFLAGS=-C target-cpu=native -C opt-level=3 -C lto=fat
set CARGO_PROFILE_RELEASE_LTO=true
set CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1

echo [Phase 1] Building winpath library (CRITICAL)...
cd shared\winpath
cargo build --release
if errorlevel 1 (
    echo ERROR: winpath build failed!
    cd ..\..
    exit /b 1
)
cd ..\..
echo SUCCESS: winpath built
echo.

echo [Phase 2] Building derive-utils...
cd derive-utils\where
cargo build --release
cd ..\..

cd derive-utils\which
cargo build --release
cd ..\..

cd derive-utils\tree
cargo build --release
cd ..\..
echo SUCCESS: derive-utils built
echo.

echo [Phase 3] Building all coreutils (this takes 2-3 minutes)...
cargo build --release --workspace
echo.

echo ========================================
echo Build Complete!
echo ========================================
echo Binaries are in: target\x86_64-pc-windows-msvc\release\
echo.

dir target\x86_64-pc-windows-msvc\release\*.exe | find /c ".exe"
echo .exe files built
