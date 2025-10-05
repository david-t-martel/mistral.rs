# Windows Coreutils Validation Script
# Validates functionality of all utilities

param(
    [string]$BuildType = "release",
    [switch]$Verbose = $false,
    [switch]$StopOnError = $false,
    [switch]$GenerateReport = $false
)

$ErrorActionPreference = if ($StopOnError) { "Stop" } else { "Continue" }
$VerbosePreference = if ($Verbose) { "Continue" } else { "SilentlyContinue" }

# Configuration
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$TargetDir = Join-Path $ProjectRoot "target" $BuildType
$ReportFile = Join-Path $ProjectRoot "validation-report.json"
$TempDir = Join-Path $env:TEMP "winutils-validation"

# Utility lists
$CoreUtils = @(
    "arch", "b2sum", "b3sum", "base32", "base64", "basename", "basenc",
    "cat", "chgrp", "chmod", "chown", "chroot", "cksum", "comm", "cp",
    "csplit", "cut", "date", "dd", "df", "dir", "dircolors", "dirname",
    "du", "echo", "env", "expand", "expr", "factor", "false", "fmt",
    "fold", "groups", "hashsum", "head", "hostid", "hostname", "id",
    "install", "join", "kill", "link", "ln", "logname", "ls", "md5sum",
    "mkdir", "mkfifo", "mknod", "mktemp", "more", "mv", "nice", "nl",
    "nohup", "nproc", "numfmt", "od", "paste", "pathchk", "pinky", "pr",
    "printenv", "printf", "ptx", "pwd", "readlink", "realpath", "relpath",
    "rm", "rmdir", "runcon", "seq", "sha1sum", "sha224sum", "sha256sum",
    "sha384sum", "sha3-224sum", "sha3-256sum", "sha3-384sum", "sha3-512sum",
    "sha3sum", "sha512sum", "shake128sum", "shake256sum", "shred", "shuf",
    "sleep", "sort", "split", "stat", "stdbuf", "stty", "sum", "sync",
    "tac", "tail", "tee", "test", "timeout", "touch", "tr", "true",
    "truncate", "tsort", "tty", "uname", "unexpand", "uniq", "unlink",
    "uptime", "users", "vdir", "wc", "who", "whoami", "yes"
)

$DeriveUtils = @("where", "which", "tree")
$AllUtils = $CoreUtils + $DeriveUtils

# Test results storage
$Results = @{
    Total = 0
    Passed = 0
    Failed = 0
    Skipped = 0
    Details = @()
    Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    BuildType = $BuildType
}

# Create temp directory
if (-not (Test-Path $TempDir)) {
    New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
}

# Helper functions
function Test-Binary {
    param([string]$Name, [string]$Path)

    $result = @{
        Name = $Name
        Path = $Path
        Exists = $false
        Version = $null
        Help = $false
        BasicTest = $false
        PathTest = $false
        ErrorMessage = $null
    }

    try {
        # Check if binary exists
        if (-not (Test-Path $Path)) {
            throw "Binary not found"
        }
        $result.Exists = $true

        # Test --version
        $versionOutput = & $Path --version 2>&1
        if ($LASTEXITCODE -eq 0) {
            $result.Version = $versionOutput | Select-Object -First 1
        }

        # Test --help
        $helpOutput = & $Path --help 2>&1
        if ($LASTEXITCODE -eq 0 -and $helpOutput) {
            $result.Help = $true
        }

        # Perform basic functionality test based on utility type
        $result.BasicTest = Test-BasicFunctionality $Name $Path

        # Test path handling (Windows-specific)
        $result.PathTest = Test-PathHandling $Name $Path

    } catch {
        $result.ErrorMessage = $_.Exception.Message
    }

    return $result
}

function Test-BasicFunctionality {
    param([string]$Name, [string]$Path)

    try {
        switch ($Name) {
            # Text utilities
            { $_ -in @("echo", "printf") } {
                $output = & $Path "test" 2>&1
                return $output -eq "test"
            }

            # File utilities
            { $_ -in @("cat", "head", "tail") } {
                $testFile = Join-Path $TempDir "test.txt"
                "test content" | Out-File $testFile -Encoding UTF8
                $output = & $Path $testFile 2>&1
                Remove-Item $testFile -Force
                return $output -like "*test content*"
            }

            # Directory utilities
            { $_ -in @("ls", "dir", "vdir") } {
                $output = & $Path $TempDir 2>&1
                return $LASTEXITCODE -eq 0
            }

            # Info utilities
            { $_ -in @("pwd", "whoami", "hostname", "uname") } {
                $output = & $Path 2>&1
                return $output -and $LASTEXITCODE -eq 0
            }

            # Boolean utilities
            { $_ -in @("true", "false") } {
                & $Path 2>&1
                $expectedCode = if ($Name -eq "true") { 0 } else { 1 }
                return $LASTEXITCODE -eq $expectedCode
            }

            # Math utilities
            { $_ -in @("expr", "factor") } {
                $output = & $Path "2" "+" "2" 2>&1
                return $output -or $LASTEXITCODE -eq 0
            }

            # Hash utilities
            { $_ -like "*sum" } {
                $testFile = Join-Path $TempDir "hash.txt"
                "test" | Out-File $testFile -Encoding UTF8
                $output = & $Path $testFile 2>&1
                Remove-Item $testFile -Force
                return $output -and $LASTEXITCODE -eq 0
            }

            # Default: just check if it runs without crashing
            default {
                & $Path --version 2>&1
                return $LASTEXITCODE -eq 0
            }
        }
    } catch {
        return $false
    }
}

function Test-PathHandling {
    param([string]$Name, [string]$Path)

    # Only test utilities that work with paths
    if ($Name -notin @("cat", "ls", "cp", "mv", "rm", "mkdir", "stat", "readlink", "realpath")) {
        return $true
    }

    try {
        # Test different path formats
        $testPaths = @(
            "C:\Windows\System32",           # DOS path
            "C:/Windows/System32",            # DOS with forward slashes
            "/mnt/c/Windows/System32",        # WSL path
            "/cygdrive/c/Windows/System32",  # Cygwin path
            "\\?\C:\Windows\System32",        # UNC path
            ".\relative\path",                # Relative path
            "..\parent\path"                  # Parent relative path
        )

        foreach ($testPath in $testPaths) {
            switch ($Name) {
                "ls" {
                    if (Test-Path $testPath) {
                        & $Path $testPath 2>&1 | Out-Null
                        if ($LASTEXITCODE -ne 0) { return $false }
                    }
                }
                "stat" {
                    if (Test-Path $testPath) {
                        & $Path $testPath 2>&1 | Out-Null
                        if ($LASTEXITCODE -ne 0) { return $false }
                    }
                }
                default {
                    # Skip detailed testing for other utilities
                    return $true
                }
            }
        }
        return $true
    } catch {
        return $false
    }
}

# Main validation loop
Write-Host "`n=== Windows Coreutils Validation ===" -ForegroundColor Cyan
Write-Host "Build Type: $BuildType" -ForegroundColor Yellow
Write-Host "Target Directory: $TargetDir" -ForegroundColor Yellow
Write-Host "`nValidating utilities..." -ForegroundColor Yellow

foreach ($util in $AllUtils) {
    $Results.Total++
    $binaryName = if ($util -in $CoreUtils) { "uu_$util.exe" } else { "$util.exe" }
    $binaryPath = Join-Path $TargetDir $binaryName

    Write-Host -NoNewline "Testing $util... "

    $testResult = Test-Binary -Name $util -Path $binaryPath

    if (-not $testResult.Exists) {
        Write-Host "SKIPPED (not built)" -ForegroundColor Yellow
        $Results.Skipped++
        $testResult.Status = "Skipped"
    } elseif ($testResult.BasicTest -and $testResult.PathTest) {
        Write-Host "PASSED" -ForegroundColor Green
        $Results.Passed++
        $testResult.Status = "Passed"
    } else {
        Write-Host "FAILED" -ForegroundColor Red
        if ($testResult.ErrorMessage) {
            Write-Host "  Error: $($testResult.ErrorMessage)" -ForegroundColor Red
        }
        $Results.Failed++
        $testResult.Status = "Failed"
    }

    $Results.Details += $testResult

    if ($Verbose) {
        Write-Host "  Version: $($testResult.Version)"
        Write-Host "  Help: $($testResult.Help)"
        Write-Host "  Basic Test: $($testResult.BasicTest)"
        Write-Host "  Path Test: $($testResult.PathTest)"
    }
}

# Performance tests for key utilities
Write-Host "`n=== Performance Validation ===" -ForegroundColor Cyan

$perfUtils = @("cat", "ls", "cp", "sort", "wc")
foreach ($util in $perfUtils) {
    $binaryPath = Join-Path $TargetDir "uu_$util.exe"
    if (Test-Path $binaryPath) {
        Write-Host "Benchmarking $util..." -ForegroundColor Yellow

        # Create test data
        $testFile = Join-Path $TempDir "perf-test.txt"
        1..1000 | ForEach-Object { "Line $_" } | Out-File $testFile -Encoding UTF8

        # Measure performance
        $time = Measure-Command {
            & $binaryPath $testFile 2>&1 | Out-Null
        }

        Write-Host "  Execution time: $($time.TotalMilliseconds)ms" -ForegroundColor Gray
        Remove-Item $testFile -Force -ErrorAction SilentlyContinue
    }
}

# Generate summary
Write-Host "`n=== Validation Summary ===" -ForegroundColor Cyan
Write-Host "Total: $($Results.Total)" -ForegroundColor White
Write-Host "Passed: $($Results.Passed)" -ForegroundColor Green
Write-Host "Failed: $($Results.Failed)" -ForegroundColor Red
Write-Host "Skipped: $($Results.Skipped)" -ForegroundColor Yellow
Write-Host "Success Rate: $([math]::Round(($Results.Passed / $Results.Total) * 100, 2))%" -ForegroundColor White

# Generate report if requested
if ($GenerateReport) {
    $Results | ConvertTo-Json -Depth 10 | Out-File $ReportFile
    Write-Host "`nReport saved to: $ReportFile" -ForegroundColor Green
}

# Cleanup
Remove-Item $TempDir -Recurse -Force -ErrorAction SilentlyContinue

# Exit with appropriate code
if ($Results.Failed -gt 0) {
    exit 1
} else {
    exit 0
}
