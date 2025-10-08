# Get-ProjectPaths.ps1
# Resolves project paths with fallback chain: .env → auto-detect → error
# Replaces hardcoded paths throughout the project

[CmdletBinding()]
param()

function Get-ProjectRoot {
    $currentDir = Get-Location
    while ($currentDir) {
        if (Test-Path (Join-Path $currentDir "Cargo.toml")) {
            return $currentDir
        }
        $currentDir = Split-Path $currentDir -Parent
    }
    throw "Project root not found. Run from within mistral.rs directory."
}

function Get-MistralRSBinary {
    # 1. Check environment variable
    if ($env:MISTRALRS_BINARY -and (Test-Path $env:MISTRALRS_BINARY)) {
        return $env:MISTRALRS_BINARY
    }

    # 2. Check standard locations
    $projectRoot = Get-ProjectRoot
    $exeExt = if ($IsWindows -or $env:OS -eq "Windows_NT") { ".exe" } else { "" }

    $searchRoots = @()
    $searchRoots += Join-Path $projectRoot "target" "debug"
    $searchRoots += Join-Path $projectRoot "target" "release"
    if ($env:USERPROFILE) {
        $searchRoots += Join-Path $env:USERPROFILE ".cargo" "shared-target" "release"
    }
    if ($env:HOME) {
        $searchRoots += Join-Path $env:HOME ".cargo" "shared-target" "release"
    }

    $binaryNames = @(
        "mistral-rs$exeExt",
        "mistralrs-server$exeExt"
    )

    foreach ($root in $searchRoots) {
        foreach ($name in $binaryNames) {
            $candidate = Join-Path $root $name
            if (Test-Path $candidate) {
                return $candidate
            }
        }
    }

    throw @"
mistral-rs binary not found.
Build it with: make build-cuda-full
Or set environment variable: MISTRALRS_BINARY=path\to\mistral-rs.exe
"@
}

function Get-UVPath {
    # 1. Check if in PATH
    if (Get-Command uv -ErrorAction SilentlyContinue) {
        return (Get-Command uv).Source
    }

    # 2. Check environment variable
    if ($env:UV_PATH -and (Test-Path $env:UV_PATH)) {
        return $env:UV_PATH
    }

    # 3. Check standard locations
    $possiblePaths = @(
        (Join-Path $env:USERPROFILE ".local" "bin" "uv.exe"),
        (Join-Path $env:USERPROFILE ".local" "bin" "uv"),
        (Join-Path $env:HOME ".local" "bin" "uv")
    )

    foreach ($path in $possiblePaths) {
        if (Test-Path $path) {
            return $path
        }
    }

    Write-Warning "UV not found. Install with: pip install uv"
    return $null
}

function Get-HFTokenFile {
    # 1. Check environment variable
    if ($env:HF_TOKEN_FILE -and (Test-Path $env:HF_TOKEN_FILE)) {
        return $env:HF_TOKEN_FILE
    }

    # 2. Check standard location
    $defaultPath = Join-Path $env:USERPROFILE ".cache" "huggingface" "token"
    if (Test-Path $defaultPath) {
        return $defaultPath
    }

    return $null
}

function Test-HFToken {
    $tokenFile = Get-HFTokenFile
    if ($tokenFile -and (Test-Path $tokenFile)) {
        $token = Get-Content $tokenFile -Raw -ErrorAction SilentlyContinue
        if ($token -and $token.Length -gt 10) {
            return $true
        }
    }
    return $false
}

function Get-GitHubToken {
    # 1. Check environment variable (REDACTED for security)
    if ($env:GITHUB_TOKEN) {
        return $env:GITHUB_TOKEN
    }

    if ($env:GITHUB_PERSONAL_ACCESS_TOKEN) {
        return $env:GITHUB_PERSONAL_ACCESS_TOKEN
    }

    return $null
}

function Get-ModelDirectory {
    # 1. Check environment variable
    if ($env:MODEL_DIR -and (Test-Path $env:MODEL_DIR)) {
        return $env:MODEL_DIR
    }

    # 2. Check standard locations
    $projectRoot = Get-ProjectRoot
    $possiblePaths = @(
        (Join-Path $projectRoot "models"),
        "T:\models",
        (Join-Path $env:USERPROFILE "models")
    )

    foreach ($path in $possiblePaths) {
        if (Test-Path $path) {
            return $path
        }
    }

    # 3. Create default if none exist
    $defaultPath = Join-Path $projectRoot "models"
    New-Item -ItemType Directory -Path $defaultPath -Force | Out-Null
    return $defaultPath
}

function Get-RAGRedisBinary {
    # 1. Check environment variable
    if ($env:RAG_REDIS_BINARY -and (Test-Path $env:RAG_REDIS_BINARY)) {
        return $env:RAG_REDIS_BINARY
    }

    # 2. Check if in PATH
    if (Get-Command rag-redis-mcp-server -ErrorAction SilentlyContinue) {
        return (Get-Command rag-redis-mcp-server).Source
    }

    # 3. Check standard locations
    $exeExt = if ($IsWindows -or $env:OS -eq "Windows_NT") { ".exe" } else { "" }
    $possiblePaths = @(
        (Join-Path $env:USERPROFILE "bin" "rag-redis-mcp-server$exeExt"),
        "C:\Program Files\RAG-Redis\rag-redis-mcp-server$exeExt"
    )

    foreach ($path in $possiblePaths) {
        if (Test-Path $path) {
            return $path
        }
    }

    Write-Warning "RAG-Redis binary not found. Some MCP tests will be skipped."
    return $null
}

# PSScriptAnalyzer: Disable PSAvoidUsingUnapprovedVerbs
function Set-PyO3Python {
    # Ensure PYO3_PYTHON is set for PyO3 compilation
    if ($env:PYO3_PYTHON -and (Test-Path $env:PYO3_PYTHON)) {
        Write-Verbose "PYO3_PYTHON already set: $env:PYO3_PYTHON"
        return $env:PYO3_PYTHON
    }

    try {
        $pyPath = (uv run -q -p 3.12 python -c "import sys; print(sys.executable)").Trim()
        if (Test-Path $pyPath) {
            $env:PYO3_PYTHON = $pyPath
            [System.Environment]::SetEnvironmentVariable('PYO3_PYTHON', $pyPath, 'User')
            Write-Verbose "PYO3_PYTHON set to: $pyPath"
            return $pyPath
        }
    }
    catch {
        Write-Warning "Failed to find Python 3.12 via uv. Install with: uv python install 3.12"
        throw "PYO3_PYTHON not configured. PyO3 compilation will fail."
    }
}
# PSScriptAnalyzer: Enable PSAvoidUsingUnapprovedVerbs

# PSScriptAnalyzer: Disable PSAvoidUsingUnapprovedVerbs
function Add-LocalBinToPath {
    $localBin = "C:\Users\david\.local\bin"
    if (-not ($env:PATH -split ';' | Where-Object { $_ -ieq $localBin })) {
        Write-Verbose "Adding $localBin to PATH"
        $env:PATH = "$env:PATH;$localBin"
        [System.Environment]::SetEnvironmentVariable('PATH', $env:PATH, 'User')
    }
    else {
        Write-Verbose "$localBin already in PATH"
    }
}
# PSScriptAnalyzer: Enable PSAvoidUsingUnapprovedVerbs

# Export all resolved paths as a hashtable
function Get-AllProjectPaths {
    Add-LocalBinToPath
    @{
        ProjectRoot    = Get-ProjectRoot
        Binary         = Get-MistralRSBinary
        UVPath         = Get-UVPath
        HFTokenFile    = Get-HFTokenFile
        HasHFToken     = Test-HFToken
        GitHubToken    = if (Get-GitHubToken) { "[REDACTED]" } else { $null }
        ModelDirectory = Get-ModelDirectory
        RAGRedisBinary = Get-RAGRedisBinary
        PyO3Python     = try { Set-PyO3Python } catch { $null }
    }
}

# If script is run directly, display all paths
if ($MyInvocation.InvocationName -ne '.') {
    $paths = Get-AllProjectPaths

    Write-Host "`n=== Project Paths Configuration ===" -ForegroundColor Cyan
    foreach ($key in $paths.Keys | Sort-Object) {
        $value = $paths[$key]
        if ($value) {
            Write-Host "$key : " -NoNewline -ForegroundColor Yellow
            Write-Host $value -ForegroundColor Green
        } else {
            Write-Host "$key : " -NoNewline -ForegroundColor Yellow
            Write-Host "[NOT CONFIGURED]" -ForegroundColor Gray
        }
    }
    Write-Host ""
}
