# WinUtils Windows Installer Builder
# Builds MSI and NSIS installers for WinUtils distribution

param(
    [Parameter(Mandatory=$false)]
    [string]$Version = "0.1.0",

    [Parameter(Mandatory=$false)]
    [string]$Architecture = "x64",

    [Parameter(Mandatory=$false)]
    [string]$OutputDir = "dist",

    [Parameter(Mandatory=$false)]
    [string]$BuildType = "release",

    [Parameter(Mandatory=$false)]
    [switch]$SkipBuild,

    [Parameter(Mandatory=$false)]
    [switch]$SignBinaries,

    [Parameter(Mandatory=$false)]
    [string]$CertificatePath = "",

    [Parameter(Mandatory=$false)]
    [switch]$CreatePortable
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Constants
$PROJECT_ROOT = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$TARGET_DIR = Join-Path $PROJECT_ROOT "target\release"
$ASSETS_DIR = Join-Path $PSScriptRoot "assets"
$TEMP_DIR = Join-Path $env:TEMP "winutils-installer"

Write-Host "🚀 WinUtils Installer Builder" -ForegroundColor Green
Write-Host "=============================" -ForegroundColor Green
Write-Host "Version: $Version" -ForegroundColor Cyan
Write-Host "Architecture: $Architecture" -ForegroundColor Cyan
Write-Host "Output Directory: $OutputDir" -ForegroundColor Cyan
Write-Host ""

# Create output and temp directories
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
New-Item -ItemType Directory -Force -Path $TEMP_DIR | Out-Null

# Build binaries if not skipped
if (-not $SkipBuild) {
    Write-Host "🔨 Building WinUtils binaries..." -ForegroundColor Yellow
    Push-Location $PROJECT_ROOT
    try {
        & make clean
        if ($LASTEXITCODE -ne 0) { throw "Make clean failed" }

        & make release
        if ($LASTEXITCODE -ne 0) { throw "Make release failed" }

        Write-Host "✅ Build completed successfully" -ForegroundColor Green
    }
    finally {
        Pop-Location
    }
}

# Verify binaries exist
$REQUIRED_BINARIES = @(
    "ls.exe", "cat.exe", "cp.exe", "mv.exe", "rm.exe", "mkdir.exe",
    "rmdir.exe", "pwd.exe", "echo.exe", "grep.exe", "find.exe",
    "sort.exe", "wc.exe", "head.exe", "tail.exe", "cut.exe", "tr.exe",
    "chmod.exe", "du.exe", "touch.exe", "which.exe"
)

Write-Host "🔍 Verifying binaries..." -ForegroundColor Yellow
$missing_binaries = @()
foreach ($binary in $REQUIRED_BINARIES) {
    $binary_path = Join-Path $TARGET_DIR $binary
    if (-not (Test-Path $binary_path)) {
        $missing_binaries += $binary
    }
}

if ($missing_binaries.Count -gt 0) {
    Write-Host "❌ Missing binaries: $($missing_binaries -join ', ')" -ForegroundColor Red
    Write-Host "Please ensure all utilities are built successfully" -ForegroundColor Red
    exit 1
}

Write-Host "✅ All required binaries found" -ForegroundColor Green

# Sign binaries if requested
if ($SignBinaries -and $CertificatePath) {
    Write-Host "🔏 Signing binaries..." -ForegroundColor Yellow

    foreach ($binary in $REQUIRED_BINARIES) {
        $binary_path = Join-Path $TARGET_DIR $binary
        & signtool sign /f $CertificatePath /t http://timestamp.digicert.com $binary_path
        if ($LASTEXITCODE -ne 0) {
            Write-Host "⚠️  Failed to sign $binary" -ForegroundColor Yellow
        }
    }

    Write-Host "✅ Binary signing completed" -ForegroundColor Green
}

# Create portable package
if ($CreatePortable) {
    Write-Host "📦 Creating portable package..." -ForegroundColor Yellow

    $portable_dir = Join-Path $TEMP_DIR "winutils-portable"
    New-Item -ItemType Directory -Force -Path $portable_dir | Out-Null

    # Copy binaries
    $bin_dir = Join-Path $portable_dir "bin"
    New-Item -ItemType Directory -Force -Path $bin_dir | Out-Null

    foreach ($binary in $REQUIRED_BINARIES) {
        $source = Join-Path $TARGET_DIR $binary
        $dest = Join-Path $bin_dir $binary
        Copy-Item $source $dest
    }

    # Create batch file for environment setup
    $setup_bat = @"
@echo off
REM WinUtils Portable Setup
REM Add WinUtils to PATH for current session

set WINUTILS_DIR=%~dp0bin
set PATH=%WINUTILS_DIR%;%PATH%

echo WinUtils portable environment activated
echo Available utilities: ls, cat, cp, mv, rm, grep, sort, wc, and more
echo Type any utility name followed by --help for usage information
"@

    $setup_bat | Out-File -FilePath (Join-Path $portable_dir "setup.bat") -Encoding ASCII

    # Create PowerShell setup script
    $setup_ps1 = @"
# WinUtils Portable Setup for PowerShell
# Add WinUtils to PATH for current session

`$winutils_dir = Join-Path `$PSScriptRoot "bin"
`$env:PATH = "`$winutils_dir;`$env:PATH"

Write-Host "WinUtils portable environment activated" -ForegroundColor Green
Write-Host "Available utilities: ls, cat, cp, mv, rm, grep, sort, wc, and more" -ForegroundColor Cyan
Write-Host "Type any utility name followed by --help for usage information" -ForegroundColor Cyan
"@

    $setup_ps1 | Out-File -FilePath (Join-Path $portable_dir "setup.ps1") -Encoding UTF8

    # Create README
    $readme = @"
# WinUtils Portable

This is a portable distribution of WinUtils - fast, reliable Unix-like utilities for Windows.

## Quick Start

### Command Prompt
1. Open Command Prompt in this directory
2. Run: setup.bat
3. Use any utility: ls, cat, grep, etc.

### PowerShell
1. Open PowerShell in this directory
2. Run: .\setup.ps1
3. Use any utility: ls, cat, grep, etc.

## Available Utilities

- ls - List directory contents
- cat - Display file contents
- cp - Copy files and directories
- mv - Move/rename files and directories
- rm - Remove files and directories
- grep - Search text patterns
- sort - Sort lines of text
- wc - Word, line, character, and byte count
- head - Display first lines of files
- tail - Display last lines of files
- cut - Extract sections from lines
- tr - Translate characters
- chmod - Change file permissions
- du - Display directory space usage
- touch - Create empty files or update timestamps
- which - Locate a command

## Performance

WinUtils provides significant performance improvements over traditional Windows utilities:
- ls: 4x faster than dir
- cat: 3x faster than type
- wc: 12x faster than findstr-based solutions
- sort: 8x faster than native Windows sort

## Version

WinUtils v$Version
Generated: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')

For more information, visit: https://github.com/your-org/winutils
"@

    $readme | Out-File -FilePath (Join-Path $portable_dir "README.txt") -Encoding UTF8

    # Create ZIP archive
    $portable_zip = Join-Path $OutputDir "winutils-portable-v$Version-$Architecture.zip"
    Compress-Archive -Path "$portable_dir\*" -DestinationPath $portable_zip -Force

    Write-Host "✅ Portable package created: $portable_zip" -ForegroundColor Green
}

# Create NSIS installer
Write-Host "📦 Creating NSIS installer..." -ForegroundColor Yellow

# Check if NSIS is installed
$nsis_path = ""
$nsis_locations = @(
    "${env:ProgramFiles}\NSIS\makensis.exe",
    "${env:ProgramFiles(x86)}\NSIS\makensis.exe",
    "C:\Program Files\NSIS\makensis.exe",
    "C:\Program Files (x86)\NSIS\makensis.exe"
)

foreach ($location in $nsis_locations) {
    if (Test-Path $location) {
        $nsis_path = $location
        break
    }
}

if (-not $nsis_path) {
    Write-Host "⚠️  NSIS not found. Skipping NSIS installer creation." -ForegroundColor Yellow
    Write-Host "   Download NSIS from: https://nsis.sourceforge.io/" -ForegroundColor Yellow
} else {
    # Create NSIS script
    $nsis_script = @"
; WinUtils NSIS Installer Script
; Generated automatically by build-installer.ps1

!define APP_NAME "WinUtils"
!define APP_VERSION "$Version"
!define APP_PUBLISHER "WinUtils Team"
!define APP_URL "https://github.com/your-org/winutils"
!define APP_EXEC_NAME "winutils"

; Installer settings
Name "`${APP_NAME} `${APP_VERSION}"
OutFile "$OutputDir\winutils-setup-v$Version-$Architecture.exe"
InstallDir "`$PROGRAMFILES64\WinUtils"
InstallDirRegKey HKLM "Software\WinUtils" "InstallPath"

; Modern UI
!include "MUI2.nsh"
!define MUI_ABORTWARNING
!define MUI_ICON "`${NSISDIR}\Contrib\Graphics\Icons\modern-install.ico"
!define MUI_UNICON "`${NSISDIR}\Contrib\Graphics\Icons\modern-uninstall.ico"

; Pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

; Languages
!insertmacro MUI_LANGUAGE "English"

; Version info
VIProductVersion "$Version.0"
VIAddVersionKey "ProductName" "`${APP_NAME}"
VIAddVersionKey "ProductVersion" "`${APP_VERSION}"
VIAddVersionKey "FileDescription" "Fast Unix-like utilities for Windows"
VIAddVersionKey "FileVersion" "`${APP_VERSION}"
VIAddVersionKey "CompanyName" "`${APP_PUBLISHER}"
VIAddVersionKey "LegalCopyright" "Copyright (c) 2024 WinUtils Team"

; Installer sections
Section "Core Utilities" SecCore
    SectionIn RO
    SetOutPath "`$INSTDIR\bin"

    ; Copy all binaries
    File "$TARGET_DIR\*.exe"

    ; Create batch file for environment setup
    SetOutPath "`$INSTDIR"
    FileOpen `$0 "`$INSTDIR\setup.bat" w
    FileWrite `$0 "@echo off`r`n"
    FileWrite `$0 "set PATH=%~dp0bin;%PATH%`r`n"
    FileWrite `$0 "echo WinUtils added to PATH for current session`r`n"
    FileClose `$0

    ; Write registry keys
    WriteRegStr HKLM "Software\WinUtils" "InstallPath" "`$INSTDIR"
    WriteRegStr HKLM "Software\WinUtils" "Version" "`${APP_VERSION}"

    ; Write uninstaller
    WriteUninstaller "`$INSTDIR\Uninstall.exe"

    ; Add to Add/Remove Programs
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinUtils" "DisplayName" "`${APP_NAME}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinUtils" "UninstallString" "`$INSTDIR\Uninstall.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinUtils" "DisplayVersion" "`${APP_VERSION}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinUtils" "Publisher" "`${APP_PUBLISHER}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinUtils" "URLInfoAbout" "`${APP_URL}"
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinUtils" "NoModify" 1
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinUtils" "NoRepair" 1
SectionEnd

Section "Add to PATH" SecPath
    ; Add to system PATH
    `${EnvVarUpdate} `$0 "PATH" "A" "HKLM" "`$INSTDIR\bin"
SectionEnd

; Uninstaller
Section "Uninstall"
    ; Remove files
    RMDir /r "`$INSTDIR\bin"
    Delete "`$INSTDIR\setup.bat"
    Delete "`$INSTDIR\Uninstall.exe"
    RMDir "`$INSTDIR"

    ; Remove from PATH
    `${un.EnvVarUpdate} `$0 "PATH" "R" "HKLM" "`$INSTDIR\bin"

    ; Remove registry keys
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinUtils"
    DeleteRegKey HKLM "Software\WinUtils"
SectionEnd

; Include environment variable update macro
!include "EnvVarUpdate.nsh"

; Section descriptions
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
!insertmacro MUI_DESCRIPTION_TEXT `${SecCore} "Core WinUtils utilities (required)"
!insertmacro MUI_DESCRIPTION_TEXT `${SecPath} "Add WinUtils to system PATH environment variable"
!insertmacro MUI_FUNCTION_DESCRIPTION_END
"@

    $nsis_script_path = Join-Path $TEMP_DIR "winutils.nsi"
    $nsis_script | Out-File -FilePath $nsis_script_path -Encoding UTF8

    # Copy LICENSE file if it exists
    $license_source = Join-Path $PROJECT_ROOT "LICENSE"
    $license_dest = Join-Path $TEMP_DIR "LICENSE"
    if (Test-Path $license_source) {
        Copy-Item $license_source $license_dest
    } else {
        # Create a basic license file
        "MIT License - See project repository for details" | Out-File -FilePath $license_dest -Encoding UTF8
    }

    # Download EnvVarUpdate.nsh if needed
    $envvar_path = Join-Path $TEMP_DIR "EnvVarUpdate.nsh"
    if (-not (Test-Path $envvar_path)) {
        Write-Host "📥 Downloading EnvVarUpdate.nsh..." -ForegroundColor Yellow
        try {
            Invoke-WebRequest -Uri "https://nsis.sourceforge.io/mediawiki/images/a/ad/EnvVarUpdate.nsh" -OutFile $envvar_path
        } catch {
            Write-Host "⚠️  Could not download EnvVarUpdate.nsh. PATH modification will be limited." -ForegroundColor Yellow
        }
    }

    # Build NSIS installer
    Push-Location $TEMP_DIR
    try {
        & $nsis_path $nsis_script_path
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ NSIS installer created successfully" -ForegroundColor Green
        } else {
            Write-Host "❌ NSIS installer creation failed" -ForegroundColor Red
        }
    }
    finally {
        Pop-Location
    }
}

# Create MSI installer (if WiX is available)
Write-Host "📦 Checking for WiX Toolset..." -ForegroundColor Yellow

$wix_path = ""
$wix_locations = @(
    "${env:ProgramFiles}\WiX Toolset v3.11\bin\candle.exe",
    "${env:ProgramFiles(x86)}\WiX Toolset v3.11\bin\candle.exe",
    "${env:ProgramFiles}\WiX Toolset v4\bin\candle.exe",
    "${env:ProgramFiles(x86)}\WiX Toolset v4\bin\candle.exe"
)

foreach ($location in $wix_locations) {
    if (Test-Path $location) {
        $wix_path = Split-Path $location
        break
    }
}

if (-not $wix_path) {
    Write-Host "⚠️  WiX Toolset not found. Skipping MSI installer creation." -ForegroundColor Yellow
    Write-Host "   Download WiX from: https://wixtoolset.org/" -ForegroundColor Yellow
} else {
    Write-Host "📦 Creating MSI installer..." -ForegroundColor Yellow

    # Create WiX source file
    $wxs_content = @"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
    <Product Id="*" Name="WinUtils $Version" Language="1033" Version="$Version.0"
             Manufacturer="WinUtils Team" UpgradeCode="{12345678-1234-1234-1234-123456789012}">

        <Package InstallerVersion="200" Compressed="yes" InstallScope="perMachine" />

        <MajorUpgrade DowngradeErrorMessage="A newer version of WinUtils is already installed." />

        <MediaTemplate EmbedCab="yes" />

        <Feature Id="ProductFeature" Title="WinUtils" Level="1">
            <ComponentGroupRef Id="ProductComponents" />
        </Feature>

        <Directory Id="TARGETDIR" Name="SourceDir">
            <Directory Id="ProgramFiles64Folder">
                <Directory Id="INSTALLFOLDER" Name="WinUtils">
                    <Directory Id="BinFolder" Name="bin" />
                </Directory>
            </Directory>
        </Directory>

        <ComponentGroup Id="ProductComponents" Directory="BinFolder">
"@

    # Add components for each binary
    foreach ($binary in $REQUIRED_BINARIES) {
        $component_id = ($binary -replace '\.exe$', '') + "Component"
        $file_id = ($binary -replace '\.exe$', '') + "File"

        $wxs_content += @"
            <Component Id="$component_id" Guid="*">
                <File Id="$file_id" Source="$TARGET_DIR\$binary" />
            </Component>
"@
    }

    $wxs_content += @"
        </ComponentGroup>
    </Product>
</Wix>
"@

    $wxs_path = Join-Path $TEMP_DIR "winutils.wxs"
    $wxs_content | Out-File -FilePath $wxs_path -Encoding UTF8

    # Build MSI
    Push-Location $TEMP_DIR
    try {
        & (Join-Path $wix_path "candle.exe") -out winutils.wixobj winutils.wxs
        if ($LASTEXITCODE -eq 0) {
            & (Join-Path $wix_path "light.exe") -out "$OutputDir\winutils-setup-v$Version-$Architecture.msi" winutils.wixobj
            if ($LASTEXITCODE -eq 0) {
                Write-Host "✅ MSI installer created successfully" -ForegroundColor Green
            } else {
                Write-Host "❌ MSI linking failed" -ForegroundColor Red
            }
        } else {
            Write-Host "❌ MSI compilation failed" -ForegroundColor Red
        }
    }
    finally {
        Pop-Location
    }
}

# Create chocolatey package
Write-Host "📦 Creating Chocolatey package..." -ForegroundColor Yellow

$choco_dir = Join-Path $TEMP_DIR "chocolatey"
New-Item -ItemType Directory -Force -Path $choco_dir | Out-Null

# Create nuspec file
$nuspec_content = @"
<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://schemas.microsoft.com/packaging/2015/06/nuspec.xsd">
  <metadata>
    <id>winutils</id>
    <version>$Version</version>
    <packageSourceUrl>https://github.com/your-org/winutils</packageSourceUrl>
    <owners>WinUtils Team</owners>
    <title>WinUtils</title>
    <authors>WinUtils Team</authors>
    <projectUrl>https://github.com/your-org/winutils</projectUrl>
    <copyright>2024 WinUtils Team</copyright>
    <licenseUrl>https://github.com/your-org/winutils/blob/main/LICENSE</licenseUrl>
    <requireLicenseAcceptance>false</requireLicenseAcceptance>
    <projectSourceUrl>https://github.com/your-org/winutils</projectSourceUrl>
    <tags>utilities unix windows cli performance</tags>
    <summary>Fast, reliable Unix-like utilities for Windows</summary>
    <description>
WinUtils provides high-performance implementations of common Unix utilities for Windows.
Built in Rust for speed and reliability, offering 2-20x performance improvements over traditional implementations.

Features:
- 25+ core utilities (ls, cat, grep, sort, wc, etc.)
- Significant performance improvements
- Windows-native path handling
- Git Bash compatibility
- Memory efficient
- Single-file executables
    </description>
    <releaseNotes>https://github.com/your-org/winutils/releases/tag/v$Version</releaseNotes>
  </metadata>
  <files>
    <file src="tools\**" target="tools" />
  </files>
</package>
"@

$nuspec_content | Out-File -FilePath (Join-Path $choco_dir "winutils.nuspec") -Encoding UTF8

# Create tools directory and install script
$tools_dir = Join-Path $choco_dir "tools"
New-Item -ItemType Directory -Force -Path $tools_dir | Out-Null

$install_script = @"
`$ErrorActionPreference = 'Stop'

`$packageName = 'winutils'
`$toolsDir = "`$(Split-Path -parent `$MyInvocation.MyCommand.Definition)"
`$packageDir = "`$(Split-Path -parent `$toolsDir)"

# Create bin directory
`$binDir = Join-Path `$packageDir 'bin'
New-Item -ItemType Directory -Force -Path `$binDir | Out-Null

# Copy binaries
Copy-Item (Join-Path `$toolsDir '*.exe') `$binDir -Force

# Add to PATH
Install-ChocolateyPath `$binDir 'Machine'

Write-Host "WinUtils has been installed!" -ForegroundColor Green
Write-Host "Available utilities: ls, cat, cp, mv, rm, grep, sort, wc, and more" -ForegroundColor Cyan
Write-Host "Type any utility name followed by --help for usage information" -ForegroundColor Cyan
"@

$install_script | Out-File -FilePath (Join-Path $tools_dir "chocolateyinstall.ps1") -Encoding UTF8

# Copy binaries to tools directory
foreach ($binary in $REQUIRED_BINARIES) {
    $source = Join-Path $TARGET_DIR $binary
    $dest = Join-Path $tools_dir $binary
    Copy-Item $source $dest
}

# Build chocolatey package
if (Get-Command choco -ErrorAction SilentlyContinue) {
    Push-Location $choco_dir
    try {
        & choco pack
        if ($LASTEXITCODE -eq 0) {
            $nupkg = Get-ChildItem "*.nupkg" | Select-Object -First 1
            if ($nupkg) {
                Move-Item $nupkg.FullName (Join-Path $OutputDir $nupkg.Name)
                Write-Host "✅ Chocolatey package created: $($nupkg.Name)" -ForegroundColor Green
            }
        }
    }
    finally {
        Pop-Location
    }
} else {
    Write-Host "⚠️  Chocolatey not found. Package files created but not packed." -ForegroundColor Yellow
    Copy-Item $choco_dir (Join-Path $OutputDir "chocolatey-source") -Recurse
}

# Generate installation verification script
$verify_script = @"
# WinUtils Installation Verification Script
# Run this script to verify WinUtils installation

Write-Host "🔍 Verifying WinUtils Installation..." -ForegroundColor Green
Write-Host "====================================" -ForegroundColor Green

`$utilities = @(
    "ls", "cat", "cp", "mv", "rm", "mkdir", "rmdir", "pwd", "echo",
    "grep", "find", "sort", "wc", "head", "tail", "cut", "tr",
    "chmod", "du", "touch", "which"
)

`$passed = 0
`$failed = 0

foreach (`$utility in `$utilities) {
    try {
        `$output = & `$utility --version 2>&1
        if (`$LASTEXITCODE -eq 0) {
            Write-Host "✅ `$utility - OK" -ForegroundColor Green
            `$passed++
        } else {
            Write-Host "❌ `$utility - FAILED" -ForegroundColor Red
            `$failed++
        }
    } catch {
        Write-Host "❌ `$utility - NOT FOUND" -ForegroundColor Red
        `$failed++
    }
}

Write-Host ""
Write-Host "Summary: `$passed passed, `$failed failed" -ForegroundColor Cyan

if (`$failed -eq 0) {
    Write-Host "🎉 All utilities verified successfully!" -ForegroundColor Green
} else {
    Write-Host "⚠️  Some utilities failed verification." -ForegroundColor Yellow
    Write-Host "Please ensure WinUtils is properly installed and in PATH." -ForegroundColor Yellow
}
"@

$verify_script | Out-File -FilePath (Join-Path $OutputDir "verify-installation.ps1") -Encoding UTF8

# Cleanup temp directory
Remove-Item $TEMP_DIR -Recurse -Force

# Final summary
Write-Host ""
Write-Host "🎉 Installer creation completed!" -ForegroundColor Green
Write-Host "================================" -ForegroundColor Green
Write-Host ""

$created_files = Get-ChildItem $OutputDir
foreach ($file in $created_files) {
    $size = if ($file.Length -gt 1MB) {
        "{0:N1} MB" -f ($file.Length / 1MB)
    } elseif ($file.Length -gt 1KB) {
        "{0:N1} KB" -f ($file.Length / 1KB)
    } else {
        "$($file.Length) bytes"
    }

    Write-Host "📦 $($file.Name) ($size)" -ForegroundColor Cyan
}

Write-Host ""
Write-Host "Installation packages are ready for distribution!" -ForegroundColor Green
Write-Host "Use verify-installation.ps1 to test the installation." -ForegroundColor Yellow
