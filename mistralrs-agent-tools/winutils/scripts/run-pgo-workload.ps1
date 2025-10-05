# Profile-Guided Optimization Workload Script
# This script runs a representative workload for PGO training

param(
    [string]$BinDir = ".\target\release",
    [string]$DataDir = "pgo-workload-data",
    [int]$Iterations = 5
)

$ErrorActionPreference = "Stop"

Write-Host "Running PGO training workload..." -ForegroundColor Cyan

# Ensure data directory exists
New-Item -ItemType Directory -Force -Path $DataDir | Out-Null

# Generate representative test data
Write-Host "Generating test data..." -ForegroundColor Yellow

# Text file for text processing utilities
$textFile = Join-Path $DataDir "text.txt"
$textContent = @()
for ($i = 0; $i -lt 10000; $i++) {
    $textContent += "Line $i: The quick brown fox jumps over the lazy dog. Random number: $([System.Random]::new().Next())."
}
$textContent | Out-File $textFile -Encoding UTF8

# CSV file for column utilities
$csvFile = Join-Path $DataDir "data.csv"
$csvContent = "Name,Age,City,Salary`n"
for ($i = 0; $i -lt 5000; $i++) {
    $csvContent += "Person$i,$([System.Random]::new().Next(18, 80)),City$([System.Random]::new().Next(1, 100)),$([System.Random]::new().Next(30000, 200000))`n"
}
$csvContent | Out-File $csvFile -Encoding UTF8

# Directory structure for file utilities
$testDir = Join-Path $DataDir "files"
New-Item -ItemType Directory -Force -Path $testDir | Out-Null
for ($i = 0; $i -lt 100; $i++) {
    $subDir = Join-Path $testDir "dir$i"
    New-Item -ItemType Directory -Force -Path $subDir | Out-Null
    for ($j = 0; $j -lt 10; $j++) {
        $file = Join-Path $subDir "file$j.txt"
        "Content of file $j in directory $i" | Out-File $file
    }
}

# Binary file for encoding utilities
$binaryFile = Join-Path $DataDir "binary.bin"
$bytes = [byte[]]::new(1024 * 1024)  # 1MB
[System.Random]::new().NextBytes($bytes)
[System.IO.File]::WriteAllBytes($binaryFile, $bytes)

Write-Host "Running workload iterations..." -ForegroundColor Yellow

# Define representative workload
$workload = @(
    # Text processing
    @{cmd = "cat"; args = $textFile},
    @{cmd = "wc"; args = "-lwc $textFile"},
    @{cmd = "head"; args = "-n 100 $textFile"},
    @{cmd = "tail"; args = "-n 100 $textFile"},
    @{cmd = "sort"; args = $textFile},
    @{cmd = "uniq"; args = $textFile},

    # Pattern matching
    @{cmd = "grep-wrapper"; args = "'fox' $textFile"},
    @{cmd = "sed-wrapper"; args = "'s/fox/cat/g' $textFile"},

    # File operations
    @{cmd = "ls"; args = "-laR $testDir"},
    @{cmd = "find-wrapper"; args = "$testDir -name '*.txt'"},
    @{cmd = "du"; args = "-sh $testDir"},
    @{cmd = "tree"; args = $testDir},

    # Column processing
    @{cmd = "cut"; args = "-d',' -f2,4 $csvFile"},
    @{cmd = "awk-wrapper"; args = "'{print `$2}' $csvFile"},
    @{cmd = "paste"; args = "$textFile $csvFile"},

    # Encoding
    @{cmd = "base64"; args = $binaryFile},
    @{cmd = "base32"; args = $binaryFile},

    # Checksums
    @{cmd = "hashsum"; args = "--sha256 $binaryFile"},
    @{cmd = "hashsum"; args = "--blake3 $binaryFile"},
    @{cmd = "cksum"; args = $binaryFile},

    # Path operations
    @{cmd = "basename"; args = $textFile},
    @{cmd = "dirname"; args = $textFile},
    @{cmd = "realpath"; args = $textFile},

    # File manipulation
    @{cmd = "cp"; args = "$textFile $textFile.copy"},
    @{cmd = "mv"; args = "$textFile.copy $textFile.moved"},
    @{cmd = "rm"; args = "$textFile.moved"},
    @{cmd = "touch"; args = "$DataDir/newfile.txt"},

    # Compression (if available)
    @{cmd = "gzip-wrapper"; args = "-c $textFile"},
    @{cmd = "zcat-wrapper"; args = "$textFile.gz"},

    # System info
    @{cmd = "whoami"; args = ""},
    @{cmd = "hostname"; args = ""},
    @{cmd = "pwd"; args = ""},
    @{cmd = "env"; args = ""},

    # Date/time
    @{cmd = "date"; args = ""},
    @{cmd = "sleep"; args = "0.1"},

    # Math operations
    @{cmd = "factor"; args = "12345678"},
    @{cmd = "seq"; args = "1 100"},
    @{cmd = "expr"; args = "10 + 20 * 3"},

    # Text formatting
    @{cmd = "fmt"; args = $textFile},
    @{cmd = "fold"; args = "-w 80 $textFile"},
    @{cmd = "expand"; args = $textFile},
    @{cmd = "unexpand"; args = $textFile},

    # Advanced utilities
    @{cmd = "comm"; args = "$textFile $textFile"},
    @{cmd = "join"; args = "$csvFile $csvFile"},
    @{cmd = "tsort"; args = $textFile},
    @{cmd = "shuf"; args = "-n 10 $textFile"}
)

$totalCommands = $workload.Count * $Iterations
$completed = 0

for ($iter = 1; $iter -le $Iterations; $iter++) {
    Write-Host "  Iteration $iter of $Iterations" -ForegroundColor Cyan

    foreach ($work in $workload) {
        $exe = Join-Path $BinDir "$($work.cmd).exe"

        if (Test-Path $exe) {
            try {
                # Execute command (output redirected to null for performance)
                if ($work.args) {
                    $result = & $exe $work.args.Split() 2>&1 | Out-Null
                } else {
                    $result = & $exe 2>&1 | Out-Null
                }
            } catch {
                # Ignore errors during training
            }
        }

        $completed++
        $progress = [math]::Round(($completed / $totalCommands) * 100, 1)
        Write-Progress -Activity "PGO Training" -Status "$progress% Complete" -PercentComplete $progress
    }
}

Write-Progress -Activity "PGO Training" -Completed

Write-Host "PGO workload completed successfully!" -ForegroundColor Green
Write-Host "Profile data has been collected for optimization." -ForegroundColor Green

# Cleanup temporary files
Write-Host "Cleaning up temporary files..." -ForegroundColor Yellow
Remove-Item -Path $DataDir -Recurse -Force

Write-Host "Done!" -ForegroundColor Green
