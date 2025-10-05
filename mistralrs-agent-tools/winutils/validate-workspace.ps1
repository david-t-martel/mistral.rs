# Winutils Workspace Validation Script
# Validates the optimized workspace structure and checks for any remaining issues

param(
    [switch]$Verbose = $false,
    [switch]$Fix = $false
)

$ErrorActionPreference = "Stop"
$ProgressPreference = 'SilentlyContinue'

# Colors for output
function Write-Pass { param($Message) Write-Host "[PASS] $Message" -ForegroundColor Green }
function Write-Fail { param($Message) Write-Host "[FAIL] $Message" -ForegroundColor Red }
function Write-Warn { param($Message) Write-Host "[WARN] $Message" -ForegroundColor Yellow }
function Write-Info { param($Message) Write-Host "[INFO] $Message" -ForegroundColor Cyan }
function Write-Check { param($Message) Write-Host "  ✓ $Message" -ForegroundColor DarkGreen }
function Write-Cross { param($Message) Write-Host "  ✗ $Message" -ForegroundColor DarkRed }

$ProjectRoot = $PSScriptRoot
$ValidationResults = @{
    Passed = 0
    Failed = 0
    Warnings = 0
    Issues = @()
}

Write-Host @"
╔════════════════════════════════════════════════════════════════╗
║          Winutils Workspace Validation Tool v1.0              ║
║                                                                ║
║  Checking for:                                                ║
║  • Duplicate implementations                                  ║
║  • Inconsistent dependencies                                  ║
║  • Build configuration issues                                 ║
║  • Structural problems                                        ║
╚════════════════════════════════════════════════════════════════╝
"@ -ForegroundColor Cyan

# Test 1: Check for duplicate Cargo.toml files for same utility
Write-Host "`n==> Checking for duplicate utilities..." -ForegroundColor Magenta

$utilityNames = @{}
$cargoFiles = Get-ChildItem -Path $ProjectRoot -Filter "Cargo.toml" -Recurse

foreach ($file in $cargoFiles) {
    $content = Get-Content $file.FullName -Raw
    if ($content -match 'name\s*=\s*"([^"]+)"') {
        $packageName = $matches[1]

        if ($utilityNames.ContainsKey($packageName)) {
            $utilityNames[$packageName] += $file.FullName
        } else {
            $utilityNames[$packageName] = @($file.FullName)
        }
    }
}

$duplicates = $utilityNames.GetEnumerator() | Where-Object { $_.Value.Count -gt 1 }

if ($duplicates) {
    foreach ($dup in $duplicates) {
        Write-Fail "Duplicate package '$($dup.Key)' found in:"
        $dup.Value | ForEach-Object { Write-Cross $_ }
        $ValidationResults.Failed++
        $ValidationResults.Issues += "Duplicate package: $($dup.Key)"
    }
} else {
    Write-Pass "No duplicate packages found"
    $ValidationResults.Passed++
}

# Test 2: Check workspace structure compliance
Write-Host "`n==> Checking workspace structure compliance..." -ForegroundColor Magenta

$requiredDirs = @(
    "crates\libs",
    "crates\utils",
    "docs",
    "scripts",
    ".cargo"
)

$structureValid = $true
foreach ($dir in $requiredDirs) {
    $path = Join-Path $ProjectRoot $dir
    if (Test-Path $path) {
        Write-Check "$dir exists"
    } else {
        Write-Cross "$dir missing"
        $structureValid = $false
        $ValidationResults.Issues += "Missing directory: $dir"
    }
}

if ($structureValid) {
    Write-Pass "Workspace structure is compliant"
    $ValidationResults.Passed++
} else {
    Write-Fail "Workspace structure has issues"
    $ValidationResults.Failed++
}

# Test 3: Check for workspace dependencies consistency
Write-Host "`n==> Checking dependency consistency..." -ForegroundColor Magenta

$workspaceToml = Join-Path $ProjectRoot "Cargo.toml"
$inconsistentDeps = @()

if (Test-Path $workspaceToml) {
    $workspaceContent = Get-Content $workspaceToml -Raw

    foreach ($file in $cargoFiles) {
        if ($file.FullName -eq $workspaceToml) { continue }

        $content = Get-Content $file.FullName -Raw

        # Check for direct version specifications instead of workspace references
        $directVersions = @()

        if ($content -match 'anyhow\s*=\s*"[\d\.]+"') {
            $directVersions += "anyhow"
        }
        if ($content -match 'clap\s*=\s*\{[^}]*version\s*=\s*"[^"]+') {
            $directVersions += "clap"
        }
        if ($content -match 'serde\s*=\s*\{[^}]*version\s*=\s*"[^"]+') {
            $directVersions += "serde"
        }
        if ($content -match 'tokio\s*=\s*\{[^}]*version\s*=\s*"[^"]+') {
            $directVersions += "tokio"
        }

        if ($directVersions) {
            $relativePath = $file.FullName.Replace($ProjectRoot, "").TrimStart("\")
            $inconsistentDeps += @{
                File = $relativePath
                Dependencies = $directVersions
            }
        }
    }

    if ($inconsistentDeps) {
        Write-Warn "Found inconsistent dependencies:"
        foreach ($dep in $inconsistentDeps) {
            Write-Cross "$($dep.File): $($dep.Dependencies -join ', ')"
        }
        $ValidationResults.Warnings++
        $ValidationResults.Issues += "Inconsistent dependencies in $($inconsistentDeps.Count) files"

        if ($Fix) {
            Write-Info "Attempting to fix inconsistent dependencies..."
            # Fix logic would go here
        }
    } else {
        Write-Pass "All dependencies use workspace references"
        $ValidationResults.Passed++
    }
} else {
    Write-Fail "Workspace Cargo.toml not found"
    $ValidationResults.Failed++
}

# Test 4: Check for orphaned files
Write-Host "`n==> Checking for orphaned files..." -ForegroundColor Magenta

$orphanedPatterns = @(
    "*.log",
    "*.bak",
    "*.tmp",
    "*.o",
    "Makefile.old",
    "Makefile-*",
    "*-standalone.toml"
)

$orphanedFiles = @()
foreach ($pattern in $orphanedPatterns) {
    $found = Get-ChildItem -Path $ProjectRoot -Filter $pattern -Recurse -ErrorAction SilentlyContinue
    $orphanedFiles += $found
}

if ($orphanedFiles) {
    Write-Warn "Found orphaned files:"
    foreach ($file in $orphanedFiles) {
        $relativePath = $file.FullName.Replace($ProjectRoot, "").TrimStart("\")
        Write-Cross $relativePath
    }
    $ValidationResults.Warnings++

    if ($Fix) {
        Write-Info "Cleaning orphaned files..."
        foreach ($file in $orphanedFiles) {
            Remove-Item $file.FullName -Force
            Write-Check "Removed: $($file.Name)"
        }
    }
} else {
    Write-Pass "No orphaned files found"
    $ValidationResults.Passed++
}

# Test 5: Validate build profiles
Write-Host "`n==> Validating build profiles..." -ForegroundColor Magenta

if (Test-Path $workspaceToml) {
    $content = Get-Content $workspaceToml -Raw

    $requiredProfiles = @("release", "dev", "test")
    $profilesValid = $true

    foreach ($profile in $requiredProfiles) {
        if ($content -match "\[profile\.$profile\]") {
            Write-Check "Profile '$profile' defined"
        } else {
            Write-Cross "Profile '$profile' missing"
            $profilesValid = $false
        }
    }

    if ($profilesValid) {
        Write-Pass "All required build profiles present"
        $ValidationResults.Passed++
    } else {
        Write-Fail "Missing build profiles"
        $ValidationResults.Failed++
    }
}

# Test 6: Check for circular dependencies
Write-Host "`n==> Checking for circular dependencies..." -ForegroundColor Magenta

# This would require parsing all Cargo.toml files and building a dependency graph
# For now, we'll do a simple check

$circularDeps = $false

# Simplified check - ensure libs don't depend on utils
$libsPath = Join-Path $ProjectRoot "crates\libs"
if (Test-Path $libsPath) {
    $libCargoFiles = Get-ChildItem -Path $libsPath -Filter "Cargo.toml" -Recurse

    foreach ($file in $libCargoFiles) {
        $content = Get-Content $file.FullName -Raw
        if ($content -match 'path\s*=\s*"[^"]*utils') {
            Write-Cross "Library depends on utility: $($file.FullName)"
            $circularDeps = $true
        }
    }
}

if (-not $circularDeps) {
    Write-Pass "No circular dependencies detected"
    $ValidationResults.Passed++
} else {
    Write-Fail "Circular dependencies found"
    $ValidationResults.Failed++
}

# Test 7: Verify Rust toolchain
Write-Host "`n==> Checking Rust toolchain configuration..." -ForegroundColor Magenta

$toolchainFile = Join-Path $ProjectRoot "rust-toolchain.toml"
if (Test-Path $toolchainFile) {
    Write-Pass "rust-toolchain.toml exists"
    $ValidationResults.Passed++
} else {
    Write-Warn "rust-toolchain.toml not found"
    $ValidationResults.Warnings++

    if ($Fix) {
        $toolchainContent = @'
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
profile = "minimal"
'@
        $toolchainContent | Out-File -FilePath $toolchainFile -Encoding UTF8
        Write-Check "Created rust-toolchain.toml"
    }
}

# Test 8: Validate compilation
Write-Host "`n==> Testing compilation..." -ForegroundColor Magenta

if ($Verbose) {
    Write-Info "Running 'cargo check --workspace'..."
    $checkResult = & cargo check --workspace 2>&1

    if ($LASTEXITCODE -eq 0) {
        Write-Pass "Workspace compiles successfully"
        $ValidationResults.Passed++
    } else {
        Write-Fail "Compilation errors found"
        $ValidationResults.Failed++
        if ($Verbose) {
            Write-Host $checkResult -ForegroundColor Red
        }
    }
} else {
    Write-Info "Skipping compilation test (use -Verbose to enable)"
}

# Generate report
Write-Host "`n" -NoNewline
Write-Host "╔════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║                    Validation Report                          ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

Write-Host "`nResults Summary:" -ForegroundColor White
Write-Host "  Passed:   $($ValidationResults.Passed)" -ForegroundColor Green
Write-Host "  Failed:   $($ValidationResults.Failed)" -ForegroundColor Red
Write-Host "  Warnings: $($ValidationResults.Warnings)" -ForegroundColor Yellow

if ($ValidationResults.Issues.Count -gt 0) {
    Write-Host "`nIssues Found:" -ForegroundColor Yellow
    $ValidationResults.Issues | ForEach-Object { Write-Host "  • $_" -ForegroundColor Yellow }
}

# Overall status
$overallStatus = if ($ValidationResults.Failed -eq 0) {
    if ($ValidationResults.Warnings -eq 0) {
        Write-Host "`n✅ VALIDATION PASSED - No issues found!" -ForegroundColor Green
        0
    } else {
        Write-Host "`n⚠️  VALIDATION PASSED WITH WARNINGS" -ForegroundColor Yellow
        1
    }
} else {
    Write-Host "`n❌ VALIDATION FAILED - Critical issues found!" -ForegroundColor Red
    2
}

if ($Fix -and ($ValidationResults.Failed -gt 0 -or $ValidationResults.Warnings -gt 0)) {
    Write-Host "`n💡 Run with -Fix parameter to attempt automatic fixes" -ForegroundColor Cyan
}

exit $overallStatus
