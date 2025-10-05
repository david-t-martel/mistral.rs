# GNU Compatibility Test Script
# Tests winutils against GNU coreutils for functional parity

param(
    [string]$GnuPath = "C:\Program Files\Git\usr\bin",
    [string]$WinUtilsPath = ".\target\release",
    [switch]$Verbose = $false,
    [switch]$DetailedReport = $false,
    [string]$OutputFile = "gnu-compat-report.json"
)

$ErrorActionPreference = "Continue"

# Test configuration
$TempDir = Join-Path $env:TEMP "gnu-compat-test"
$TestFile = Join-Path $TempDir "test.txt"
$TestDir = Join-Path $TempDir "testdir"

# Create test environment
if (Test-Path $TempDir) {
    Remove-Item $TempDir -Recurse -Force
}
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
New-Item -ItemType Directory -Path $TestDir -Force | Out-Null

# Create test files
@"
Line 1
Line 2
Line 3
Line 4
Line 5
"@ | Out-File $TestFile -Encoding UTF8

1..10 | ForEach-Object {
    "Test content $_" | Out-File (Join-Path $TestDir "file$_.txt") -Encoding UTF8
}

# Test results
$Results = @{
    TotalTests = 0
    Passed = 0
    Failed = 0
    Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    Details = @()
}

# Helper function to compare outputs
function Compare-Outputs {
    param(
        [string]$Util,
        [string]$Args,
        [string]$GnuExe,
        [string]$WinExe
    )

    $test = @{
        Utility = $Util
        Arguments = $Args
        GnuOutput = $null
        WinOutput = $null
        GnuExitCode = $null
        WinExitCode = $null
        Match = $false
        Error = $null
    }

    try {
        # Run GNU version
        $gnuResult = & $GnuExe $Args.Split() 2>&1
        $test.GnuExitCode = $LASTEXITCODE
        $test.GnuOutput = $gnuResult -join "`n"

        # Run WinUtils version
        $winResult = & $WinExe $Args.Split() 2>&1
        $test.WinExitCode = $LASTEXITCODE
        $test.WinOutput = $winResult -join "`n"

        # Compare outputs (normalize line endings)
        $gnuNorm = $test.GnuOutput -replace "`r`n", "`n"
        $winNorm = $test.WinOutput -replace "`r`n", "`n"

        $test.Match = ($gnuNorm -eq $winNorm) -and ($test.GnuExitCode -eq $test.WinExitCode)

        if ($test.Match) {
            $Results.Passed++
            Write-Host "✓" -ForegroundColor Green -NoNewline
        } else {
            $Results.Failed++
            Write-Host "✗" -ForegroundColor Red -NoNewline
            if ($Verbose) {
                Write-Host ""
                Write-Host "  GNU Output: $($test.GnuOutput.Substring(0, [Math]::Min(50, $test.GnuOutput.Length)))"
                Write-Host "  Win Output: $($test.WinOutput.Substring(0, [Math]::Min(50, $test.WinOutput.Length)))"
                Write-Host "  GNU Exit: $($test.GnuExitCode), Win Exit: $($test.WinExitCode)"
            }
        }
    } catch {
        $test.Error = $_.Exception.Message
        $Results.Failed++
        Write-Host "!" -ForegroundColor Yellow -NoNewline
    }

    $Results.TotalTests++
    $Results.Details += $test
    return $test
}

# Test suite definitions
$TestSuites = @{
    "Basic Text Processing" = @(
        @{ Util = "echo"; Tests = @(
            @{ Args = "hello world"; Desc = "Simple echo" },
            @{ Args = "-n test"; Desc = "No newline" },
            @{ Args = "-e \thello\n"; Desc = "Escape sequences" }
        )},
        @{ Util = "cat"; Tests = @(
            @{ Args = "$TestFile"; Desc = "Read file" },
            @{ Args = "-n $TestFile"; Desc = "Number lines" },
            @{ Args = "-b $TestFile"; Desc = "Number non-blank" }
        )},
        @{ Util = "head"; Tests = @(
            @{ Args = "$TestFile"; Desc = "Default head" },
            @{ Args = "-n 2 $TestFile"; Desc = "First 2 lines" },
            @{ Args = "-c 10 $TestFile"; Desc = "First 10 bytes" }
        )},
        @{ Util = "tail"; Tests = @(
            @{ Args = "$TestFile"; Desc = "Default tail" },
            @{ Args = "-n 2 $TestFile"; Desc = "Last 2 lines" },
            @{ Args = "-c 10 $TestFile"; Desc = "Last 10 bytes" }
        )}
    )

    "File Operations" = @(
        @{ Util = "ls"; Tests = @(
            @{ Args = "$TestDir"; Desc = "List directory" },
            @{ Args = "-l $TestDir"; Desc = "Long format" },
            @{ Args = "-a $TestDir"; Desc = "Show all" }
        )},
        @{ Util = "cp"; Tests = @(
            @{ Args = "$TestFile $TempDir\copy.txt"; Desc = "Copy file" },
            @{ Args = "-r $TestDir $TempDir\copydir"; Desc = "Copy directory" }
        )},
        @{ Util = "mv"; Tests = @(
            @{ Args = "$TempDir\copy.txt $TempDir\moved.txt"; Desc = "Move file" }
        )},
        @{ Util = "rm"; Tests = @(
            @{ Args = "$TempDir\moved.txt"; Desc = "Remove file" },
            @{ Args = "-r $TempDir\copydir"; Desc = "Remove directory" }
        )}
    )

    "Text Utilities" = @(
        @{ Util = "wc"; Tests = @(
            @{ Args = "$TestFile"; Desc = "Word count" },
            @{ Args = "-l $TestFile"; Desc = "Line count" },
            @{ Args = "-w $TestFile"; Desc = "Word count only" }
        )},
        @{ Util = "sort"; Tests = @(
            @{ Args = "$TestFile"; Desc = "Sort lines" },
            @{ Args = "-r $TestFile"; Desc = "Reverse sort" },
            @{ Args = "-n $TestFile"; Desc = "Numeric sort" }
        )},
        @{ Util = "uniq"; Tests = @(
            @{ Args = "$TestFile"; Desc = "Remove duplicates" },
            @{ Args = "-c $TestFile"; Desc = "Count duplicates" }
        )}
    )

    "System Information" = @(
        @{ Util = "whoami"; Tests = @(
            @{ Args = ""; Desc = "Current user" }
        )},
        @{ Util = "hostname"; Tests = @(
            @{ Args = ""; Desc = "System hostname" }
        )},
        @{ Util = "pwd"; Tests = @(
            @{ Args = ""; Desc = "Current directory" }
        )},
        @{ Util = "uname"; Tests = @(
            @{ Args = ""; Desc = "System info" },
            @{ Args = "-a"; Desc = "All system info" }
        )}
    )

    "Date and Time" = @(
        @{ Util = "date"; Tests = @(
            @{ Args = "+%Y-%m-%d"; Desc = "Format date" },
            @{ Args = "+%H:%M:%S"; Desc = "Format time" }
        )},
        @{ Util = "sleep"; Tests = @(
            @{ Args = "0.1"; Desc = "Sleep 100ms" }
        )}
    )

    "Checksums" = @(
        @{ Util = "md5sum"; Tests = @(
            @{ Args = "$TestFile"; Desc = "MD5 checksum" }
        )},
        @{ Util = "sha1sum"; Tests = @(
            @{ Args = "$TestFile"; Desc = "SHA1 checksum" }
        )},
        @{ Util = "sha256sum"; Tests = @(
            @{ Args = "$TestFile"; Desc = "SHA256 checksum" }
        )}
    )

    "Boolean Utilities" = @(
        @{ Util = "true"; Tests = @(
            @{ Args = ""; Desc = "True exit code" }
        )},
        @{ Util = "false"; Tests = @(
            @{ Args = ""; Desc = "False exit code" }
        )}
    )

    "Math Utilities" = @(
        @{ Util = "expr"; Tests = @(
            @{ Args = "2 + 2"; Desc = "Addition" },
            @{ Args = "10 - 5"; Desc = "Subtraction" },
            @{ Args = "3 \* 4"; Desc = "Multiplication" }
        )},
        @{ Util = "factor"; Tests = @(
            @{ Args = "12"; Desc = "Factor number" },
            @{ Args = "17"; Desc = "Prime factor" }
        )},
        @{ Util = "seq"; Tests = @(
            @{ Args = "1 5"; Desc = "Sequence 1 to 5" },
            @{ Args = "1 2 10"; Desc = "Sequence with step" }
        )}
    )
}

# Run compatibility tests
Write-Host "`n=== GNU Coreutils Compatibility Test ===" -ForegroundColor Cyan
Write-Host "GNU Path: $GnuPath" -ForegroundColor Yellow
Write-Host "WinUtils Path: $WinUtilsPath" -ForegroundColor Yellow
Write-Host ""

foreach ($suiteName in $TestSuites.Keys) {
    Write-Host "`n[$suiteName]" -ForegroundColor Cyan

    foreach ($utilTest in $TestSuites[$suiteName]) {
        $util = $utilTest.Util
        Write-Host "  $util " -NoNewline

        foreach ($test in $utilTest.Tests) {
            $gnuExe = Join-Path $GnuPath "$util.exe"
            $winExe = Join-Path $WinUtilsPath "uu_$util.exe"

            # Check if both executables exist
            if (-not (Test-Path $gnuExe)) {
                Write-Host "?" -ForegroundColor Gray -NoNewline
                continue
            }
            if (-not (Test-Path $winExe)) {
                Write-Host "!" -ForegroundColor Yellow -NoNewline
                continue
            }

            # Run comparison
            $result = Compare-Outputs -Util $util -Args $test.Args `
                                     -GnuExe $gnuExe -WinExe $winExe
        }
        Write-Host ""
    }
}

# Additional path format tests
Write-Host "`n[Windows Path Compatibility]" -ForegroundColor Cyan
$pathTests = @(
    @{ Path = "C:\Windows\System32"; Desc = "DOS path" },
    @{ Path = "C:/Windows/System32"; Desc = "DOS forward slash" },
    @{ Path = "/mnt/c/Windows/System32"; Desc = "WSL path" },
    @{ Path = "/cygdrive/c/Windows/System32"; Desc = "Cygwin path" }
)

foreach ($pathTest in $pathTests) {
    Write-Host "  Testing path format: $($pathTest.Desc)" -NoNewline

    $winLs = Join-Path $WinUtilsPath "uu_ls.exe"
    if (Test-Path $winLs) {
        try {
            & $winLs $pathTest.Path 2>&1 | Out-Null
            if ($LASTEXITCODE -eq 0) {
                Write-Host " ✓" -ForegroundColor Green
                $Results.Passed++
            } else {
                Write-Host " ✗" -ForegroundColor Red
                $Results.Failed++
            }
        } catch {
            Write-Host " !" -ForegroundColor Yellow
            $Results.Failed++
        }
        $Results.TotalTests++
    } else {
        Write-Host " SKIP" -ForegroundColor Gray
    }
}

# Performance comparison for key utilities
Write-Host "`n[Performance Comparison]" -ForegroundColor Cyan
$perfUtils = @("cat", "ls", "wc", "sort")

foreach ($util in $perfUtils) {
    $gnuExe = Join-Path $GnuPath "$util.exe"
    $winExe = Join-Path $WinUtilsPath "uu_$util.exe"

    if ((Test-Path $gnuExe) -and (Test-Path $winExe)) {
        Write-Host "  $util performance:" -NoNewline

        # Create larger test file for performance testing
        $perfFile = Join-Path $TempDir "perf-test.txt"
        1..10000 | ForEach-Object { "Line $_ with test content" } | Out-File $perfFile -Encoding UTF8

        # Measure GNU performance
        $gnuTime = Measure-Command {
            & $gnuExe $perfFile 2>&1 | Out-Null
        }

        # Measure WinUtils performance
        $winTime = Measure-Command {
            & $winExe $perfFile 2>&1 | Out-Null
        }

        $ratio = [Math]::Round($gnuTime.TotalMilliseconds / $winTime.TotalMilliseconds, 2)
        $faster = if ($ratio -gt 1) {
            Write-Host " WinUtils ${ratio}x faster" -ForegroundColor Green
        } elseif ($ratio -eq 1) {
            Write-Host " Equal performance" -ForegroundColor Yellow
        } else {
            $slower = [Math]::Round(1 / $ratio, 2)
            Write-Host " GNU ${slower}x faster" -ForegroundColor Red
        }

        Remove-Item $perfFile -Force
    }
}

# Generate summary
Write-Host "`n=== Compatibility Summary ===" -ForegroundColor Cyan
Write-Host "Total Tests: $($Results.TotalTests)" -ForegroundColor White
Write-Host "Passed: $($Results.Passed)" -ForegroundColor Green
Write-Host "Failed: $($Results.Failed)" -ForegroundColor Red
$compatRate = [Math]::Round(($Results.Passed / $Results.TotalTests) * 100, 2)
Write-Host "Compatibility Rate: $compatRate%" -ForegroundColor $(if ($compatRate -ge 80) { "Green" } elseif ($compatRate -ge 60) { "Yellow" } else { "Red" })

# Generate detailed report if requested
if ($DetailedReport) {
    $Results | ConvertTo-Json -Depth 10 | Out-File $OutputFile
    Write-Host "`nDetailed report saved to: $OutputFile" -ForegroundColor Green
}

# Cleanup
Remove-Item $TempDir -Recurse -Force -ErrorAction SilentlyContinue

# Exit code based on compatibility
if ($compatRate -ge 90) {
    exit 0
} elseif ($compatRate -ge 70) {
    exit 1
} else {
    exit 2
}
