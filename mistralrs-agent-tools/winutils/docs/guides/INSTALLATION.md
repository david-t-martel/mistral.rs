# WinUtils Installation Guide

## Table of Contents

1. [Installation Methods](#installation-methods)
1. [Pre-built Binaries](#pre-built-binaries)
1. [Building from Source](#building-from-source)
1. [Platform-Specific Instructions](#platform-specific-instructions)
1. [Post-Installation Setup](#post-installation-setup)
1. [Verification](#verification)
1. [Uninstallation](#uninstallation)
1. [Troubleshooting](#troubleshooting)

## Installation Methods

### Quick Decision Guide

| Method             | Best For         | Time   | Technical Level |
| ------------------ | ---------------- | ------ | --------------- |
| Pre-built Binaries | End users        | 2 min  | Beginner        |
| PowerShell Script  | Windows users    | 5 min  | Intermediate    |
| Build from Source  | Developers       | 10 min | Advanced        |
| Docker Container   | Isolated testing | 3 min  | Intermediate    |

## Pre-built Binaries

### Windows Installer (Recommended)

Download and run the MSI installer:

```powershell
# Download installer
$version = "1.0.0"
$url = "https://github.com/david-t-martel/uutils-windows/releases/download/v$version/winutils-$version-x64.msi"
Invoke-WebRequest -Uri $url -OutFile "winutils-installer.msi"

# Run installer
Start-Process msiexec.exe -ArgumentList "/i", "winutils-installer.msi", "/quiet" -Wait

# Verify installation
where wu-ls
```

### Portable ZIP Archive

For portable installation without admin rights:

```powershell
# Download portable version
$version = "1.0.0"
$url = "https://github.com/david-t-martel/uutils-windows/releases/download/v$version/winutils-$version-portable.zip"
$dest = "$env:USERPROFILE\.local\bin"

# Create directory
New-Item -ItemType Directory -Force -Path $dest

# Download and extract
Invoke-WebRequest -Uri $url -OutFile "winutils.zip"
Expand-Archive -Path "winutils.zip" -DestinationPath $dest -Force

# Add to PATH for current session
$env:Path += ";$dest"

# Add to PATH permanently (user)
[Environment]::SetEnvironmentVariable(
    "Path",
    $env:Path,
    [EnvironmentVariableTarget]::User
)
```

### Scoop Package Manager

```powershell
# Install Scoop if not present
if (!(Get-Command scoop -ErrorAction SilentlyContinue)) {
    iwr -useb get.scoop.sh | iex
}

# Add bucket and install
scoop bucket add winutils https://github.com/david-t-martel/scoop-winutils
scoop install winutils

# Update
scoop update winutils
```

### Chocolatey Package Manager

```powershell
# Install Chocolatey if not present
if (!(Get-Command choco -ErrorAction SilentlyContinue)) {
    Set-ExecutionPolicy Bypass -Scope Process -Force
    [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
    iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
}

# Install WinUtils
choco install winutils

# Update
choco upgrade winutils
```

## Building from Source

### Prerequisites

#### 1. Rust Toolchain

```bash
# Windows (PowerShell)
Invoke-WebRequest -Uri https://win.rustup.rs -OutFile rustup-init.exe
.\rustup-init.exe -y
$env:Path += ";$env:USERPROFILE\.cargo\bin"

# Verify installation
rustc --version
cargo --version

# Add Windows target
rustup target add x86_64-pc-windows-msvc
```

#### 2. Visual Studio Build Tools

Download and install from: https://visualstudio.microsoft.com/downloads/

Required components:

- MSVC v143 - VS 2022 C++ x64/x86 build tools
- Windows 11 SDK (10.0.22000.0)
- C++ CMake tools for Windows

#### 3. GNU Make

Via Git Bash:

```bash
# If Git Bash is installed
# Make is included
make --version
```

Via MSYS2:

```bash
# Install MSYS2 from https://www.msys2.org/
pacman -S make
pacman -S mingw-w64-x86_64-toolchain
```

Via Chocolatey:

```powershell
choco install make
```

### Build Process

#### 🚨 CRITICAL: Use Make ONLY 🚨

```bash
# Clone repository
git clone https://github.com/david-t-martel/uutils-windows.git
cd winutils

# MANDATORY: Use Make commands ONLY
make clean          # Clean previous builds
make release        # Build optimized binaries
make test          # Run test suite
make install       # Install to ~/.local/bin

# NEVER use these (will fail):
# cargo build      ❌ FORBIDDEN
# cargo install    ❌ FORBIDDEN
```

### Build Options

```bash
# Debug build (with symbols)
make debug

# Release with maximum optimization
make release PROFILE=release-fast

# Specific utility only
make build-ls

# Cross-compilation
make release TARGET=x86_64-pc-windows-gnu

# Custom installation prefix
make install PREFIX=C:/custom/path
```

### Build Verification

```bash
# Validate all 77 utilities
make validate-all-77

# Run comprehensive tests
make test-all

# Performance benchmarks
make bench
```

## Platform-Specific Instructions

### Windows 10/11

```powershell
# Standard installation
$installPath = "C:\Program Files\WinUtils"
$binPath = "$installPath\bin"

# Create directory with admin rights
New-Item -ItemType Directory -Force -Path $installPath

# Download and extract
$url = "https://github.com/david-t-martel/uutils-windows/releases/latest/download/winutils-windows.zip"
Invoke-WebRequest -Uri $url -OutFile "$env:TEMP\winutils.zip"
Expand-Archive -Path "$env:TEMP\winutils.zip" -DestinationPath $installPath

# Add to system PATH (requires admin)
$systemPath = [Environment]::GetEnvironmentVariable("Path", "Machine")
[Environment]::SetEnvironmentVariable("Path", "$systemPath;$binPath", "Machine")

# Refresh environment
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
```

### Git Bash

```bash
# Install to Git Bash environment
cd /c/Users/$USER
mkdir -p .local/bin

# Download
curl -L https://github.com/david-t-martel/uutils-windows/releases/latest/download/winutils-gitbash.tar.gz -o winutils.tar.gz

# Extract
tar -xzf winutils.tar.gz -C .local/bin/

# Add to PATH in ~/.bashrc
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Create aliases
cat >> ~/.bashrc << 'EOF'
alias ls='wu-ls --color=auto'
alias grep='wu-grep --color=auto'
alias ll='wu-ls -la'
EOF
```

### WSL (Windows Subsystem for Linux)

```bash
# For WSL2 - Install Windows version and access via /mnt/c
cd /mnt/c/Users/$USER
mkdir -p .local/bin

# Download Windows binaries
wget https://github.com/david-t-martel/uutils-windows/releases/latest/download/winutils-windows.zip
unzip winutils-windows.zip -d .local/bin/

# Add to PATH
echo 'export PATH="/mnt/c/Users/$USER/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Test path normalization
wu-ls /mnt/c/Windows  # Should work correctly
```

### Cygwin

```bash
# Install in Cygwin environment
mkdir -p ~/bin
cd ~/bin

# Download Cygwin-compatible build
wget https://github.com/david-t-martel/uutils-windows/releases/latest/download/winutils-cygwin.tar.gz
tar -xzf winutils-cygwin.tar.gz

# Add to PATH
echo 'export PATH="$HOME/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Test
wu-ls /cygdrive/c/Windows
```

## Post-Installation Setup

### Environment Variables

```powershell
# Set up environment variables (PowerShell)
[Environment]::SetEnvironmentVariable("WINUTILS_HOME", "C:\Program Files\WinUtils", "User")
[Environment]::SetEnvironmentVariable("WINUTILS_CACHE", "1", "User")
[Environment]::SetEnvironmentVariable("WINUTILS_COLOR", "always", "User")

# Performance optimizations
[Environment]::SetEnvironmentVariable("WINUTILS_THREADS", "8", "User")
[Environment]::SetEnvironmentVariable("WINUTILS_BUFFER_SIZE", "65536", "User")
```

### Shell Integration

#### PowerShell Profile

```powershell
# Edit profile
notepad $PROFILE

# Add these lines
$env:Path += ";C:\Program Files\WinUtils\bin"

# Aliases
Set-Alias ls wu-ls
Set-Alias cat wu-cat
Set-Alias grep wu-grep
Set-Alias find wu-find
Set-Alias which wu-which

# Functions
function ll { wu-ls -la @args }
function la { wu-ls -a @args }

# Tab completion
Register-ArgumentCompleter -CommandName wu-* -ScriptBlock {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)
    Get-ChildItem -Path $wordToComplete* | ForEach-Object {
        [System.Management.Automation.CompletionResult]::new($_.FullName, $_.Name, 'ParameterValue', $_.Name)
    }
}
```

#### Windows Terminal

```json
// settings.json
{
  "profiles": {
    "defaults": {
      "env": {
        "WINUTILS_HOME": "C:\\Program Files\\WinUtils",
        "PATH": "C:\\Program Files\\WinUtils\\bin;%PATH%"
      }
    }
  }
}
```

#### VS Code

```json
// settings.json
{
  "terminal.integrated.env.windows": {
    "PATH": "C:\\Program Files\\WinUtils\\bin;${env:PATH}"
  },
  "terminal.integrated.defaultProfile.windows": "WinUtils Shell",
  "terminal.integrated.profiles.windows": {
    "WinUtils Shell": {
      "path": "C:\\Windows\\System32\\cmd.exe",
      "env": {
        "PATH": "C:\\Program Files\\WinUtils\\bin;${env:PATH}"
      }
    }
  }
}
```

## Verification

### Basic Verification

```bash
# Check installation
where wu-ls
wu-ls --version

# Test basic functionality
wu-ls C:\Windows
wu-cat C:\Windows\System32\drivers\etc\hosts
wu-which notepad

# Run validation suite
make validate-all-77
```

### Comprehensive Testing

```powershell
# PowerShell test script
$utilities = @(
    "arch", "base32", "base64", "basename", "cat", "cksum", "comm", "cp",
    "csplit", "cut", "date", "dd", "df", "dir", "dircolors", "dirname",
    "du", "echo", "env", "expand", "expr", "factor", "false", "fmt",
    "fold", "hashsum", "head", "hostname", "join", "link", "ln", "ls",
    "mkdir", "mktemp", "more", "mv", "nl", "nproc", "numfmt", "od",
    "paste", "pr", "printenv", "printf", "ptx", "pwd", "readlink",
    "realpath", "rm", "rmdir", "seq", "shred", "shuf", "sleep", "sort",
    "split", "sum", "sync", "tac", "tail", "tee", "test", "touch", "tr",
    "tree", "true", "truncate", "tsort", "unexpand", "uniq", "unlink",
    "vdir", "wc", "where", "which", "whoami", "yes"
)

$failed = @()
foreach ($util in $utilities) {
    $exe = "wu-$util"
    if (!(Get-Command $exe -ErrorAction SilentlyContinue)) {
        $failed += $util
    }
}

if ($failed.Count -eq 0) {
    Write-Host "✓ All 77 utilities installed successfully!" -ForegroundColor Green
} else {
    Write-Host "✗ Missing utilities: $($failed -join ', ')" -ForegroundColor Red
}
```

### Performance Verification

```bash
# Benchmark against native tools
hyperfine --warmup 3 'wu-ls C:\Windows' 'cmd /c dir C:\Windows'
hyperfine 'wu-sort large_file.txt' 'sort large_file.txt'
hyperfine 'wu-grep pattern file.txt' 'findstr pattern file.txt'
```

## Uninstallation

### MSI Installer

```powershell
# Uninstall via MSI
$app = Get-WmiObject -Class Win32_Product | Where-Object { $_.Name -match "WinUtils" }
if ($app) {
    $app.Uninstall()
}

# Or via msiexec
msiexec /x "{PRODUCT-GUID}" /quiet
```

### Manual Uninstallation

```powershell
# Remove binaries
Remove-Item -Path "C:\Program Files\WinUtils" -Recurse -Force

# Remove from PATH
$path = [Environment]::GetEnvironmentVariable("Path", "Machine")
$newPath = ($path.Split(';') | Where-Object { $_ -notlike '*WinUtils*' }) -join ';'
[Environment]::SetEnvironmentVariable("Path", $newPath, "Machine")

# Remove environment variables
[Environment]::SetEnvironmentVariable("WINUTILS_HOME", $null, "User")
[Environment]::SetEnvironmentVariable("WINUTILS_CACHE", $null, "User")
```

### Package Managers

```powershell
# Scoop
scoop uninstall winutils

# Chocolatey
choco uninstall winutils
```

## Troubleshooting

### Common Installation Issues

#### Issue: Make command not found

```bash
# Solution 1: Use Git Bash
# Make is included with Git Bash

# Solution 2: Install via Chocolatey
choco install make

# Solution 3: Use PowerShell build script
.\build.ps1
```

#### Issue: Visual Studio build tools missing

```powershell
# Install Build Tools via PowerShell
Invoke-WebRequest -Uri https://aka.ms/vs/17/release/vs_buildtools.exe -OutFile vs_buildtools.exe
Start-Process -FilePath .\vs_buildtools.exe -ArgumentList '--quiet', '--wait', '--add', 'Microsoft.VisualStudio.Workload.VCTools', '--includeRecommended' -Wait
```

#### Issue: Path not updated

```powershell
# Refresh environment in current session
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

# Or restart shell
exit
```

#### Issue: Permission denied during installation

```powershell
# Run as Administrator
Start-Process powershell -Verb RunAs

# Or install to user directory
$userBin = "$env:USERPROFILE\.local\bin"
# Install there instead
```

#### Issue: Utilities not working in Git Bash

```bash
# Ensure winpath.exe is present
ls ~/.local/bin/winpath.exe

# Rebuild if missing
make clean
make release
make install
```

### Getting Help

- **GitHub Issues**: https://github.com/david-t-martel/uutils-windows/issues
- **Email Support**: david.martel@auricleinc.com
- **Documentation**: See `/docs` directory
- **Discord**: [Community Discord Server]

______________________________________________________________________

*Installation Guide Version: 1.0.0*
*Last Updated: January 2025*
*Ensure proper installation for optimal performance*
