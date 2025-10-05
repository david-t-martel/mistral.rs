# Windows Coreutils Deployment Script
# Deploy binaries to production environments

param(
    [string]$Environment = "local",
    [string]$Version = "1.0.0",
    [switch]$Force = $false,
    [switch]$CreateSymlinks = $false,
    [switch]$UpdatePath = $false,
    [switch]$InstallService = $false
)

$ErrorActionPreference = "Stop"

# Configuration
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$SourceDir = Join-Path $ProjectRoot "target" "release"
$PackageDir = Join-Path $ProjectRoot "dist"
$LogFile = Join-Path $ProjectRoot "deployment.log"

# Deployment targets
$Targets = @{
    local = @{
        Path = "C:\users\david\.local\bin"
        Prefix = "wu-"
        CreateBackup = $true
    }
    system = @{
        Path = "C:\Windows\System32"
        Prefix = "wu-"
        CreateBackup = $true
        RequireAdmin = $true
    }
    portable = @{
        Path = Join-Path $ProjectRoot "portable"
        Prefix = ""
        CreateBackup = $false
    }
}

# Utility lists
$CoreUtils = @(
    "arch", "b2sum", "b3sum", "base32", "base64", "basename", "basenc",
    "cat", "chgrp", "chmod", "chown", "chroot", "cksum", "comm", "cp",
    "csplit", "cut", "date", "dd", "df", "dir", "dircolors", "dirname",
    "du", "echo", "env", "expand", "expr", "factor", "false", "fmt",
    "fold", "groups", "hashsum", "head", "hostid", "hostname", "id",
    "install", "join", "kill", "link", "ln", "logname", "ls", "md5sum",
    "mkdir", "mkfifo", "mknod", "mktemp", "more", "mv", "nice", "nl",
    "nohup", "nproc", "numfmt", "od", "paste", "pathchk", "pinky", "pr",
    "printenv", "printf", "ptx", "pwd", "readlink", "realpath", "relpath",
    "rm", "rmdir", "runcon", "seq", "sha1sum", "sha224sum", "sha256sum",
    "sha384sum", "sha3-224sum", "sha3-256sum", "sha3-384sum", "sha3-512sum",
    "sha3sum", "sha512sum", "shake128sum", "shake256sum", "shred", "shuf",
    "sleep", "sort", "split", "stat", "stdbuf", "stty", "sum", "sync",
    "tac", "tail", "tee", "test", "timeout", "touch", "tr", "true",
    "truncate", "tsort", "tty", "uname", "unexpand", "uniq", "unlink",
    "uptime", "users", "vdir", "wc", "who", "whoami", "yes"
)

$DeriveUtils = @("where", "which", "tree")
$ExternalUtils = @("rg", "fd")

# Helper functions
function Write-Log {
    param([string]$Message, [string]$Level = "INFO")

    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logEntry = "$timestamp [$Level] $Message"

    Add-Content -Path $LogFile -Value $logEntry

    switch ($Level) {
        "ERROR" { Write-Host $Message -ForegroundColor Red }
        "WARNING" { Write-Host $Message -ForegroundColor Yellow }
        "SUCCESS" { Write-Host $Message -ForegroundColor Green }
        default { Write-Host $Message -ForegroundColor White }
    }
}

function Test-Administrator {
    $currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    return $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Backup-Existing {
    param([string]$Path)

    if (-not (Test-Path $Path)) { return }

    $backupDir = Join-Path (Split-Path $Path) "backup_$(Get-Date -Format 'yyyyMMdd_HHmmss')"
    New-Item -ItemType Directory -Path $backupDir -Force | Out-Null

    Get-ChildItem -Path $Path -Filter "*.exe" | ForEach-Object {
        Copy-Item -Path $_.FullName -Destination $backupDir -Force
    }

    Write-Log "Created backup in $backupDir" "INFO"
    return $backupDir
}

function Deploy-Binaries {
    param(
        [string]$TargetPath,
        [string]$Prefix,
        [bool]$CreateBackup
    )

    # Create target directory if it doesn't exist
    if (-not (Test-Path $TargetPath)) {
        New-Item -ItemType Directory -Path $TargetPath -Force | Out-Null
        Write-Log "Created directory: $TargetPath" "INFO"
    }

    # Backup existing if requested
    if ($CreateBackup) {
        $backupPath = Backup-Existing -Path $TargetPath
    }

    $deployed = 0
    $failed = 0

    # Deploy core utilities
    foreach ($util in $CoreUtils) {
        $sourceName = "uu_$util.exe"
        $sourcePath = Join-Path $SourceDir $sourceName
        $targetName = "$Prefix$util.exe"
        $targetPath = Join-Path $TargetPath $targetName

        if (Test-Path $sourcePath) {
            try {
                Copy-Item -Path $sourcePath -Destination $targetPath -Force
                $deployed++
                Write-Verbose "Deployed $util to $targetPath"
            } catch {
                Write-Log "Failed to deploy $util: $_" "ERROR"
                $failed++
            }
        } else {
            Write-Log "Source not found: $sourcePath" "WARNING"
        }
    }

    # Deploy derive utilities
    foreach ($util in $DeriveUtils) {
        $sourceName = "$util.exe"
        $sourcePath = Join-Path $SourceDir $sourceName
        $targetPath = Join-Path $TargetPath $sourceName

        if (Test-Path $sourcePath) {
            try {
                Copy-Item -Path $sourcePath -Destination $targetPath -Force
                $deployed++
                Write-Verbose "Deployed $util to $targetPath"
            } catch {
                Write-Log "Failed to deploy $util: $_" "ERROR"
                $failed++
            }
        }
    }

    # Deploy external utilities
    foreach ($util in $ExternalUtils) {
        $sourceName = "$util.exe"
        $sourcePath = Join-Path $SourceDir $sourceName
        $targetPath = Join-Path $TargetPath $sourceName

        if (Test-Path $sourcePath) {
            try {
                Copy-Item -Path $sourcePath -Destination $targetPath -Force
                $deployed++
                Write-Verbose "Deployed $util to $targetPath"
            } catch {
                Write-Log "Failed to deploy $util: $_" "ERROR"
                $failed++
            }
        }
    }

    Write-Log "Deployment complete: $deployed succeeded, $failed failed" "SUCCESS"
    return @{ Deployed = $deployed; Failed = $failed; BackupPath = $backupPath }
}

function Create-Symlinks {
    param([string]$TargetPath, [string]$Prefix)

    if (-not (Test-Administrator)) {
        Write-Log "Administrator privileges required for symlink creation" "ERROR"
        return
    }

    $created = 0
    foreach ($util in $CoreUtils) {
        $targetExe = Join-Path $TargetPath "$Prefix$util.exe"
        $linkPath = Join-Path $TargetPath "$util.exe"

        if ((Test-Path $targetExe) -and (-not (Test-Path $linkPath))) {
            try {
                New-Item -ItemType SymbolicLink -Path $linkPath -Target $targetExe -Force | Out-Null
                $created++
            } catch {
                Write-Log "Failed to create symlink for $util: $_" "WARNING"
            }
        }
    }

    Write-Log "Created $created symlinks" "INFO"
}

function Update-SystemPath {
    param([string]$Path)

    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$Path*") {
        $newPath = "$currentPath;$Path"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Log "Added $Path to user PATH" "SUCCESS"
    } else {
        Write-Log "$Path already in PATH" "INFO"
    }
}

function Install-AsService {
    param([string]$ServiceName = "WinCoreutils", [string]$BinaryPath)

    if (-not (Test-Administrator)) {
        Write-Log "Administrator privileges required for service installation" "ERROR"
        return
    }

    try {
        $service = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
        if ($service) {
            Stop-Service -Name $ServiceName -Force
            Remove-Service -Name $ServiceName -Force
        }

        New-Service -Name $ServiceName `
                   -BinaryPathName $BinaryPath `
                   -DisplayName "Windows Coreutils Service" `
                   -Description "Windows-optimized GNU coreutils implementation" `
                   -StartupType Manual

        Write-Log "Service '$ServiceName' installed successfully" "SUCCESS"
    } catch {
        Write-Log "Failed to install service: $_" "ERROR"
    }
}

function Create-Package {
    param([string]$OutputPath)

    if (-not (Test-Path $PackageDir)) {
        New-Item -ItemType Directory -Path $PackageDir -Force | Out-Null
    }

    $packageName = "winutils-$Version-$(Get-Date -Format 'yyyyMMdd').zip"
    $packagePath = Join-Path $PackageDir $packageName

    # Create temporary staging directory
    $stagingDir = Join-Path $env:TEMP "winutils-staging"
    if (Test-Path $stagingDir) {
        Remove-Item -Path $stagingDir -Recurse -Force
    }
    New-Item -ItemType Directory -Path $stagingDir -Force | Out-Null

    # Copy binaries
    $binDir = Join-Path $stagingDir "bin"
    New-Item -ItemType Directory -Path $binDir -Force | Out-Null
    Copy-Item -Path "$SourceDir\*.exe" -Destination $binDir -Force

    # Copy documentation
    $docDir = Join-Path $stagingDir "docs"
    New-Item -ItemType Directory -Path $docDir -Force | Out-Null
    if (Test-Path (Join-Path $ProjectRoot "README.md")) {
        Copy-Item -Path (Join-Path $ProjectRoot "README.md") -Destination $docDir
    }

    # Copy scripts
    $scriptsDir = Join-Path $stagingDir "scripts"
    Copy-Item -Path (Join-Path $ProjectRoot "scripts") -Destination $scriptsDir -Recurse -Force

    # Create manifest
    $manifest = @{
        Version = $Version
        Date = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
        Utilities = @{
            Core = $CoreUtils
            Derive = $DeriveUtils
            External = $ExternalUtils
        }
        Platform = "Windows x64"
        Compiler = "Rust $(rustc --version)"
    }
    $manifest | ConvertTo-Json -Depth 10 | Out-File (Join-Path $stagingDir "manifest.json")

    # Create ZIP package
    Compress-Archive -Path "$stagingDir\*" -DestinationPath $packagePath -Force

    # Cleanup
    Remove-Item -Path $stagingDir -Recurse -Force

    Write-Log "Package created: $packagePath" "SUCCESS"
    return $packagePath
}

# Main deployment logic
Write-Host "`n=== Windows Coreutils Deployment ===" -ForegroundColor Cyan
Write-Host "Environment: $Environment" -ForegroundColor Yellow
Write-Host "Version: $Version" -ForegroundColor Yellow

# Validate environment
if (-not $Targets.ContainsKey($Environment)) {
    Write-Log "Invalid environment: $Environment" "ERROR"
    exit 1
}

$target = $Targets[$Environment]

# Check admin privileges if required
if ($target.RequireAdmin -and -not (Test-Administrator)) {
    Write-Log "Administrator privileges required for $Environment deployment" "ERROR"
    Write-Log "Please run this script as Administrator" "WARNING"
    exit 1
}

# Check source directory
if (-not (Test-Path $SourceDir)) {
    Write-Log "Source directory not found: $SourceDir" "ERROR"
    Write-Log "Please build the project first: cargo build --release" "WARNING"
    exit 1
}

# Perform deployment
Write-Log "Starting deployment to $Environment" "INFO"
$result = Deploy-Binaries -TargetPath $target.Path -Prefix $target.Prefix -CreateBackup $target.CreateBackup

# Create symlinks if requested
if ($CreateSymlinks) {
    Create-Symlinks -TargetPath $target.Path -Prefix $target.Prefix
}

# Update PATH if requested
if ($UpdatePath) {
    Update-SystemPath -Path $target.Path
}

# Install service if requested
if ($InstallService) {
    $serviceBinary = Join-Path $target.Path "wu-service.exe"
    Install-AsService -BinaryPath $serviceBinary
}

# Create package
$packagePath = Create-Package

# Generate deployment report
$report = @{
    Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    Environment = $Environment
    Version = $Version
    TargetPath = $target.Path
    DeployedCount = $result.Deployed
    FailedCount = $result.Failed
    BackupPath = $result.BackupPath
    PackagePath = $packagePath
}

$reportPath = Join-Path $ProjectRoot "deployment-report.json"
$report | ConvertTo-Json -Depth 10 | Out-File $reportPath

Write-Host "`n=== Deployment Summary ===" -ForegroundColor Cyan
Write-Host "Deployed: $($result.Deployed) utilities" -ForegroundColor Green
Write-Host "Failed: $($result.Failed) utilities" -ForegroundColor $(if ($result.Failed -gt 0) { "Red" } else { "Green" })
Write-Host "Target: $($target.Path)" -ForegroundColor White
Write-Host "Package: $packagePath" -ForegroundColor White
Write-Host "Report: $reportPath" -ForegroundColor White

if ($result.BackupPath) {
    Write-Host "Backup: $($result.BackupPath)" -ForegroundColor Yellow
}

Write-Log "Deployment completed successfully" "SUCCESS"
exit 0
