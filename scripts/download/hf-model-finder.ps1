<#
.SYNOPSIS
    Unified HuggingFace Model Finder and Downloader for mistral.rs

.DESCRIPTION
    Searches HuggingFace Hub for mistral.rs compatible models with advanced filtering,
    downloads selected models, validates compatibility, and updates MODEL_INVENTORY.json.

.PARAMETER Architecture
    Model architecture to search for: llama, gemma, qwen, phi, mistral, or all

.PARAMETER Format
    Model format: gguf (default), safetensors, or both

.PARAMETER Quantization
    Quantization level: Q4_K_M (default), Q5_K_M, Q8_0, etc.

.PARAMETER MaxSize
    Maximum model size: 2GB (fast), 5GB (medium), 10GB (large), unlimited

.PARAMETER Download
    Execute download of selected models

.PARAMETER UpdateInventory
    Update MODEL_INVENTORY.json with downloaded models

.PARAMETER Interactive
    Interactive mode with user prompts for selection

.PARAMETER LocalDir
    Base directory for model storage (default: C:\codedev\llm\.models)

.EXAMPLE
    .\hf-model-finder.ps1 -Architecture qwen -Format gguf -MaxSize 2GB -Interactive

.EXAMPLE
    .\hf-model-finder.ps1 -Architecture all -Download -UpdateInventory

.NOTES
    Version: 1.0
    Author: Claude AI Systems
    Compatible with: mistral.rs 0.6.0+
#>

[CmdletBinding(DefaultParameterSetName='Search')]
param(
    [Parameter(ParameterSetName='Search')]
    [Parameter(ParameterSetName='Download')]
    [ValidateSet('llama', 'gemma', 'qwen', 'phi', 'mistral', 'mamba', 'gpt2', 'all')]
    [string]$Architecture = 'all',

    [Parameter(ParameterSetName='Search')]
    [Parameter(ParameterSetName='Download')]
    [ValidateSet('gguf', 'safetensors', 'both')]
    [string]$Format = 'gguf',

    [Parameter(ParameterSetName='Search')]
    [Parameter(ParameterSetName='Download')]
    [string]$Quantization = 'Q4_K_M',

    [Parameter(ParameterSetName='Search')]
    [Parameter(ParameterSetName='Download')]
    [ValidateSet('2GB', '5GB', '10GB', 'unlimited')]
    [string]$MaxSize = '5GB',

    [Parameter(ParameterSetName='Download')]
    [switch]$Download,

    [Parameter(ParameterSetName='Download')]
    [switch]$UpdateInventory,

    [Parameter(ParameterSetName='Search')]
    [Parameter(ParameterSetName='Download')]
    [switch]$Interactive,

    [Parameter(ParameterSetName='Search')]
    [Parameter(ParameterSetName='Download')]
    [string]$LocalDir = "C:\codedev\llm\.models",

    [Parameter(ParameterSetName='Search')]
    [Parameter(ParameterSetName='Download')]
    [int]$MaxResults = 50
)

$ErrorActionPreference = "Continue"

# ============================================================================
# Constants and Configuration
# ============================================================================

$HF_API_BASE = "https://huggingface.co/api"
$HF_MODELS_API = "$HF_API_BASE/models"
$MODEL_INVENTORY_PATH = "MODEL_INVENTORY.json"

# Supported architectures from mistral.rs GGUF implementation
$SUPPORTED_ARCHITECTURES = @{
    'llama' = @('llama', 'llama2', 'llama3', 'llama-3')
    'gemma' = @('gemma', 'gemma2', 'gemma3', 'gemma-2', 'gemma-3')
    'qwen' = @('qwen', 'qwen2', 'qwen2.5', 'qwen3', 'qwen-2', 'qwen-3')
    'phi' = @('phi', 'phi2', 'phi3', 'phi-2', 'phi-3')
    'mistral' = @('mistral', 'mixtral')
    'mamba' = @('mamba')
    'gpt2' = @('gpt2', 'gptneox', 'gptj')
}

# Size limits in bytes
$SIZE_LIMITS = @{
    '2GB' = 2GB
    '5GB' = 5GB
    '10GB' = 10GB
    'unlimited' = [Int64]::MaxValue
}

# ============================================================================
# Utility Functions
# ============================================================================

function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$Color = "White",
        [switch]$NoNewline
    )

    if ($NoNewline) {
        Write-Host $Message -ForegroundColor $Color -NoNewline
    } else {
        Write-Host $Message -ForegroundColor $Color
    }
}

function Get-FriendlySize {
    param([int64]$Bytes)

    if ($Bytes -lt 1KB) { return "$Bytes B" }
    if ($Bytes -lt 1MB) { return "{0:N2} KB" -f ($Bytes / 1KB) }
    if ($Bytes -lt 1GB) { return "{0:N2} MB" -f ($Bytes / 1MB) }
    return "{0:N2} GB" -f ($Bytes / 1GB)
}

function Test-ArchitectureMatch {
    param(
        [string]$ModelId,
        [string]$TargetArch
    )

    if ($TargetArch -eq 'all') {
        return $true
    }

    $patterns = $SUPPORTED_ARCHITECTURES[$TargetArch]
    $modelLower = $ModelId.ToLower()

    foreach ($pattern in $patterns) {
        if ($modelLower -match $pattern) {
            return $true
        }
    }

    return $false
}

function Get-ModelArchitecture {
    param([string]$ModelId)

    $modelLower = $ModelId.ToLower()

    foreach ($arch in $SUPPORTED_ARCHITECTURES.Keys) {
        foreach ($pattern in $SUPPORTED_ARCHITECTURES[$arch]) {
            if ($modelLower -match $pattern) {
                return $arch
            }
        }
    }

    return "unknown"
}

# ============================================================================
# HuggingFace API Functions
# ============================================================================

function Search-HuggingFaceModels {
    param(
        [string]$Architecture,
        [string]$Format,
        [string]$Quantization,
        [int]$Limit = 50
    )

    Write-ColorOutput "`n==> Searching HuggingFace Hub..." "Cyan"
    Write-ColorOutput "Architecture: $Architecture" "Gray"
    Write-ColorOutput "Format: $Format" "Gray"
    Write-ColorOutput "Quantization: $Quantization" "Gray"
    Write-ColorOutput ""

    $searchTerms = @()

    # Build search query
    if ($Architecture -ne 'all') {
        $patterns = $SUPPORTED_ARCHITECTURES[$Architecture]
        $searchTerms += $patterns[0]
    }

    if ($Format -eq 'gguf' -or $Format -eq 'both') {
        $searchTerms += "GGUF"
    }

    if ($Quantization) {
        $searchTerms += $Quantization
    }

    $query = $searchTerms -join " "
    $encodedQuery = [System.Web.HttpUtility]::UrlEncode($query)

    $apiUrl = "$HF_MODELS_API`?search=$encodedQuery&sort=downloads&direction=-1&limit=$Limit"

    Write-ColorOutput "Query: $query" "Gray"
    Write-ColorOutput "Fetching models from HuggingFace API..." "Yellow"

    try {
        $response = Invoke-RestMethod -Uri $apiUrl -Method Get -UseBasicParsing -TimeoutSec 30

        $filteredModels = @()

        foreach ($model in $response) {
            # Skip if architecture doesn't match
            if (-not (Test-ArchitectureMatch -ModelId $model.id -TargetArch $Architecture)) {
                continue
            }

            # Check tags for format
            $hasGGUF = $model.tags -contains "gguf"
            $hasSafetensors = $model.tags -contains "safetensors"

            if ($Format -eq 'gguf' -and -not $hasGGUF) { continue }
            if ($Format -eq 'safetensors' -and -not $hasSafetensors) { continue }

            # Extract info
            $detectedArch = Get-ModelArchitecture -ModelId $model.id

            $modelInfo = [PSCustomObject]@{
                Id = $model.id
                Author = ($model.id -split '/')[0]
                Name = ($model.id -split '/')[1]
                Architecture = $detectedArch
                Downloads = $model.downloads
                Likes = $model.likes
                Tags = $model.tags
                HasGGUF = $hasGGUF
                HasSafetensors = $hasSafetensors
                LastModified = $model.lastModified
            }

            $filteredModels += $modelInfo
        }

        Write-ColorOutput "`n✓ Found $($filteredModels.Count) matching models" "Green"

        return $filteredModels

    } catch {
        Write-ColorOutput "`n✗ Error searching HuggingFace: $_" "Red"
        return @()
    }
}

function Get-ModelFiles {
    param(
        [string]$ModelId,
        [string]$Format,
        [string]$Quantization
    )

    Write-ColorOutput "  Fetching file list..." "Gray"

    $apiUrl = "$HF_API_BASE/models/$ModelId/tree/main"

    try {
        $response = Invoke-RestMethod -Uri $apiUrl -Method Get -UseBasicParsing -TimeoutSec 30

        $files = @()

        foreach ($item in $response) {
            if ($item.type -ne "file") { continue }

            $fileName = $item.path
            $fileExt = [System.IO.Path]::GetExtension($fileName)

            # Filter by format
            if ($Format -eq 'gguf' -and $fileExt -ne '.gguf') { continue }
            if ($Format -eq 'safetensors' -and $fileExt -ne '.safetensors') { continue }

            # Filter by quantization (if specified)
            if ($Quantization -and $fileName -notmatch $Quantization) { continue }

            $fileInfo = [PSCustomObject]@{
                Path = $fileName
                Size = $item.size
                Oid = $item.oid
                Url = "https://huggingface.co/$ModelId/resolve/main/$fileName"
            }

            $files += $fileInfo
        }

        return $files

    } catch {
        Write-ColorOutput "  ✗ Error fetching file list: $_" "Red"
        return @()
    }
}

function Test-ModelCompatibility {
    param(
        [string]$ModelId,
        [string]$LocalPath
    )

    Write-ColorOutput "  Validating compatibility..." "Yellow"

    $ext = [System.IO.Path]::GetExtension($LocalPath)

    # For GGUF files, basic validation
    if ($ext -eq '.gguf') {
        if (-not (Test-Path $LocalPath)) {
            Write-ColorOutput "  ✗ File not found" "Red"
            return $false
        }

        $fileSize = (Get-Item $LocalPath).Length
        if ($fileSize -lt 1MB) {
            Write-ColorOutput "  ✗ File too small (possibly corrupted)" "Red"
            return $false
        }

        # Check GGUF magic header (GGU*)
        $bytes = [System.IO.File]::ReadAllBytes($LocalPath) | Select-Object -First 4
        $magic = [System.Text.Encoding]::ASCII.GetString($bytes)

        if ($magic -notmatch "^GGU") {
            Write-ColorOutput "  ✗ Invalid GGUF header" "Red"
            return $false
        }

        Write-ColorOutput "  ✓ GGUF validation passed" "Green"
        return $true
    }

    # For safetensors, check index file
    if ($ext -eq '.safetensors') {
        $dir = Split-Path $LocalPath -Parent
        $indexFile = Join-Path $dir "model.safetensors.index.json"

        if (Test-Path $indexFile) {
            try {
                $index = Get-Content $indexFile | ConvertFrom-Json

                # Check for required fields
                if (-not $index.metadata -or -not $index.weight_map) {
                    Write-ColorOutput "  ⚠ Index file missing required fields" "Yellow"
                    return $false
                }

                Write-ColorOutput "  ✓ Safetensors index validation passed" "Green"
                return $true
            } catch {
                Write-ColorOutput "  ⚠ Could not parse index file" "Yellow"
                return $false
            }
        } else {
            Write-ColorOutput "  ℹ Single safetensors file (no index)" "Gray"
            return $true
        }
    }

    Write-ColorOutput "  ⚠ Unknown format, skipping validation" "Yellow"
    return $true
}

# ============================================================================
# Download Functions
# ============================================================================

function Download-ModelFile {
    param(
        [string]$Url,
        [string]$OutputPath,
        [string]$ModelName
    )

    $fileName = [System.IO.Path]::GetFileName($OutputPath)

    # Check if already exists
    if (Test-Path $OutputPath) {
        $existingSize = (Get-Item $OutputPath).Length
        Write-ColorOutput "  ✓ File already exists: $fileName ($(Get-FriendlySize $existingSize))" "Yellow"
        return $true
    }

    # Create directory
    $dir = Split-Path $OutputPath -Parent
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }

    Write-ColorOutput "  Downloading: $fileName" "Cyan"
    Write-ColorOutput "  From: $Url" "Gray"

    try {
        # Use .NET WebClient for progress reporting
        $webClient = New-Object System.Net.WebClient

        # Track download progress
        $lastPercent = -1
        $progressHandler = {
            param($sender, $e)
            $percent = $e.ProgressPercentage
            if ($percent -ne $lastPercent) {
                $lastPercent = $percent
                $received = Get-FriendlySize $e.BytesReceived
                $total = Get-FriendlySize $e.TotalBytesToReceive
                Write-Progress -Activity "Downloading $fileName" `
                    -Status "$percent% ($received / $total)" `
                    -PercentComplete $percent
            }
        }

        Register-ObjectEvent -InputObject $webClient -EventName DownloadProgressChanged -Action $progressHandler | Out-Null

        $startTime = Get-Date
        $webClient.DownloadFile($Url, $OutputPath)
        $duration = ((Get-Date) - $startTime).TotalSeconds

        Write-Progress -Activity "Downloading $fileName" -Completed

        # Verify download
        if (Test-Path $OutputPath) {
            $size = (Get-Item $OutputPath).Length
            $speed = Get-FriendlySize ($size / $duration)

            Write-ColorOutput "  ✓ Download complete: $(Get-FriendlySize $size) in $([math]::Round($duration, 1))s ($speed/s)" "Green"
            return $true
        } else {
            Write-ColorOutput "  ✗ Download failed: File not created" "Red"
            return $false
        }

    } catch {
        Write-ColorOutput "  ✗ Download error: $_" "Red"

        # Clean up partial download
        if (Test-Path $OutputPath) {
            Remove-Item $OutputPath -Force -ErrorAction SilentlyContinue
        }

        return $false
    }
}

function Download-SafetensorsModel {
    param(
        [string]$ModelId,
        [string]$LocalDir
    )

    Write-ColorOutput "`n  Using huggingface-cli for safetensors download..." "Yellow"

    # Check for huggingface-cli
    $hfCli = Get-Command huggingface-cli -ErrorAction SilentlyContinue

    if (-not $hfCli) {
        Write-ColorOutput "  ✗ huggingface-cli not found" "Red"
        Write-ColorOutput "  Install with: uv pip install --system huggingface-hub[cli]" "Yellow"
        return $false
    }

    # Check for HF token
    $hfTokenFile = "$env:USERPROFILE\.cache\huggingface\token"
    if (-not (Test-Path $hfTokenFile)) {
        Write-ColorOutput "  ✗ HuggingFace token not found" "Red"
        Write-ColorOutput "  Run: huggingface-cli login" "Yellow"
        return $false
    }

    try {
        $args = @(
            "download",
            $ModelId,
            "--local-dir", $LocalDir,
            "--local-dir-use-symlinks", "False"
        )

        Write-ColorOutput "  Running: huggingface-cli $($args -join ' ')" "Gray"

        & huggingface-cli @args

        if ($LASTEXITCODE -eq 0) {
            Write-ColorOutput "  ✓ Download complete" "Green"
            return $true
        } else {
            Write-ColorOutput "  ✗ Download failed with exit code: $LASTEXITCODE" "Red"
            return $false
        }

    } catch {
        Write-ColorOutput "  ✗ Download error: $_" "Red"
        return $false
    }
}

# ============================================================================
# Inventory Management
# ============================================================================

function Get-ModelInventory {
    if (Test-Path $MODEL_INVENTORY_PATH) {
        try {
            $json = Get-Content $MODEL_INVENTORY_PATH -Raw | ConvertFrom-Json
            return @($json)
        } catch {
            Write-ColorOutput "⚠ Could not parse MODEL_INVENTORY.json" "Yellow"
            return @()
        }
    }
    return @()
}

function Update-ModelInventory {
    param(
        [string]$Name,
        [string]$Path,
        [string]$Format,
        [double]$SizeGB,
        [string]$Notes
    )

    $inventory = Get-ModelInventory

    # Check if already exists
    $existing = $inventory | Where-Object { $_.path -eq $Path }

    if ($existing) {
        Write-ColorOutput "  ℹ Model already in inventory" "Gray"
        return
    }

    # Add new entry
    $newEntry = [PSCustomObject]@{
        name = $Name
        path = $Path
        format = $Format
        size_gb = $SizeGB
        notes = $Notes
    }

    $inventory += $newEntry

    # Save
    try {
        $inventory | ConvertTo-Json -Depth 10 | Set-Content $MODEL_INVENTORY_PATH -Encoding UTF8
        Write-ColorOutput "  ✓ Updated MODEL_INVENTORY.json" "Green"
    } catch {
        Write-ColorOutput "  ✗ Failed to update inventory: $_" "Red"
    }
}

# ============================================================================
# Interactive Mode
# ============================================================================

function Show-ModelSelectionMenu {
    param([array]$Models)

    Write-ColorOutput "`n==> Available Models" "Cyan"
    Write-ColorOutput ""

    for ($i = 0; $i -lt $Models.Count; $i++) {
        $model = $Models[$i]
        $num = $i + 1

        Write-ColorOutput "[$num] $($model.Id)" "White"
        Write-ColorOutput "    Architecture: $($model.Architecture)" "Gray"
        Write-ColorOutput "    Downloads: $($model.Downloads)" "Gray"
        Write-ColorOutput "    Formats: $(if($model.HasGGUF){'GGUF'}) $(if($model.HasSafetensors){'Safetensors'})" "Gray"
        Write-ColorOutput ""
    }

    Write-ColorOutput "Select models to download (e.g., 1,3,5 or 1-3 or 'all'): " "Yellow" -NoNewline
    $selection = Read-Host

    $selectedIndices = @()

    if ($selection -eq 'all') {
        $selectedIndices = 0..($Models.Count - 1)
    } else {
        $parts = $selection -split ','
        foreach ($part in $parts) {
            if ($part -match '(\d+)-(\d+)') {
                $start = [int]$matches[1] - 1
                $end = [int]$matches[2] - 1
                $selectedIndices += $start..$end
            } elseif ($part -match '\d+') {
                $selectedIndices += [int]$part - 1
            }
        }
    }

    $selectedModels = @()
    foreach ($idx in $selectedIndices) {
        if ($idx -ge 0 -and $idx -lt $Models.Count) {
            $selectedModels += $Models[$idx]
        }
    }

    return $selectedModels
}

# ============================================================================
# Main Execution
# ============================================================================

function Main {
    Write-ColorOutput "`n============================================" "Cyan"
    Write-ColorOutput "HuggingFace Model Finder for mistral.rs" "Cyan"
    Write-ColorOutput "============================================" "Cyan"

    # Add System.Web for URL encoding
    Add-Type -AssemblyName System.Web

    # Search for models
    $models = Search-HuggingFaceModels -Architecture $Architecture -Format $Format -Quantization $Quantization -Limit $MaxResults

    if ($models.Count -eq 0) {
        Write-ColorOutput "`n✗ No models found matching criteria" "Red"
        return
    }

    # Apply size filter
    $sizeLimit = $SIZE_LIMITS[$MaxSize]

    # Interactive selection
    if ($Interactive) {
        $selectedModels = Show-ModelSelectionMenu -Models $models

        if ($selectedModels.Count -eq 0) {
            Write-ColorOutput "`n✗ No models selected" "Yellow"
            return
        }

        $models = $selectedModels
    } else {
        # Show top 10 results
        Write-ColorOutput "`nTop results:" "White"
        $models | Select-Object -First 10 | ForEach-Object {
            Write-ColorOutput "  • $($_.Id) [$($_.Architecture)] - $($_.Downloads) downloads" "Gray"
        }
    }

    # Download if requested
    if ($Download) {
        Write-ColorOutput "`n==> Starting Downloads" "Cyan"
        Write-ColorOutput ""

        foreach ($model in $models) {
            Write-ColorOutput "[$($model.Id)]" "Yellow"

            # Get files
            $files = Get-ModelFiles -ModelId $model.Id -Format $Format -Quantization $Quantization

            if ($files.Count -eq 0) {
                Write-ColorOutput "  ⚠ No matching files found" "Yellow"
                continue
            }

            # Filter by size
            $files = $files | Where-Object { $_.Size -le $sizeLimit }

            if ($files.Count -eq 0) {
                Write-ColorOutput "  ⚠ No files within size limit" "Yellow"
                continue
            }

            Write-ColorOutput "  Found $($files.Count) file(s)" "Gray"

            # Create model directory
            $modelDirName = $model.Name.ToLower() -replace '[^a-z0-9\-]', '-'
            $modelDir = Join-Path $LocalDir $modelDirName

            # Download each file
            $downloadedFiles = @()
            foreach ($file in $files) {
                $localPath = Join-Path $modelDir $file.Path

                $success = Download-ModelFile -Url $file.Url -OutputPath $localPath -ModelName $model.Name

                if ($success) {
                    $downloadedFiles += $localPath

                    # Validate
                    Test-ModelCompatibility -ModelId $model.Id -LocalPath $localPath | Out-Null
                }
            }

            # Update inventory if requested
            if ($UpdateInventory -and $downloadedFiles.Count -gt 0) {
                $primaryFile = $downloadedFiles[0]
                $sizeGB = [math]::Round((Get-Item $primaryFile).Length / 1GB, 2)

                $notes = "$($model.Architecture) model"
                if ($Quantization) {
                    $notes += ", $Quantization quantization"
                }

                Update-ModelInventory -Name $model.Name `
                    -Path $primaryFile `
                    -Format $Format `
                    -SizeGB $sizeGB `
                    -Notes $notes
            }

            Write-ColorOutput ""
        }

        Write-ColorOutput "==> Download Summary" "Cyan"
        Write-ColorOutput "✓ Completed downloads" "Green"

        if ($UpdateInventory) {
            Write-ColorOutput "✓ Updated MODEL_INVENTORY.json" "Green"
        }

    } else {
        Write-ColorOutput "`nℹ Use -Download flag to download selected models" "Yellow"
    }

    Write-ColorOutput ""
}

# Run main function
Main
