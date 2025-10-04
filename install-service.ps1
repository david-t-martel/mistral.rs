# PowerShell script to install mistral.rs as a Windows service
# Uses NSSM (Non-Sucking Service Manager)

[CmdletBinding()]
param(
    [Parameter()][switch]$Install,
    [Parameter()][switch]$Uninstall,
    [Parameter()][switch]$Start,
    [Parameter()][switch]$Stop,
    [Parameter()][switch]$Restart,
    [Parameter()][switch]$Status,
    [Parameter()][string]$ServiceName = "MistralRS",
    [Parameter()][string]$BinaryPath = "$PSScriptRoot\target\release\mistralrs-server.exe",
    [Parameter()][string]$ModelDir = "C:\codedev\llm\.models\qwen2.5-1.5b-it-gguf",
    [Parameter()][string]$ModelFile = "Qwen2.5-1.5B-Instruct-Q4_K_M.gguf",
    [Parameter()][string]$McpConfig = "$PSScriptRoot\configs\prod\mcp-config.json",
    [Parameter()][int]$Port = 8080,
    [Parameter()][string]$LogDir = "$PSScriptRoot\logs"
)

# Require administrator privileges
if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Error "This script requires administrator privileges."
    exit 1
}

function Test-NSSMInstalled {
    try {
        $null = Get-Command nssm.exe -ErrorAction Stop
        return $true
    } catch {
        return $false
    }
}

function Install-MistralRSService {
    Write-Host "Installing $ServiceName service..." -ForegroundColor Cyan

    # Verify binary exists
    if (-not (Test-Path $BinaryPath)) {
        Write-Error "Binary not found: $BinaryPath"
        Write-Host "Build first with: make build-cuda-full" -ForegroundColor Yellow
        exit 1
    }

    # Create log directory
    if (-not (Test-Path $LogDir)) {
        New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
    }

    # Check if service already exists
    $existingService = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if ($existingService) {
        Write-Host "Service already exists. Removing..." -ForegroundColor Yellow
        nssm stop $ServiceName
        nssm remove $ServiceName confirm
        Start-Sleep -Seconds 2
    }

    # Install with NSSM
    nssm install $ServiceName $BinaryPath

    # Configure service arguments
    $args = @(
        "--port", $Port,
        "--mcp-config", $McpConfig,
        "gguf",
        "-m", $ModelDir,
        "-f", $ModelFile
    )
    nssm set $ServiceName AppParameters ($args -join " ")

    # Set working directory
    nssm set $ServiceName AppDirectory (Split-Path $BinaryPath -Parent)

    # Configure logging
    $stdoutLog = Join-Path $LogDir "mistralrs-stdout.log"
    $stderrLog = Join-Path $LogDir "mistralrs-stderr.log"
    nssm set $ServiceName AppStdout $stdoutLog
    nssm set $ServiceName AppStderr $stderrLog

    # Enable log rotation
    nssm set $ServiceName AppRotateFiles 1
    nssm set $ServiceName AppRotateSeconds 86400  # Daily
    nssm set $ServiceName AppRotateBytes 104857600  # 100MB

    # Set environment variables
    nssm set $ServiceName AppEnvironmentExtra "RUST_LOG=info" "RUST_BACKTRACE=1" "CUDA_VISIBLE_DEVICES=0"

    # Auto-restart on failure
    nssm set $ServiceName AppExit Default Restart
    nssm set $ServiceName AppRestartDelay 10000  # 10 seconds

    # Set description
    nssm set $ServiceName Description "mistral.rs LLM Inference Server - Fast CUDA-accelerated LLM serving"

    # Auto-start on boot
    nssm set $ServiceName Start SERVICE_AUTO_START

    Write-Host "Service installed successfully!" -ForegroundColor Green
    Write-Host "  Binary: $BinaryPath" -ForegroundColor Cyan
    Write-Host "  Port: $Port" -ForegroundColor Cyan
    Write-Host "  Logs: $LogDir" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Start with: .\install-service.ps1 -Start" -ForegroundColor Yellow
}

function Uninstall-MistralRSService {
    Write-Host "Uninstalling $ServiceName service..." -ForegroundColor Cyan

    $service = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if (-not $service) {
        Write-Warning "Service not installed."
        return
    }

    nssm stop $ServiceName
    Start-Sleep -Seconds 2
    nssm remove $ServiceName confirm

    Write-Host "Service uninstalled." -ForegroundColor Green
}

function Start-MistralRSService {
    Write-Host "Starting $ServiceName service..." -ForegroundColor Cyan

    $service = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if (-not $service) {
        Write-Error "Service not installed. Run with -Install first."
        exit 1
    }

    Start-Service -Name $ServiceName
    Start-Sleep -Seconds 3

    $service = Get-Service -Name $ServiceName
    if ($service.Status -eq 'Running') {
        Write-Host "Service started!" -ForegroundColor Green
        Write-Host "API: http://localhost:$Port" -ForegroundColor Cyan
        Write-Host "Logs: $LogDir" -ForegroundColor Cyan
    } else {
        Write-Error "Service failed to start. Check logs."
    }
}

function Stop-MistralRSService {
    Write-Host "Stopping $ServiceName service..." -ForegroundColor Cyan
    Stop-Service -Name $ServiceName -Force
    Write-Host "Service stopped." -ForegroundColor Green
}

function Restart-MistralRSService {
    Stop-MistralRSService
    Start-Sleep -Seconds 2
    Start-MistralRSService
}

function Get-MistralRSStatus {
    $service = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if (-not $service) {
        Write-Host "Service not installed." -ForegroundColor Yellow
        return
    }

    Write-Host ""
    Write-Host "Service Status:" -ForegroundColor Cyan
    Write-Host "  Name: $($service.Name)"
    Write-Host "  Status: $($service.Status)" -ForegroundColor $(if ($service.Status -eq 'Running') { 'Green' } else { 'Red' })
    Write-Host "  Start Type: $($service.StartType)"
    Write-Host ""

    if ($service.Status -eq 'Running') {
        try {
            $response = Invoke-WebRequest -Uri "http://localhost:$Port/health" -TimeoutSec 5 -UseBasicParsing -ErrorAction Stop
            Write-Host "  API: Responding (HTTP $($response.StatusCode))" -ForegroundColor Green
        } catch {
            Write-Host "  API: Not responding" -ForegroundColor Red
        }
    }

    # Show recent logs
    $stderrLog = Join-Path $LogDir "mistralrs-stderr.log"
    if (Test-Path $stderrLog) {
        Write-Host ""
        Write-Host "Recent logs:" -ForegroundColor Cyan
        Get-Content $stderrLog -Tail 10 | ForEach-Object { Write-Host "  $_" }
    }
}

# Check for NSSM
if (-not (Test-NSSMInstalled)) {
    Write-Error "NSSM not found. Install with: choco install nssm"
    Write-Host "Or download from: https://nssm.cc/" -ForegroundColor Yellow
    exit 1
}

# Main logic
if ($Install) { Install-MistralRSService }
elseif ($Uninstall) { Uninstall-MistralRSService }
elseif ($Start) { Start-MistralRSService }
elseif ($Stop) { Stop-MistralRSService }
elseif ($Restart) { Restart-MistralRSService }
elseif ($Status) { Get-MistralRSStatus }
else {
    Write-Host "mistral.rs Windows Service Manager" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage:" -ForegroundColor Yellow
    Write-Host "  .\install-service.ps1 -Install    # Install service"
    Write-Host "  .\install-service.ps1 -Uninstall  # Remove service"
    Write-Host "  .\install-service.ps1 -Start      # Start service"
    Write-Host "  .\install-service.ps1 -Stop       # Stop service"
    Write-Host "  .\install-service.ps1 -Restart    # Restart service"
    Write-Host "  .\install-service.ps1 -Status     # Check status"
    Write-Host ""
}
