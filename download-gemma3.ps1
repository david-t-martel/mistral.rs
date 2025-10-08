# Download Gemma 3 4B Model from Hugging Face
# Requires: huggingface-cli and HF token

param(
    [string]$ModelId = "google/gemma-3-4b-it",
    [string]$LocalDir = "C:\codedev\llm\.models\gemma-3-4b-it-hf",
    [switch]$Force
)

Write-Host "==> Downloading Gemma 3 4B Model" -ForegroundColor Cyan
Write-Host ""

# Set HF_HOME
$env:HF_HOME = "C:\codedev\llm\.cache\huggingface"
Write-Host "HF_HOME: $env:HF_HOME" -ForegroundColor Gray

# Check for huggingface-cli and install if needed
Write-Host "\nChecking for huggingface-cli..." -ForegroundColor White
$hfCliPath = Get-Command huggingface-cli -ErrorAction SilentlyContinue

if (-not $hfCliPath) {
    Write-Host "✗ huggingface-cli not found" -ForegroundColor Yellow
    Write-Host "Installing huggingface-hub[cli] via uv..." -ForegroundColor White

    # Check for uv
    $uvPath = Get-Command uv -ErrorAction SilentlyContinue
    if (-not $uvPath) {
        Write-Host "Error: uv not found. Please install uv first." -ForegroundColor Red
        Write-Host "Visit: https://github.com/astral-sh/uv" -ForegroundColor Yellow
        exit 1
    }

    try {
        & uv pip install --system huggingface-hub[cli] 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✓ huggingface-cli installed successfully" -ForegroundColor Green

            # Refresh PATH to pick up newly installed command
            $env:PATH = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")

            # Verify installation
            $hfCliPath = Get-Command huggingface-cli -ErrorAction SilentlyContinue
            if (-not $hfCliPath) {
                Write-Host "Warning: huggingface-cli installed but not found in PATH" -ForegroundColor Yellow
                Write-Host "You may need to restart your shell" -ForegroundColor Yellow
            }
        } else {
            Write-Host "Error: Failed to install huggingface-cli" -ForegroundColor Red
            exit 1
        }
    } catch {
        Write-Host "Error installing huggingface-cli: $_" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "✓ huggingface-cli found at: $($hfCliPath.Source)" -ForegroundColor Green
}

# Check for HF token
Write-Host "\nChecking for Hugging Face token..." -ForegroundColor White
$hfTokenFile = "$env:USERPROFILE\.cache\huggingface\token"
if (-not (Test-Path $hfTokenFile)) {
    Write-Host "✗ No Hugging Face token found" -ForegroundColor Yellow
    Write-Host "\nYou need to authenticate with Hugging Face to download models." -ForegroundColor White
    Write-Host "\nOptions:" -ForegroundColor White
    Write-Host "  1. Run: huggingface-cli login" -ForegroundColor Cyan
    Write-Host "  2. Get a token from: https://huggingface.co/settings/tokens" -ForegroundColor Cyan
    Write-Host "\nWould you like to login now? (Y/N): " -ForegroundColor Yellow -NoNewline

    $response = Read-Host
    if ($response -eq 'Y' -or $response -eq 'y') {
        Write-Host "\nLaunching huggingface-cli login..." -ForegroundColor White
        & huggingface-cli login

        if ($LASTEXITCODE -ne 0) {
            Write-Host "\nError: Login failed" -ForegroundColor Red
            exit 1
        }

        # Verify token was created
        if (-not (Test-Path $hfTokenFile)) {
            Write-Host "\nError: Token file not created after login" -ForegroundColor Red
            exit 1
        }

        Write-Host "\n✓ Login successful" -ForegroundColor Green
    } else {
        Write-Host "\nPlease authenticate with Hugging Face and run this script again." -ForegroundColor Yellow
        exit 1
    }
} else {
    Write-Host "✓ HF token found" -ForegroundColor Green
}

# Create target directory if it doesn't exist
if (-not (Test-Path $LocalDir)) {
    New-Item -ItemType Directory -Path $LocalDir -Force | Out-Null
    Write-Host "✓ Created directory: $LocalDir" -ForegroundColor Green
}

# Check if model already exists
$modelFiles = Get-ChildItem $LocalDir -File -ErrorAction SilentlyContinue
if ($modelFiles -and -not $Force) {
    Write-Host "✓ Model files already exist in $LocalDir" -ForegroundColor Yellow
    Write-Host "Use -Force to re-download" -ForegroundColor Gray
    exit 0
}

Write-Host ""
Write-Host "Downloading $ModelId to $LocalDir..." -ForegroundColor White
Write-Host "This may take a while (model is ~8GB)..." -ForegroundColor Gray
Write-Host ""

# Download using huggingface-cli
$command = "huggingface-cli"
$args = @(
    "download",
    $ModelId,
    "--local-dir", $LocalDir,
    "--local-dir-use-symlinks", "False"
)

Write-Host "Command: $command $($args -join ' ')" -ForegroundColor Gray
Write-Host ""

try {
    & $command @args

    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Host "✓ Download complete!" -ForegroundColor Green
        Write-Host ""
        Write-Host "Model location: $LocalDir" -ForegroundColor White

        # List downloaded files
        $files = Get-ChildItem $LocalDir -Recurse -File | Select-Object Name, @{Name="SizeMB";Expression={[math]::Round($_.Length/1MB,2)}}
        Write-Host ""
        Write-Host "Downloaded files:" -ForegroundColor White
        $files | Format-Table -AutoSize

        Write-Host ""
        Write-Host "To use with mistral.rs:" -ForegroundColor Yellow
        Write-Host "  ./mistralrs-server -i --isq Q4_K plain -m `"$LocalDir`"" -ForegroundColor Gray
    } else {
        Write-Host ""
        Write-Host "✗ Download failed with exit code: $LASTEXITCODE" -ForegroundColor Red
        exit $LASTEXITCODE
    }
} catch {
    Write-Host ""
    Write-Host "✗ Error during download: $_" -ForegroundColor Red
    exit 1
}
