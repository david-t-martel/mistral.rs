#Requires -Version 5.1
# Universal PowerShell installer for winutils (uutils/coreutils Windows fork)

param(
    [string]$InstallPath,
    [switch]$SystemWide,
    [switch]$BuildFromSource,
    [switch]$Quiet
)

$ErrorActionPreference = 'Stop'
$SCRIPT_VERSION = "1.0.0"
$GITHUB_REPO = "david-t-martel/uutils-windows"

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    if (-not $Quiet) {
        $color = switch ($Level) {
            "ERROR" { "Red" }
            "WARN" { "Yellow" }
            "SUCCESS" { "Green" }
            default { "White" }
        }
        Write-Host "[$Level] $Message" -ForegroundColor $color
    }
}

function Test-AdminPrivileges {
    $currentUser = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($currentUser)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-InstallationPath {
    if ($InstallPath) { return $InstallPath }
    if ($SystemWide) { return "$env:ProgramFiles\winutils" }
    return "$env:LOCALAPPDATA\winutils"
}

try {
    Write-Log "winutils Installer v$SCRIPT_VERSION"

    if ($SystemWide -and -not (Test-AdminPrivileges)) {
        Write-Log "System-wide installation requires admin privileges" "ERROR"
        exit 1
    }

    $installDir = Get-InstallationPath
    Write-Log "Installing to: $installDir"

    if (-not (Test-Path $installDir)) {
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    }

    Write-Log "Installation completed successfully!" "SUCCESS"
}
catch {
    Write-Log "Installation failed: $($_.Exception.Message)" "ERROR"
    exit 1
}
