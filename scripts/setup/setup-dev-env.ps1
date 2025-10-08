# Mistral.rs Development Environment Setup Script
# Creates symlinks and configures environment for building with CUDA, MKL, and Python

param(
    [switch]$Elevated
)

# Check for admin privileges
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

Write-Host "==> Mistral.rs Development Environment Setup" -ForegroundColor Cyan
Write-Host ""

# ============================================================================
# 1. Python Setup via UV
# ============================================================================
Write-Host "[1/6] Setting up Python via UV..." -ForegroundColor Green

$uvPath = "C:\users\david\.local\bin\uv.exe"
if (Test-Path $uvPath) {
    Write-Host "  ✓ UV found at: $uvPath" -ForegroundColor Gray

    # Get the default Python installation
    $pythonVersions = & $uvPath python list 2>&1 | Select-String "cpython-3\.(1[23])\.\d+" | Where-Object { $_ -match "C:\\" }

    if ($pythonVersions) {
        $latestPython = $pythonVersions | Select-Object -First 1
        if ($latestPython -match "(C:\\[^\\s]+python\.exe)") {
            $pythonExe = $matches[1]
            Write-Host "  ✓ Latest Python: $pythonExe" -ForegroundColor Gray

            # Create symlinks in .local\bin
            $symlinkTargets = @{
                "C:\users\david\.local\bin\python.exe" = $pythonExe
                "C:\users\david\.local\bin\python3.exe" = $pythonExe
            }

            foreach ($link in $symlinkTargets.Keys) {
                $target = $symlinkTargets[$link]
                if (Test-Path $link) {
                    $existingTarget = (Get-Item $link).Target
                    if ($existingTarget -eq $target) {
                        Write-Host "  ✓ Symlink exists: $link -> $target" -ForegroundColor Gray
                    } else {
                        Write-Host "  ! Symlink exists but points elsewhere: $link" -ForegroundColor Yellow
                    }
                } else {
                    if ($isAdmin) {
                        New-Item -ItemType SymbolicLink -Path $link -Target $target -Force | Out-Null
                        Write-Host "  ✓ Created symlink: $link -> $target" -ForegroundColor Green
                    } else {
                        Write-Host "  ! Need admin rights to create: $link" -ForegroundColor Yellow
                    }
                }
            }

            # Also create in C:\users\david\bin
            $binDir = "C:\users\david\bin"
            if (Test-Path $binDir) {
                $symlinkTargets2 = @{
                    "$binDir\python.exe" = $pythonExe
                    "$binDir\python3.exe" = $pythonExe
                }

                foreach ($link in $symlinkTargets2.Keys) {
                    $target = $symlinkTargets2[$link]
                    if (-not (Test-Path $link)) {
                        if ($isAdmin) {
                            New-Item -ItemType SymbolicLink -Path $link -Target $target -Force | Out-Null
                            Write-Host "  ✓ Created symlink: $link -> $target" -ForegroundColor Green
                        }
                    }
                }
            }
        }
    }
} else {
    Write-Host "  ✗ UV not found. Install from: https://github.com/astral-sh/uv" -ForegroundColor Red
}

# ============================================================================
# 2. Visual Studio Build Tools Setup
# ============================================================================
Write-Host ""
Write-Host "[2/6] Configuring Visual Studio Build Tools..." -ForegroundColor Green

$vsPath = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools"
$vswhere = "C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe"

if (Test-Path $vsPath) {
    Write-Host "  ✓ VS 2022 Build Tools found at: $vsPath" -ForegroundColor Gray

    # Find MSVC version
    $msvcVersions = Get-ChildItem "$vsPath\VC\Tools\MSVC" | Sort-Object Name -Descending
    if ($msvcVersions) {
        $latestMSVC = $msvcVersions[0].Name
        $clPath = "$vsPath\VC\Tools\MSVC\$latestMSVC\bin\Hostx64\x64\cl.exe"

        if (Test-Path $clPath) {
            Write-Host "  ✓ MSVC compiler: $latestMSVC" -ForegroundColor Gray
            Write-Host "  ✓ cl.exe at: $clPath" -ForegroundColor Gray

            # Set environment variable for this session
            $env:NVCC_CCBIN = $clPath
            Write-Host "  ✓ Set NVCC_CCBIN=$clPath" -ForegroundColor Green

            # Create symlink to vswhere in .local\bin
            $vswhereLink = "C:\users\david\.local\bin\vswhere.exe"
            if (-not (Test-Path $vswhereLink) -and (Test-Path $vswhere)) {
                if ($isAdmin) {
                    New-Item -ItemType SymbolicLink -Path $vswhereLink -Target $vswhere -Force | Out-Null
                    Write-Host "  ✓ Created symlink: vswhere.exe" -ForegroundColor Green
                }
            }
        }
    }
} else {
    Write-Host "  ✗ Visual Studio 2022 Build Tools not found" -ForegroundColor Red
    Write-Host "    Install from: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022" -ForegroundColor Yellow
}

# ============================================================================
# 3. CUDA Setup
# ============================================================================
Write-Host ""
Write-Host "[3/6] Verifying CUDA installation..." -ForegroundColor Green

$cudaPath = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9"
if (Test-Path $cudaPath) {
    Write-Host "  ✓ CUDA 12.9 found at: $cudaPath" -ForegroundColor Gray
    $nvccPath = "$cudaPath\bin\nvcc.exe"
    if (Test-Path $nvccPath) {
        Write-Host "  ✓ nvcc.exe found" -ForegroundColor Gray
    }

    # Set CUDA_PATH for this session
    $env:CUDA_PATH = $cudaPath
    Write-Host "  ✓ Set CUDA_PATH=$cudaPath" -ForegroundColor Green
} else {
    Write-Host "  ✗ CUDA 12.9 not found" -ForegroundColor Red
}

# ============================================================================
# 4. cuDNN Setup
# ============================================================================
Write-Host ""
Write-Host "[4/6] Verifying cuDNN installation..." -ForegroundColor Green

$cudnnPath = "C:\Program Files\NVIDIA\CUDNN"
if (Test-Path $cudnnPath) {
    Write-Host "  ✓ cuDNN found at: $cudnnPath" -ForegroundColor Gray

    # Check for cuda subdirectory structure
    $cudnnBin = "$cudnnPath\cuda\bin"
    if (Test-Path $cudnnBin) {
        Write-Host "  ✓ cuDNN bin directory exists" -ForegroundColor Gray
    } else {
        # Look for version-specific directories
        $cudnnDirs = Get-ChildItem $cudnnPath -Directory | Where-Object { $_.Name -match "^v\d" }
        if ($cudnnDirs) {
            $latestCudnn = $cudnnDirs | Sort-Object Name -Descending | Select-Object -First 1
            Write-Host "  ✓ cuDNN version: $($latestCudnn.Name)" -ForegroundColor Gray
            $cudnnBin = "$($latestCudnn.FullName)\bin"
        }
    }

    $env:CUDNN_PATH = $cudnnPath
    Write-Host "  ✓ Set CUDNN_PATH=$cudnnPath" -ForegroundColor Green
} else {
    Write-Host "  ✗ cuDNN not found at expected location" -ForegroundColor Red
}

# ============================================================================
# 5. Intel MKL Setup
# ============================================================================
Write-Host ""
Write-Host "[5/6] Configuring Intel MKL..." -ForegroundColor Green

$mklPath = "C:\Program Files (x86)\Intel\oneAPI\mkl\latest"
if (Test-Path $mklPath) {
    Write-Host "  ✓ Intel MKL found at: $mklPath" -ForegroundColor Gray
    $env:MKLROOT = $mklPath
    $env:ONEAPI_ROOT = "C:\Program Files (x86)\Intel\oneAPI"
    Write-Host "  ✓ Set MKLROOT=$mklPath" -ForegroundColor Green
} else {
    # Try to find any MKL version
    $mklBase = "C:\Program Files (x86)\Intel\oneAPI\mkl"
    if (Test-Path $mklBase) {
        $mklVersions = Get-ChildItem $mklBase -Directory | Sort-Object Name -Descending
        if ($mklVersions) {
            $mklPath = $mklVersions[0].FullName
            Write-Host "  ✓ Intel MKL found: $($mklVersions[0].Name)" -ForegroundColor Gray
            $env:MKLROOT = $mklPath
        }
    } else {
        Write-Host "  ✗ Intel MKL not found" -ForegroundColor Red
    }
}

# ============================================================================
# 6. Hugging Face Setup
# ============================================================================
Write-Host ""
Write-Host "[6/6] Configuring Hugging Face..." -ForegroundColor Green

# Set HF_HOME to store cache in C:\codedev\llm\.cache
$hfHome = "C:\codedev\llm\.cache\huggingface"
if (-not (Test-Path $hfHome)) {
    New-Item -ItemType Directory -Path $hfHome -Force | Out-Null
    Write-Host "  ✓ Created HF_HOME directory: $hfHome" -ForegroundColor Green
} else {
    Write-Host "  ✓ HF_HOME directory exists: $hfHome" -ForegroundColor Gray
}

$env:HF_HOME = $hfHome
Write-Host "  ✓ Set HF_HOME=$hfHome" -ForegroundColor Green

# Check for HF token
$hfTokenFile = "$env:USERPROFILE\.cache\huggingface\token"
if (Test-Path $hfTokenFile) {
    Write-Host "  ✓ Hugging Face token found" -ForegroundColor Gray
} else {
    Write-Host "  ! No HF token found. Run: huggingface-cli login" -ForegroundColor Yellow
}

# Check if huggingface-cli is available
$hfCli = Get-Command huggingface-cli -ErrorAction SilentlyContinue
if ($hfCli) {
    Write-Host "  ✓ huggingface-cli available" -ForegroundColor Gray
} else {
    Write-Host "  ! huggingface-cli not found. Install: pip install huggingface-hub[cli]" -ForegroundColor Yellow
}

# ============================================================================
# Summary & Next Steps
# ============================================================================
Write-Host ""
Write-Host "==> Environment Configuration Complete" -ForegroundColor Cyan
Write-Host ""
Write-Host "Environment Variables Set (Current Session):" -ForegroundColor White
Write-Host "  NVCC_CCBIN = $env:NVCC_CCBIN" -ForegroundColor Gray
Write-Host "  CUDA_PATH = $env:CUDA_PATH" -ForegroundColor Gray
Write-Host "  CUDNN_PATH = $env:CUDNN_PATH" -ForegroundColor Gray
Write-Host "  MKLROOT = $env:MKLROOT" -ForegroundColor Gray
Write-Host "  HF_HOME = $env:HF_HOME" -ForegroundColor Gray
Write-Host ""
Write-Host "Next Steps:" -ForegroundColor White
Write-Host "  1. To persist these settings, run:" -ForegroundColor Yellow
Write-Host "     [Environment]::SetEnvironmentVariable('HF_HOME', '$hfHome', 'User')" -ForegroundColor Gray
Write-Host ""
Write-Host "  2. To build mistral.rs with CUDA:" -ForegroundColor Yellow
Write-Host "     cargo build --release --package mistralrs-server --features `"cuda flash-attn cudnn mkl`"" -ForegroundColor Gray
Write-Host ""
Write-Host "  3. To download Gemma 3 model (requires HF token):" -ForegroundColor Yellow
Write-Host "     huggingface-cli download google/gemma-3-4b-it --local-dir C:\codedev\llm\.models\gemma-3-4b-it" -ForegroundColor Gray
Write-Host ""

if (-not $isAdmin) {
    Write-Host "Note: Some symlinks could not be created. Re-run with admin privileges:" -ForegroundColor Yellow
    Write-Host "  Start-Process powershell -Verb RunAs -ArgumentList '-File',
'$PSCommandPath'" -ForegroundColor Gray
}
