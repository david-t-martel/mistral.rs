@echo off
REM Configure sccache environment for WinUtils build optimization

echo Setting up sccache environment...

REM Set environment variables for current session
set RUSTC_WRAPPER=sccache
set SCCACHE_DIR=T:\projects\coreutils\sccache-cache
set SCCACHE_CACHE_SIZE=10G

echo Environment configured:
echo   RUSTC_WRAPPER=%RUSTC_WRAPPER%
echo   SCCACHE_DIR=%SCCACHE_DIR%
echo   SCCACHE_CACHE_SIZE=%SCCACHE_CACHE_SIZE%

REM Ensure sccache server is running
sccache --stop-server 2>nul
sccache --start-server

echo.
echo sccache is ready! Current statistics:
sccache --show-stats

echo.
echo To use sccache for builds, run:
echo   cargo build --release
echo.
echo To monitor cache hits during build:
echo   sccache --show-stats
