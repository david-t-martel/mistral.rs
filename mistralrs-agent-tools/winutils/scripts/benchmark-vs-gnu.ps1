# WinUtils vs GNU Coreutils Performance Benchmark Script
# Requires: hyperfine (cargo install hyperfine or scoop install hyperfine)

param(
    [string]$Utility = "all",
    [string]$OutputDir = "benchmark-results",
    [int]$Runs = 10,
    [int]$Warmup = 3,
    [switch]$CreateTestData,
    [switch]$SkipGNU
)

# Configuration
$ErrorActionPreference = "Stop"
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$outputPath = Join-Path $OutputDir $timestamp

# Ensure output directory exists
New-Item -ItemType Directory -Force -Path $outputPath | Out-Null

# Test data configuration
$testDataDir = "benchmark-test-data"
$largeFile = Join-Path $testDataDir "large-file.txt"
$randomLines = Join-Path $testDataDir "random-lines.txt"
$manyFiles = Join-Path $testDataDir "many-files"

# Create test data if requested or doesn't exist
if ($CreateTestData -or -not (Test-Path $testDataDir)) {
    Write-Host "Creating test data..." -ForegroundColor Yellow

    New-Item -ItemType Directory -Force -Path $testDataDir | Out-Null
    New-Item -ItemType Directory -Force -Path $manyFiles | Out-Null

    # Create large file (100MB)
    $content = "The quick brown fox jumps over the lazy dog`n" * 100
    $sb = [System.Text.StringBuilder]::new()
    for ($i = 0; $i -lt 2000; $i++) {
        [void]$sb.AppendLine($content)
    }
    Set-Content -Path $largeFile -Value $sb.ToString() -NoNewline

    # Create random lines file for sorting (10MB)
    $random = [System.Random]::new()
    $lines = @()
    for ($i = 0; $i -lt 100000; $i++) {
        $lines += "line_{0:D8}_{1}" -f $random.Next(0, 1000000), ("x" * $random.Next(10, 100))
    }
    Set-Content -Path $randomLines -Value ($lines -join "`n")

    # Create many small files
    for ($i = 0; $i -lt 1000; $i++) {
        $filePath = Join-Path $manyFiles "file_$i.txt"
        Set-Content -Path $filePath -Value "Content of file $i"
    }

    Write-Host "Test data created successfully" -ForegroundColor Green
}

# Utility configurations
$utilities = @(
    @{
        name = "cat"
        winutils = ".\target\release\cat.exe"
        gnu = "C:\Program Files\Git\usr\bin\cat.exe"
        args = $largeFile
        description = "Reading 100MB file"
    },
    @{
        name = "wc"
        winutils = ".\target\release\wc.exe"
        gnu = "C:\Program Files\Git\usr\bin\wc.exe"
        args = "-lwc $largeFile"
        description = "Counting lines, words, bytes in 100MB file"
    },
    @{
        name = "sort"
        winutils = ".\target\release\sort.exe"
        gnu = "C:\Program Files\Git\usr\bin\sort.exe"
        args = $randomLines
        description = "Sorting 100,000 random lines"
    },
    @{
        name = "ls"
        winutils = ".\target\release\ls.exe"
        gnu = "C:\Program Files\Git\usr\bin\ls.exe"
        args = "-la C:\Windows\System32"
        description = "Listing Windows System32 directory"
    },
    @{
        name = "hashsum"
        winutils = ".\target\release\hashsum.exe"
        gnu = "C:\Program Files\Git\usr\bin\sha256sum.exe"
        args = $largeFile
        gnuArgs = $largeFile  # Different args for GNU version
        description = "Computing SHA256 of 100MB file"
    },
    @{
        name = "find"
        winutils = ".\target\release\find-wrapper.exe"
        gnu = "C:\Program Files\Git\usr\bin\find.exe"
        args = "$manyFiles -name '*.txt'"
        description = "Finding files in directory with 1000 files"
    },
    @{
        name = "grep"
        winutils = ".\target\release\grep-wrapper.exe"
        gnu = "C:\Program Files\Git\usr\bin\grep.exe"
        args = "'fox' $largeFile"
        description = "Searching pattern in 100MB file"
    },
    @{
        name = "cp"
        winutils = ".\target\release\cp.exe"
        gnu = "C:\Program Files\Git\usr\bin\cp.exe"
        args = "$largeFile $largeFile.copy"
        cleanup = { Remove-Item -Path "$largeFile.copy" -ErrorAction SilentlyContinue }
        description = "Copying 100MB file"
    },
    @{
        name = "base64"
        winutils = ".\target\release\base64.exe"
        gnu = "C:\Program Files\Git\usr\bin\base64.exe"
        args = $largeFile
        description = "Base64 encoding 100MB file"
    }
)

# Results storage
$results = @{
    timestamp = $timestamp
    utilities = @()
    summary = @{}
}

# Benchmark each utility
foreach ($util in $utilities) {
    if ($Utility -ne "all" -and $Utility -ne $util.name) { continue }

    Write-Host "`nBenchmarking $($util.name): $($util.description)" -ForegroundColor Cyan

    # Check if executables exist
    if (-not (Test-Path $util.winutils)) {
        Write-Host "  WinUtils binary not found: $($util.winutils)" -ForegroundColor Red
        continue
    }

    $commands = @("'$($util.winutils) $($util.args)'")
    $commandNames = @("winutils")

    if (-not $SkipGNU) {
        if (Test-Path $util.gnu) {
            $gnuArgs = if ($util.gnuArgs) { $util.gnuArgs } else { $util.args }
            $commands += "'$($util.gnu) $gnuArgs'"
            $commandNames += "gnu"
        } else {
            Write-Host "  GNU binary not found: $($util.gnu)" -ForegroundColor Yellow
        }
    }

    # Run hyperfine benchmark
    $jsonOutput = Join-Path $outputPath "$($util.name).json"
    $markdownOutput = Join-Path $outputPath "$($util.name).md"

    $hyperfineCmd = "hyperfine --warmup $Warmup --runs $Runs --export-json '$jsonOutput' --export-markdown '$markdownOutput'"

    foreach ($cmd in $commands) {
        $hyperfineCmd += " $cmd"
    }

    try {
        Invoke-Expression $hyperfineCmd

        # Parse results
        $jsonResults = Get-Content $jsonOutput | ConvertFrom-Json

        $utilResult = @{
            name = $util.name
            description = $util.description
            results = @()
        }

        for ($i = 0; $i -lt $jsonResults.results.Count; $i++) {
            $result = $jsonResults.results[$i]
            $utilResult.results += @{
                command = $commandNames[$i]
                mean = $result.mean
                stddev = $result.stddev
                median = $result.median
                min = $result.min
                max = $result.max
            }
        }

        # Calculate speedup if both versions were tested
        if ($utilResult.results.Count -eq 2) {
            $winMean = $utilResult.results | Where-Object { $_.command -eq "winutils" } | Select-Object -ExpandProperty mean
            $gnuMean = $utilResult.results | Where-Object { $_.command -eq "gnu" } | Select-Object -ExpandProperty mean
            $speedup = [math]::Round($gnuMean / $winMean, 2)
            $utilResult.speedup = $speedup

            if ($speedup -gt 1) {
                Write-Host "  WinUtils is ${speedup}x faster!" -ForegroundColor Green
            } elseif ($speedup -eq 1) {
                Write-Host "  Performance is comparable" -ForegroundColor Yellow
            } else {
                Write-Host "  GNU is $([math]::Round(1/$speedup, 2))x faster" -ForegroundColor Red
            }
        }

        $results.utilities += $utilResult

    } catch {
        Write-Host "  Benchmark failed: $_" -ForegroundColor Red
    }

    # Run cleanup if specified
    if ($util.cleanup) {
        & $util.cleanup
    }
}

# Generate summary
Write-Host "`n" + ("=" * 60) -ForegroundColor Cyan
Write-Host "BENCHMARK SUMMARY" -ForegroundColor Cyan
Write-Host ("=" * 60) -ForegroundColor Cyan

$totalSpeedup = 0
$utilityCount = 0

foreach ($util in $results.utilities) {
    if ($util.speedup) {
        $totalSpeedup += $util.speedup
        $utilityCount++

        $color = if ($util.speedup -gt 1) { "Green" } elseif ($util.speedup -eq 1) { "Yellow" } else { "Red" }
        Write-Host ("{0,-15} {1,6}x" -f $util.name, $util.speedup) -ForegroundColor $color
    }
}

if ($utilityCount -gt 0) {
    $averageSpeedup = [math]::Round($totalSpeedup / $utilityCount, 2)
    Write-Host ("-" * 22) -ForegroundColor Cyan
    Write-Host ("Average:        {0,6}x" -f $averageSpeedup) -ForegroundColor Cyan

    $results.summary = @{
        averageSpeedup = $averageSpeedup
        utilitiesTested = $utilityCount
    }
}

# Save results summary
$summaryPath = Join-Path $outputPath "summary.json"
$results | ConvertTo-Json -Depth 10 | Set-Content $summaryPath

# Generate HTML report
$htmlReport = @"
<!DOCTYPE html>
<html>
<head>
    <title>WinUtils Performance Report - $timestamp</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        h1 { color: #333; }
        .summary { background: #f0f0f0; padding: 15px; border-radius: 5px; margin: 20px 0; }
        table { border-collapse: collapse; width: 100%; margin: 20px 0; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #4CAF50; color: white; }
        tr:nth-child(even) { background-color: #f2f2f2; }
        .faster { color: green; font-weight: bold; }
        .slower { color: red; }
        .neutral { color: orange; }
    </style>
</head>
<body>
    <h1>WinUtils Performance Benchmark Report</h1>
    <div class="summary">
        <h2>Summary</h2>
        <p>Date: $timestamp</p>
        <p>Utilities Tested: $utilityCount</p>
        <p>Average Speedup: <span class="faster">${averageSpeedup}x</span></p>
    </div>

    <h2>Detailed Results</h2>
    <table>
        <tr>
            <th>Utility</th>
            <th>Description</th>
            <th>WinUtils (ms)</th>
            <th>GNU (ms)</th>
            <th>Speedup</th>
        </tr>
"@

foreach ($util in $results.utilities) {
    $winResult = $util.results | Where-Object { $_.command -eq "winutils" }
    $gnuResult = $util.results | Where-Object { $_.command -eq "gnu" }

    if ($winResult -and $gnuResult) {
        $speedupClass = if ($util.speedup -gt 1) { "faster" } elseif ($util.speedup -eq 1) { "neutral" } else { "slower" }
        $winMean = [math]::Round($winResult.mean * 1000, 2)
        $gnuMean = [math]::Round($gnuResult.mean * 1000, 2)

        $htmlReport += @"
        <tr>
            <td>$($util.name)</td>
            <td>$($util.description)</td>
            <td>$winMean</td>
            <td>$gnuMean</td>
            <td class="$speedupClass">$($util.speedup)x</td>
        </tr>
"@
    }
}

$htmlReport += @"
    </table>
</body>
</html>
"@

$htmlPath = Join-Path $outputPath "report.html"
$htmlReport | Set-Content $htmlPath

Write-Host "`nResults saved to: $outputPath" -ForegroundColor Green
Write-Host "  - JSON results: Individual .json files"
Write-Host "  - Markdown results: Individual .md files"
Write-Host "  - Summary: summary.json"
Write-Host "  - HTML Report: report.html"

# Open HTML report in default browser
Start-Process $htmlPath
