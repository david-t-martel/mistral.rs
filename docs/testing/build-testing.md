# Build System Testing Guide

## Overview

Build testing ensures that mistral.rs compiles correctly across all target platforms, with proper feature flags, optimizations, and binary validation. The project uses a comprehensive Makefile system that MUST be used for all builds.

## Core Principle: ALWAYS Use Makefile

```bash
# ✅ CORRECT - Always use make
make build
make build-cuda-full
make test

# ❌ WRONG - Never use bare cargo
cargo build --release
cargo test
```

The Makefile handles:
- Platform detection (Windows/Linux/macOS)
- CUDA environment setup
- Feature flag combinations
- Build caching with sccache
- Cross-compilation settings

## Build Targets

### Quick Reference Table

| Target | Purpose | Platform | Features | Time |
|--------|---------|----------|----------|------|
| `make check` | Syntax validation | All | N/A | < 1 min |
| `make dev` | Debug build | All | Basic | 2-5 min |
| `make build` | Release CPU | All | CPU only | 5-10 min |
| `make build-cuda` | CUDA basic | Win/Linux | CUDA | 15-20 min |
| `make build-cuda-full` | CUDA complete | Win/Linux | CUDA+Flash+cuDNN | 20-30 min |
| `make build-metal` | Metal acceleration | macOS | Metal | 10-15 min |
| `make release` | Optimized release | All | All available | 30-45 min |

## Platform-Specific Testing

### Windows CUDA Build Testing

**Test**: `scripts/build/test-windows-cuda.ps1`

```powershell
# Test Windows CUDA build configuration
function Test-WindowsCUDABuild {
    Write-Host "Testing Windows CUDA Build" -ForegroundColor Cyan

    # 1. Verify CUDA environment
    $cudaChecks = @{
        "CUDA Toolkit" = Test-Path "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9"
        "cuDNN" = Test-Path "C:\Program Files\NVIDIA\CUDNN\v9.8"
        "NVCC" = (Get-Command nvcc -ErrorAction SilentlyContinue) -ne $null
        "Visual Studio" = Test-Path "C:\Program Files\Microsoft Visual Studio\2022"
    }

    foreach ($check in $cudaChecks.GetEnumerator()) {
        if ($check.Value) {
            Write-Success "✓ $($check.Key) found"
        } else {
            Write-Error "✗ $($check.Key) missing"
            return $false
        }
    }

    # 2. Set NVCC_CCBIN for CUDA compilation
    $msvcPath = Get-ChildItem -Path "C:\Program Files\Microsoft Visual Studio\2022\*\VC\Tools\MSVC" -Recurse |
                Where-Object { $_.Name -eq "cl.exe" -and $_.Directory -like "*x64*" } |
                Select-Object -First 1

    if ($msvcPath) {
        $env:NVCC_CCBIN = $msvcPath.FullName
        Write-Success "NVCC_CCBIN set to: $env:NVCC_CCBIN"
    } else {
        Write-Error "Could not find MSVC compiler"
        return $false
    }

    # 3. Run CUDA build
    Write-Info "Starting CUDA build..."
    $buildStart = Get-Date

    $output = make build-cuda-full 2>&1
    $exitCode = $LASTEXITCODE

    $buildTime = (Get-Date) - $buildStart
    Write-Info "Build completed in: $([math]::Round($buildTime.TotalMinutes, 2)) minutes"

    if ($exitCode -eq 0) {
        Write-Success "CUDA build successful"

        # 4. Verify binary
        $binary = "target\release\mistralrs-server.exe"
        if (Test-Path $binary) {
            $fileInfo = Get-Item $binary
            Write-Info "Binary size: $([math]::Round($fileInfo.Length / 1MB, 2)) MB"

            # Check CUDA symbols in binary
            $strings = & strings $binary 2>$null | Select-String -Pattern "cuda|nvml|cublas|cudnn"
            if ($strings) {
                Write-Success "CUDA symbols found in binary"
            } else {
                Write-Warning "No CUDA symbols found - may not be linked correctly"
            }
        }

        return $true
    } else {
        Write-Error "Build failed with exit code: $exitCode"
        Write-Host "Last 20 lines of output:" -ForegroundColor Yellow
        $output | Select-Object -Last 20
        return $false
    }
}
```

### Linux Build Testing

**Test**: `scripts/build/test-linux-build.sh`

```bash
#!/bin/bash
set -e

echo "Testing Linux Build Configuration"

# 1. Check dependencies
check_dependency() {
    if command -v $1 &> /dev/null; then
        echo "✓ $1 found"
        return 0
    else
        echo "✗ $1 missing"
        return 1
    fi
}

# Required tools
check_dependency gcc
check_dependency cargo
check_dependency make

# Optional tools
check_dependency sccache || echo "  (optional - speeds up builds)"
check_dependency lld || echo "  (optional - faster linking)"

# 2. Check for CUDA (optional)
if [ -d "/usr/local/cuda" ]; then
    echo "✓ CUDA found at /usr/local/cuda"
    export CUDA_PATH="/usr/local/cuda"
    export LD_LIBRARY_PATH="$CUDA_PATH/lib64:$LD_LIBRARY_PATH"
    BUILD_TARGET="build-cuda-full"
else
    echo "ℹ CUDA not found - using CPU build"
    BUILD_TARGET="build"
fi

# 3. Run build
echo "Starting build with target: $BUILD_TARGET"
time make $BUILD_TARGET

# 4. Verify binary
BINARY="target/release/mistralrs-server"
if [ -f "$BINARY" ]; then
    echo "✓ Binary created: $(du -h $BINARY | cut -f1)"

    # Check linked libraries
    echo "Linked libraries:"
    ldd $BINARY | grep -E "(cuda|blas|mkl)" || echo "  No acceleration libraries linked"

    # Test run
    echo "Testing binary execution:"
    $BINARY --version
    echo "✓ Binary executes successfully"
else
    echo "✗ Binary not found at $BINARY"
    exit 1
fi
```

### macOS Metal Build Testing

**Test**: `scripts/build/test-macos-metal.sh`

```bash
#!/bin/bash
set -e

echo "Testing macOS Metal Build"

# 1. Verify Metal support
if system_profiler SPDisplaysDataType | grep -q "Metal"; then
    echo "✓ Metal support detected"
else
    echo "✗ No Metal support found"
    exit 1
fi

# 2. Check for Xcode tools
if xcode-select -p &> /dev/null; then
    echo "✓ Xcode Command Line Tools installed"
else
    echo "✗ Xcode Command Line Tools missing"
    echo "  Run: xcode-select --install"
    exit 1
fi

# 3. Build with Metal
echo "Starting Metal build..."
time make build-metal

# 4. Verify Metal framework linking
BINARY="target/release/mistralrs-server"
if otool -L $BINARY | grep -q "Metal.framework"; then
    echo "✓ Metal framework linked"
else
    echo "✗ Metal framework not linked"
    exit 1
fi

echo "✓ macOS Metal build successful"
```

## Feature Flag Testing

### Testing Feature Combinations

**Test**: `scripts/build/test-features.ps1`

```powershell
# Test various feature combinations
$featureSets = @(
    @{
        Name = "CPU Only"
        Features = ""
        Required = $true
    },
    @{
        Name = "CUDA Basic"
        Features = "cuda"
        Required = $false
    },
    @{
        Name = "CUDA Full"
        Features = "cuda,flash-attn,cudnn"
        Required = $false
    },
    @{
        Name = "MKL"
        Features = "mkl"
        Required = $false
    },
    @{
        Name = "Metal"
        Features = "metal"
        Required = ($env:OS -eq "Darwin")
    }
)

foreach ($set in $featureSets) {
    Write-Host "`nTesting feature set: $($set.Name)" -ForegroundColor Cyan

    try {
        if ($set.Features) {
            $output = cargo check --features $set.Features 2>&1
        } else {
            $output = cargo check 2>&1
        }

        if ($LASTEXITCODE -eq 0) {
            Write-Success "✓ $($set.Name) configuration valid"
        } else {
            throw "Feature check failed"
        }
    } catch {
        if ($set.Required) {
            Write-Error "✗ Required feature set failed: $($set.Name)"
            exit 1
        } else {
            Write-Warning "⚠ Optional feature set unavailable: $($set.Name)"
        }
    }
}
```

## Build Optimization Testing

### Binary Size Optimization

**Test**: `scripts/build/test-binary-optimization.ps1`

```powershell
# Test binary size optimization
function Test-BinaryOptimization {
    $configs = @(
        @{
            Name = "Debug"
            Target = "dev"
            ExpectedMaxMB = 2000
        },
        @{
            Name = "Release"
            Target = "build"
            ExpectedMaxMB = 500
        },
        @{
            Name = "Release+LTO"
            Target = "release"
            ExpectedMaxMB = 400
        },
        @{
            Name = "Release+Strip"
            Target = "release-stripped"
            ExpectedMaxMB = 350
        }
    )

    $results = @()

    foreach ($config in $configs) {
        Write-Info "Building $($config.Name) configuration..."

        # Build
        make $config.Target

        # Get binary path
        $binary = if ($config.Name -eq "Debug") {
            "target\debug\mistralrs-server.exe"
        } else {
            "target\release\mistralrs-server.exe"
        }

        if (Test-Path $binary) {
            $size = (Get-Item $binary).Length / 1MB
            $results += @{
                Config = $config.Name
                SizeMB = [math]::Round($size, 2)
                UnderLimit = $size -lt $config.ExpectedMaxMB
            }

            if ($size -lt $config.ExpectedMaxMB) {
                Write-Success "✓ $($config.Name): $([math]::Round($size, 2)) MB (< $($config.ExpectedMaxMB) MB)"
            } else {
                Write-Warning "⚠ $($config.Name): $([math]::Round($size, 2)) MB (> $($config.ExpectedMaxMB) MB expected)"
            }
        } else {
            Write-Error "Binary not found for $($config.Name)"
        }
    }

    # Summary
    Write-Host "`nBinary Size Summary:" -ForegroundColor Cyan
    $results | Format-Table -AutoSize

    # Check compression potential
    $releaseBinary = "target\release\mistralrs-server.exe"
    if (Test-Path $releaseBinary) {
        $compressed = [System.IO.Path]::GetTempFileName() + ".gz"
        & gzip -c $releaseBinary > $compressed
        $compressedSize = (Get-Item $compressed).Length / 1MB
        $compressionRatio = (1 - ($compressedSize / ((Get-Item $releaseBinary).Length / 1MB))) * 100

        Write-Info "Compressed size: $([math]::Round($compressedSize, 2)) MB"
        Write-Info "Compression ratio: $([math]::Round($compressionRatio, 1))%"

        Remove-Item $compressed
    }
}
```

### Build Performance Testing

**Test**: `scripts/build/test-build-performance.ps1`

```powershell
# Test build performance and caching
function Test-BuildPerformance {
    $iterations = 3
    $results = @{
        Cold = @()
        Warm = @()
        Incremental = @()
    }

    Write-Host "Testing build performance..." -ForegroundColor Cyan

    # 1. Cold build (clean cache)
    Write-Info "Cold build test (clean cache)..."
    make clean-all
    Remove-Item -Path "$env:USERPROFILE\.cargo\registry\cache" -Recurse -Force -ErrorAction SilentlyContinue

    for ($i = 1; $i -le $iterations; $i++) {
        make clean
        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
        make build
        $stopwatch.Stop()
        $results.Cold += $stopwatch.Elapsed.TotalSeconds
        Write-Info "  Iteration $i: $([math]::Round($stopwatch.Elapsed.TotalMinutes, 2)) minutes"
    }

    # 2. Warm build (with cache)
    Write-Info "Warm build test (with cache)..."

    for ($i = 1; $i -le $iterations; $i++) {
        make clean
        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
        make build
        $stopwatch.Stop()
        $results.Warm += $stopwatch.Elapsed.TotalSeconds
        Write-Info "  Iteration $i: $([math]::Round($stopwatch.Elapsed.TotalMinutes, 2)) minutes"
    }

    # 3. Incremental build (small change)
    Write-Info "Incremental build test..."

    # Make small change
    $testFile = "mistralrs-core/src/lib.rs"
    Add-Content $testFile "`n// Test comment"

    for ($i = 1; $i -le $iterations; $i++) {
        # Make another small change
        Add-Content $testFile "`n// Test $i"

        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
        make build
        $stopwatch.Stop()
        $results.Incremental += $stopwatch.Elapsed.TotalSeconds
        Write-Info "  Iteration $i: $([math]::Round($stopwatch.Elapsed.TotalSeconds, 2)) seconds"
    }

    # Clean up test changes
    git checkout -- $testFile

    # Calculate statistics
    $stats = @{}
    foreach ($type in $results.Keys) {
        $values = $results[$type]
        $stats[$type] = @{
            Average = ($values | Measure-Object -Average).Average
            Min = ($values | Measure-Object -Minimum).Minimum
            Max = ($values | Measure-Object -Maximum).Maximum
        }
    }

    # Display results
    Write-Host "`nBuild Performance Summary:" -ForegroundColor Cyan
    Write-Host "Cold Build:        $([math]::Round($stats.Cold.Average/60, 2)) minutes avg"
    Write-Host "Warm Build:        $([math]::Round($stats.Warm.Average/60, 2)) minutes avg"
    Write-Host "Incremental Build: $([math]::Round($stats.Incremental.Average, 2)) seconds avg"

    # Calculate cache effectiveness
    $cacheSpeedup = ($stats.Cold.Average - $stats.Warm.Average) / $stats.Cold.Average * 100
    Write-Info "Cache speedup: $([math]::Round($cacheSpeedup, 1))%"

    # Check if sccache is working
    if (Get-Command sccache -ErrorAction SilentlyContinue) {
        $sccacheStats = sccache --show-stats
        Write-Info "sccache statistics:"
        Write-Host $sccacheStats
    }

    return $stats
}
```

## Cross-Compilation Testing

### Testing Cross-Platform Builds

**Test**: `scripts/build/test-cross-compilation.ps1`

```powershell
# Test cross-compilation to different targets
function Test-CrossCompilation {
    $targets = @(
        @{
            Triple = "x86_64-pc-windows-msvc"
            Name = "Windows x64"
            Available = $true
        },
        @{
            Triple = "x86_64-unknown-linux-gnu"
            Name = "Linux x64"
            Available = $env:WSL_DISTRO_NAME -or (Get-Command wsl -ErrorAction SilentlyContinue)
        },
        @{
            Triple = "x86_64-apple-darwin"
            Name = "macOS x64"
            Available = $false  # Requires macOS SDK
        },
        @{
            Triple = "aarch64-unknown-linux-gnu"
            Name = "Linux ARM64"
            Available = (Get-Command aarch64-linux-gnu-gcc -ErrorAction SilentlyContinue) -ne $null
        }
    )

    foreach ($target in $targets) {
        Write-Host "`nTesting cross-compilation to $($target.Name)" -ForegroundColor Cyan

        if (-not $target.Available) {
            Write-Warning "Skipping $($target.Name) - toolchain not available"
            continue
        }

        # Add target
        Write-Info "Adding target: $($target.Triple)"
        rustup target add $target.Triple

        # Attempt compilation (check only, don't link)
        Write-Info "Checking compilation for $($target.Triple)..."
        $output = cargo check --target $target.Triple 2>&1
        $exitCode = $LASTEXITCODE

        if ($exitCode -eq 0) {
            Write-Success "✓ $($target.Name) compilation check passed"
        } else {
            Write-Error "✗ $($target.Name) compilation failed"
            Write-Host "Error output:" -ForegroundColor Yellow
            $output | Select-Object -Last 10
        }
    }
}
```

## Dependency Testing

### Dependency Audit

**Test**: `scripts/build/test-dependencies.ps1`

```powershell
# Test dependency security and compatibility
function Test-Dependencies {
    Write-Host "Testing Dependencies" -ForegroundColor Cyan

    # 1. Security audit
    Write-Info "Running security audit..."
    cargo audit

    if ($LASTEXITCODE -eq 0) {
        Write-Success "✓ No known vulnerabilities"
    } else {
        Write-Warning "⚠ Security issues found - review above"
    }

    # 2. Check for duplicate dependencies
    Write-Info "Checking for duplicate dependencies..."
    $tree = cargo tree --duplicates

    if ($tree) {
        Write-Warning "Duplicate dependencies found:"
        Write-Host $tree
    } else {
        Write-Success "✓ No duplicate dependencies"
    }

    # 3. Check dependency licenses
    Write-Info "Checking licenses..."
    if (Get-Command cargo-license -ErrorAction SilentlyContinue) {
        cargo license --json | ConvertFrom-Json |
            Group-Object -Property license |
            ForEach-Object {
                Write-Host "  $($_.Name): $($_.Count) packages"
            }
    } else {
        Write-Warning "cargo-license not installed - skipping license check"
    }

    # 4. Check for outdated dependencies
    Write-Info "Checking for outdated dependencies..."
    if (Get-Command cargo-outdated -ErrorAction SilentlyContinue) {
        cargo outdated
    } else {
        Write-Warning "cargo-outdated not installed - skipping"
    }

    # 5. Analyze dependency tree size
    Write-Info "Analyzing dependency tree..."
    $treeOutput = cargo tree --prefix none 2>&1
    $uniqueDeps = $treeOutput | Select-String -Pattern "^[a-z]" | Sort-Object -Unique
    Write-Info "Total unique dependencies: $($uniqueDeps.Count)"

    # Check specific critical dependencies
    $criticalDeps = @("candle-core", "tokio", "axum", "pyo3")
    foreach ($dep in $criticalDeps) {
        $version = cargo tree -p $dep 2>&1 | Select-String -Pattern "$dep v([\d.]+)" |
                   ForEach-Object { $_.Matches[0].Groups[1].Value }
        if ($version) {
            Write-Info "  $dep version: $version"
        }
    }
}
```

## Build Validation Testing

### Post-Build Validation

**Test**: `scripts/build/test-build-validation.ps1`

```powershell
# Comprehensive post-build validation
function Test-BuildValidation {
    param(
        [string]$Binary = "target\release\mistralrs-server.exe"
    )

    Write-Host "Validating Build Output" -ForegroundColor Cyan
    $validationResults = @{}

    # 1. Binary exists
    if (Test-Path $Binary) {
        $validationResults["Binary Exists"] = $true
        $fileInfo = Get-Item $Binary
        Write-Success "✓ Binary found: $([math]::Round($fileInfo.Length / 1MB, 2)) MB"
    } else {
        Write-Error "✗ Binary not found at: $Binary"
        return $false
    }

    # 2. Version check
    $version = & $Binary --version 2>&1
    if ($version -match "mistralrs-server (\d+\.\d+\.\d+)") {
        $validationResults["Version"] = $matches[1]
        Write-Success "✓ Version: $($matches[1])"
    } else {
        Write-Error "✗ Invalid version output"
    }

    # 3. Help output
    $help = & $Binary --help 2>&1
    if ($help -match "USAGE:") {
        $validationResults["Help Output"] = $true
        Write-Success "✓ Help command works"
    } else {
        Write-Error "✗ Help command failed"
    }

    # 4. Subcommands available
    $expectedSubcommands = @("gguf", "ggml", "plain", "diffusion", "speech")
    foreach ($cmd in $expectedSubcommands) {
        if ($help -match $cmd) {
            Write-Success "  ✓ Subcommand: $cmd"
        } else {
            Write-Warning "  ⚠ Missing subcommand: $cmd"
        }
    }

    # 5. Feature detection
    Write-Info "Detecting compiled features..."

    # Check for CUDA
    if ($Binary -match "cuda" -or (& strings $Binary 2>$null | Select-String "cuda")) {
        Write-Success "  ✓ CUDA support detected"
        $validationResults["CUDA"] = $true
    }

    # Check for Flash Attention
    if (& strings $Binary 2>$null | Select-String "flash_attn") {
        Write-Success "  ✓ Flash Attention support detected"
        $validationResults["FlashAttn"] = $true
    }

    # 6. Dynamic library dependencies
    Write-Info "Checking dependencies..."

    if ($IsWindows) {
        # Use dumpbin or Dependencies.exe if available
        if (Get-Command dumpbin -ErrorAction SilentlyContinue) {
            $deps = dumpbin /DEPENDENTS $Binary
            Write-Host "Dependencies:" -ForegroundColor Yellow
            $deps | Select-String "\.dll" | ForEach-Object { Write-Host "  $_" }
        }
    } else {
        $deps = ldd $Binary
        Write-Host "Linked libraries:" -ForegroundColor Yellow
        Write-Host $deps
    }

    # 7. Dry run test
    Write-Info "Testing dry run..."
    $testModel = "C:\codedev\llm\.models\qwen2.5-1.5b-it-gguf"
    $testFile = "Qwen2.5-1.5B-Instruct-Q4_K_M.gguf"

    if (Test-Path "$testModel\$testFile") {
        $dryRun = & $Binary gguf -m $testModel -f $testFile --dry-run 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Success "✓ Dry run successful"
            $validationResults["DryRun"] = $true
        } else {
            Write-Warning "⚠ Dry run failed - may need model files"
        }
    } else {
        Write-Warning "Test model not found - skipping dry run"
    }

    # Summary
    Write-Host "`nValidation Summary:" -ForegroundColor Cyan
    $validationResults.GetEnumerator() | ForEach-Object {
        Write-Host "  $($_.Key): $($_.Value)"
    }

    $passed = ($validationResults.Values | Where-Object { $_ -eq $true }).Count
    $total = $validationResults.Count
    Write-Host "Passed: $passed/$total checks" -ForegroundColor $(if ($passed -eq $total) { "Green" } else { "Yellow" })

    return ($passed -eq $total)
}
```

## Makefile Testing

### Testing Makefile Targets

**Test**: `scripts/build/test-makefile.ps1`

```powershell
# Test all Makefile targets
function Test-MakefileTargets {
    Write-Host "Testing Makefile Targets" -ForegroundColor Cyan

    # Get all targets from Makefile
    $makefileContent = Get-Content "Makefile"
    $targets = $makefileContent |
               Select-String -Pattern "^([a-z-]+):" |
               ForEach-Object { $_.Matches[0].Groups[1].Value } |
               Where-Object { $_ -notmatch "^(all|help)$" }

    $results = @()

    foreach ($target in $targets) {
        Write-Host "`nTesting: make $target" -ForegroundColor Yellow

        # Some targets need special handling
        $testableTargets = @(
            "check", "fmt-check", "lint-check", "test-check",
            "clean", "clean-tests", "clean-cache"
        )

        if ($target -in $testableTargets) {
            $output = make $target 2>&1
            $exitCode = $LASTEXITCODE

            $results += @{
                Target = $target
                Success = ($exitCode -eq 0)
                ExitCode = $exitCode
            }

            if ($exitCode -eq 0) {
                Write-Success "✓ make $target succeeded"
            } else {
                Write-Error "✗ make $target failed (exit: $exitCode)"
            }
        } else {
            Write-Info "⊘ Skipping $target (requires full build)"
            $results += @{
                Target = $target
                Success = $null
                ExitCode = $null
            }
        }
    }

    # Summary
    Write-Host "`nMakefile Target Summary:" -ForegroundColor Cyan
    $results | Format-Table -AutoSize

    $tested = $results | Where-Object { $_.Success -ne $null }
    $passed = $tested | Where-Object { $_.Success -eq $true }
    Write-Host "Tested: $($tested.Count), Passed: $($passed.Count)" -ForegroundColor $(
        if ($passed.Count -eq $tested.Count) { "Green" } else { "Yellow" }
    )
}
```

## CI/CD Build Testing

### GitHub Actions Simulation

**Test**: `scripts/build/test-ci-build.ps1`

```powershell
# Simulate CI build locally
function Test-CIBuild {
    Write-Host "Simulating CI Build Pipeline" -ForegroundColor Cyan

    $steps = @(
        @{ Name = "Clean workspace"; Command = "make clean-all" },
        @{ Name = "Check formatting"; Command = "make fmt-check" },
        @{ Name = "Run linter"; Command = "make lint-check" },
        @{ Name = "Check compilation"; Command = "make check" },
        @{ Name = "Build release"; Command = "make build" },
        @{ Name = "Run tests"; Command = "make test" },
        @{ Name = "Build documentation"; Command = "cargo doc --no-deps" }
    )

    $results = @()
    $failed = $false

    foreach ($step in $steps) {
        Write-Host "`n[$($steps.IndexOf($step)+1)/$($steps.Count)] $($step.Name)" -ForegroundColor Cyan

        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
        $output = & cmd /c $step.Command 2>&1
        $exitCode = $LASTEXITCODE
        $stopwatch.Stop()

        $results += @{
            Step = $step.Name
            Success = ($exitCode -eq 0)
            Duration = $stopwatch.Elapsed.TotalSeconds
            ExitCode = $exitCode
        }

        if ($exitCode -eq 0) {
            Write-Success "✓ $($step.Name) passed in $([math]::Round($stopwatch.Elapsed.TotalSeconds, 2))s"
        } else {
            Write-Error "✗ $($step.Name) failed with exit code $exitCode"
            if ($env:CI -or $failed) {
                Write-Host "Stopping CI simulation due to failure" -ForegroundColor Red
                break
            }
            $failed = $true
        }
    }

    # Generate CI report
    Write-Host "`nCI Build Report:" -ForegroundColor Cyan
    $results | Format-Table -AutoSize

    $totalTime = ($results | Measure-Object -Property Duration -Sum).Sum
    Write-Host "Total CI time: $([math]::Round($totalTime/60, 2)) minutes"

    if (-not $failed) {
        Write-Success "✓ CI build simulation passed!"
        return 0
    } else {
        Write-Error "✗ CI build simulation failed"
        return 1
    }
}
```

## Performance Benchmarks

### Build Time Benchmarks

```powershell
# Benchmark different build configurations
function Benchmark-BuildTimes {
    $configurations = @(
        @{ Name = "Debug"; Target = "dev" },
        @{ Name = "Release"; Target = "build" },
        @{ Name = "Release+LTO"; Target = "release" },
        @{ Name = "CUDA"; Target = "build-cuda" },
        @{ Name = "CUDA Full"; Target = "build-cuda-full" }
    )

    $results = @()

    foreach ($config in $configurations) {
        Write-Host "`nBenchmarking: $($config.Name)" -ForegroundColor Cyan

        # Clean first
        make clean | Out-Null

        # Measure build time
        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
        make $config.Target 2>&1 | Out-Null
        $stopwatch.Stop()

        if ($LASTEXITCODE -eq 0) {
            $results += @{
                Configuration = $config.Name
                Time = [math]::Round($stopwatch.Elapsed.TotalMinutes, 2)
                Success = $true
            }
            Write-Success "✓ Built in $([math]::Round($stopwatch.Elapsed.TotalMinutes, 2)) minutes"
        } else {
            $results += @{
                Configuration = $config.Name
                Time = "N/A"
                Success = $false
            }
            Write-Error "✗ Build failed"
        }
    }

    # Display results
    Write-Host "`nBuild Time Comparison:" -ForegroundColor Cyan
    $results | Format-Table -AutoSize

    # Chart visualization (if possible)
    if ($results | Where-Object { $_.Success }) {
        $chart = ""
        foreach ($r in ($results | Where-Object { $_.Success })) {
            $bars = "█" * [math]::Min(50, [int]($r.Time * 2))
            $chart += "$($r.Configuration.PadRight(15)) $bars $($r.Time)m`n"
        }
        Write-Host "`nBuild Time Chart:" -ForegroundColor Cyan
        Write-Host $chart
    }
}
```

## Common Issues and Solutions

### Issue: "NVCC not found"

**Solution**:
```powershell
# Set CUDA environment
$env:CUDA_PATH = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9"
$env:PATH = "$env:CUDA_PATH\bin;$env:PATH"

# Set NVCC host compiler
$env:NVCC_CCBIN = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.40.33807\bin\Hostx64\x64\cl.exe"

# Verify
nvcc --version
```

### Issue: "Out of memory during linking"

**Solution**:
```bash
# Increase linker memory
export RUSTFLAGS="-C link-arg=/STACK:8388608"

# Or use mold linker (Linux)
export RUSTFLAGS="-C linker=clang -C link-arg=-fuse-ld=mold"

# Reduce parallel jobs
make build JOBS=4
```

### Issue: "Build takes too long"

**Solution**:
```bash
# Enable sccache
cargo install sccache
export RUSTC_WRAPPER=sccache

# Check sccache stats
sccache --show-stats

# Use incremental compilation
export CARGO_INCREMENTAL=1

# Build specific package only
cargo build -p mistralrs-core
```

## Next Steps

- Review [Integration Testing](integration-testing.md) for runtime testing
- See [MCP Testing](mcp-testing.md) for protocol testing
- Check [CI/CD Testing](ci-cd-testing.md) for automation
- Read [Testing Migration](../development/testing-migration.md) for updates

---

*Last Updated: 2025*
*Version: 1.0.0*
*Component: Build System Testing*