#Requires -Version 5.1
# Uninstaller for winutils

[CmdletBinding()]
param(
    [string]$InstallPath,
    [switch]$SystemWide,
    [switch]$KeepConfig
)

$ErrorActionPreference = 'Stop'

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    Write-Host "[$Level] $Message"
}

function Get-InstallationPath {
    if ($InstallPath) { return $InstallPath }
    if ($SystemWide) { return "$env:ProgramFiles\winutils" }
    return "$env:LOCALAPPDATA\winutils"
}

try {
    Write-Log "winutils Uninstaller"

    $installDir = Get-InstallationPath
    Write-Log "Uninstalling from: $installDir"

    if (Test-Path $installDir) {
        Remove-Item $installDir -Recurse -Force
        Write-Log "Removed installation directory" "SUCCESS"
    }

    # Remove from PATH
    $scope = if ($SystemWide) { "Machine" } else { "User" }
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", $scope)
    $newPath = ($currentPath -split ';' | Where-Object { $_ -ne $installDir }) -join ';'
    [Environment]::SetEnvironmentVariable("PATH", $newPath, $scope)

    Write-Log "Uninstallation completed successfully!" "SUCCESS"
}
catch {
    Write-Log "Uninstallation failed: $($_.Exception.Message)" "ERROR"
    exit 1
}
