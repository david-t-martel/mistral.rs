# Download Gemma 2 2B GGUF Model (smaller, faster, works great with mistral.rs)
# Using wget/curl to download from HuggingFace

param(
    [string]$ModelUrl = "https://huggingface.co/bartowski/gemma-2-2b-it-GGUF/resolve/main/gemma-2-2b-it-Q4_K_M.gguf",
    [string]$LocalDir = "C:\codedev\llm\.models\gemma-2-2b-it-gguf",
    [string]$ModelFile = "gemma-2-2b-it-Q4_K_M.gguf"
)

Write-Host "==> Downloading Gemma 2 2B GGUF Model" -ForegroundColor Cyan
Write-Host "Model: Gemma 2 2B Instruct (Q4_K_M quantization)" -ForegroundColor White
Write-Host "Size: ~1.7 GB" -ForegroundColor Gray
Write-Host ""

# Create directory
if (-not (Test-Path $LocalDir)) {
    New-Item -ItemType Directory -Path $LocalDir -Force | Out-Null
    Write-Host "✓ Created directory: $LocalDir" -ForegroundColor Green
}

$targetPath = Join-Path $LocalDir $ModelFile

# Check if already exists
if (Test-Path $targetPath) {
    $size = [math]::Round((Get-Item $targetPath).Length / 1GB, 2)
    Write-Host "✓ Model already exists: $targetPath ($size GB)" -ForegroundColor Yellow
    Write-Host "Delete the file to re-download" -ForegroundColor Gray
    exit 0
}

Write-Host "Downloading from: $ModelUrl" -ForegroundColor White
Write-Host "To: $targetPath" -ForegroundColor White
Write-Host ""

try {
    # Use .NET WebClient for reliable download with progress
    $webClient = New-Object System.Net.WebClient

    # Register progress event
    $progressHandler = {
        param($sender, $e)
        $percent = $e.ProgressPercentage
        Write-Progress -Activity "Downloading $ModelFile" -Status "$percent% Complete" -PercentComplete $percent
    }

    Register-ObjectEvent -InputObject $webClient -EventName DownloadProgressChanged -Action $progressHandler | Out-Null

    Write-Host "Starting download..." -ForegroundColor Yellow
    $webClient.DownloadFile($ModelUrl, $targetPath)

    Write-Progress -Activity "Downloading $ModelFile" -Completed

    if (Test-Path $targetPath) {
        $size = [math]::Round((Get-Item $targetPath).Length / 1GB, 2)
        Write-Host ""
        Write-Host "✓ Download complete!" -ForegroundColor Green
        Write-Host "  File: $targetPath" -ForegroundColor White
        Write-Host "  Size: $size GB" -ForegroundColor White
        Write-Host ""
        Write-Host "To use with mistral.rs:" -ForegroundColor Yellow
        Write-Host "  .\mistralrs-server.exe gguf -m `"$targetPath`\" -a gemma2" -ForegroundColor Cyan
    } else {
        Write-Host "✗ Download failed - file not created" -ForegroundColor Red
        exit 1
    }
} catch {
    Write-Host ""
    Write-Host "Download failed: $_" -ForegroundColor Red
    exit 1
}
