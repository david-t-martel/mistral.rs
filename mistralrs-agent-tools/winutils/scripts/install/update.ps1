#Requires -Version 5.1
# Updater for winutils

[CmdletBinding()]
param(
    [string]$Version = "latest",
    [switch]$Force
)

$ErrorActionPreference = 'Stop'
$GITHUB_REPO = "david-t-martel/uutils-windows"

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    Write-Host "[$Level] $Message"
}

function Get-CurrentVersion {
    try {
        $output = wu-ls --version 2>&1
        if ($LASTEXITCODE -eq 0) {
            return ($output -split ' ')[1]
        }
    }
    catch {
        return $null
    }
    return $null
}

function Get-LatestVersion {
    try {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$GITHUB_REPO/releases/latest"
        return $release.tag_name
    }
    catch {
        throw "Failed to fetch latest version information"
    }
}

try {
    Write-Log "winutils Updater"

    $currentVersion = Get-CurrentVersion
    if (-not $currentVersion) {
        Write-Log "winutils not found. Please install first." "ERROR"
        exit 1
    }

    Write-Log "Current version: $currentVersion"

    if ($Version -eq "latest") {
        $Version = Get-LatestVersion
    }

    Write-Log "Target version: $Version"

    if ($currentVersion -eq $Version -and -not $Force) {
        Write-Log "Already up to date!" "SUCCESS"
        exit 0
    }

    Write-Log "Updating to version $Version..."

    # Download and run installer
    $installerUrl = "https://raw.githubusercontent.com/$GITHUB_REPO/main/winutils/scripts/install/install.ps1"
    $installerScript = Invoke-WebRequest -Uri $installerUrl -UseBasicParsing

    # Execute installer
    Invoke-Expression $installerScript.Content

    Write-Log "Update completed successfully!" "SUCCESS"
}
catch {
    Write-Log "Update failed: $($_.Exception.Message)" "ERROR"
    exit 1
}
