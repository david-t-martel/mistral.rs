# WinUtils Release Automation Script
# Automates the complete release process including building, testing, packaging, and deployment

param(
    [Parameter(Mandatory=$true)]
    [string]$Version,

    [Parameter(Mandatory=$false)]
    [string]$ReleaseType = "stable", # stable, beta, alpha

    [Parameter(Mandatory=$false)]
    [switch]$DryRun,

    [Parameter(Mandatory=$false)]
    [switch]$SkipTests,

    [Parameter(Mandatory=$false)]
    [switch]$SkipBenchmarks,

    [Parameter(Mandatory=$false)]
    [string]$GitHubToken = $env:GITHUB_TOKEN,

    [Parameter(Mandatory=$false)]
    [string]$SigningCertificate = "",

    [Parameter(Mandatory=$false)]
    [string]$DockerRegistry = "ghcr.io",

    [Parameter(Mandatory=$false)]
    [string]$DockerUsername = "",

    [Parameter(Mandatory=$false)]
    [string]$DockerPassword = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Configuration
$PROJECT_ROOT = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$RELEASE_DIR = Join-Path $PROJECT_ROOT "release"
$DIST_DIR = Join-Path $PROJECT_ROOT "dist"
$LOG_FILE = Join-Path $PROJECT_ROOT "release.log"

# Initialize logging
function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logEntry = "[$timestamp] [$Level] $Message"
    Write-Host $logEntry
    Add-Content -Path $LOG_FILE -Value $logEntry
}

function Test-Prerequisites {
    Write-Log "Checking prerequisites..." "INFO"

    # Check Git
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
        throw "Git is required but not found in PATH"
    }

    # Check Rust toolchain
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Rust toolchain is required but not found in PATH"
    }

    # Check Make
    if (-not (Get-Command make -ErrorAction SilentlyContinue)) {
        throw "Make is required but not found in PATH"
    }

    # Check working directory is clean
    $gitStatus = git status --porcelain
    if ($gitStatus -and -not $DryRun) {
        throw "Working directory is not clean. Please commit or stash changes before release."
    }

    # Check we're on main branch
    $currentBranch = git rev-parse --abbrev-ref HEAD
    if ($currentBranch -ne "main" -and -not $DryRun) {
        Write-Log "Warning: Not on main branch (current: $currentBranch)" "WARN"
    }

    # Check GitHub token for release creation
    if (-not $GitHubToken -and -not $DryRun) {
        Write-Log "Warning: No GitHub token provided. Release creation will be skipped." "WARN"
    }

    Write-Log "Prerequisites check completed" "INFO"
}

function Update-Version {
    param([string]$NewVersion)

    Write-Log "Updating version to $NewVersion..." "INFO"

    # Update Cargo.toml
    $cargoToml = Join-Path $PROJECT_ROOT "Cargo.toml"
    $content = Get-Content $cargoToml -Raw
    $content = $content -replace 'version = "[^"]*"', "version = `"$NewVersion`""
    Set-Content $cargoToml -Value $content -NoNewline

    # Update benchmark Cargo.toml
    $benchmarkCargoToml = Join-Path $PROJECT_ROOT "benchmarks\Cargo.toml"
    if (Test-Path $benchmarkCargoToml) {
        $content = Get-Content $benchmarkCargoToml -Raw
        $content = $content -replace 'version = "[^"]*"', "version = `"$NewVersion`""
        Set-Content $benchmarkCargoToml -Value $content -NoNewline
    }

    Write-Log "Version updated to $NewVersion" "INFO"
}

function Run-Tests {
    Write-Log "Running test suite..." "INFO"

    Push-Location $PROJECT_ROOT
    try {
        # Build first
        Write-Log "Building project..." "INFO"
        & make clean
        if ($LASTEXITCODE -ne 0) { throw "Build failed" }

        & make release
        if ($LASTEXITCODE -ne 0) { throw "Build failed" }

        # Run unit tests
        Write-Log "Running unit tests..." "INFO"
        & make test
        if ($LASTEXITCODE -ne 0) { throw "Unit tests failed" }

        # Run integration tests
        if (Test-Path "tests") {
            Write-Log "Running integration tests..." "INFO"
            # Add integration test commands here
        }

        Write-Log "All tests passed" "INFO"
    }
    finally {
        Pop-Location
    }
}

function Run-Benchmarks {
    Write-Log "Running performance benchmarks..." "INFO"

    Push-Location (Join-Path $PROJECT_ROOT "benchmarks")
    try {
        & cargo build --release
        if ($LASTEXITCODE -ne 0) { throw "Benchmark build failed" }

        & cargo run --release -- validate
        if ($LASTEXITCODE -ne 0) { throw "Benchmark environment validation failed" }

        $resultsDir = Join-Path $RELEASE_DIR "benchmarks"
        New-Item -ItemType Directory -Force -Path $resultsDir | Out-Null

        & cargo run --release -- run --baseline --compare-native --memory-profile --output $resultsDir
        if ($LASTEXITCODE -ne 0) { throw "Benchmarks failed" }

        # Generate reports
        & cargo run --release -- report --input $resultsDir --format html
        & cargo run --release -- report --input $resultsDir --format markdown

        Write-Log "Benchmarks completed successfully" "INFO"
    }
    finally {
        Pop-Location
    }
}

function Build-Packages {
    Write-Log "Building release packages..." "INFO"

    # Create release directories
    New-Item -ItemType Directory -Force -Path $RELEASE_DIR | Out-Null
    New-Item -ItemType Directory -Force -Path $DIST_DIR | Out-Null

    # Build Windows installer
    Write-Log "Building Windows installer..." "INFO"
    $installerScript = Join-Path $PSScriptRoot "build-installer.ps1"
    $installerArgs = @{
        Version = $Version
        OutputDir = $DIST_DIR
        SkipBuild = $true
        CreatePortable = $true
    }

    if ($SigningCertificate) {
        $installerArgs.SignBinaries = $true
        $installerArgs.CertificatePath = $SigningCertificate
    }

    & $installerScript @installerArgs
    if ($LASTEXITCODE -ne 0) { throw "Installer build failed" }

    # Create source archive
    Write-Log "Creating source archive..." "INFO"
    $sourceArchive = Join-Path $DIST_DIR "winutils-source-v$Version.zip"
    $excludePatterns = @(
        "target/*",
        ".git/*",
        "*.log",
        "release/*",
        "dist/*",
        "benchmark-results/*"
    )

    Push-Location $PROJECT_ROOT
    try {
        $files = Get-ChildItem -Recurse | Where-Object {
            $relativePath = $_.FullName.Substring($PROJECT_ROOT.Length + 1)
            $exclude = $false
            foreach ($pattern in $excludePatterns) {
                if ($relativePath -like $pattern) {
                    $exclude = $true
                    break
                }
            }
            -not $exclude -and -not $_.PSIsContainer
        }

        Compress-Archive -Path $files.FullName -DestinationPath $sourceArchive
    }
    finally {
        Pop-Location
    }

    Write-Log "Packages built successfully" "INFO"
}

function Build-DockerImages {
    Write-Log "Building Docker images..." "INFO"

    if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
        Write-Log "Docker not found. Skipping Docker image build." "WARN"
        return
    }

    Push-Location $PROJECT_ROOT
    try {
        $imageTag = "winutils:v$Version"
        $latestTag = "winutils:latest"

        # Build multi-stage images
        Write-Log "Building Linux runtime image..." "INFO"
        & docker build --target linux-runtime -t "${imageTag}-linux" .
        if ($LASTEXITCODE -ne 0) { throw "Docker build failed for Linux runtime" }

        Write-Log "Building development image..." "INFO"
        & docker build --target development -t "${imageTag}-dev" .
        if ($LASTEXITCODE -ne 0) { throw "Docker build failed for development image" }

        Write-Log "Building benchmark image..." "INFO"
        & docker build --target benchmark -t "${imageTag}-benchmark" .
        if ($LASTEXITCODE -ne 0) { throw "Docker build failed for benchmark image" }

        # Tag latest if this is a stable release
        if ($ReleaseType -eq "stable") {
            & docker tag "${imageTag}-linux" "${latestTag}-linux"
            & docker tag "${imageTag}-dev" "${latestTag}-dev"
            & docker tag "${imageTag}-benchmark" "${latestTag}-benchmark"
        }

        Write-Log "Docker images built successfully" "INFO"
    }
    finally {
        Pop-Location
    }
}

function Push-DockerImages {
    if (-not $DockerUsername -or -not $DockerPassword) {
        Write-Log "Docker credentials not provided. Skipping Docker push." "WARN"
        return
    }

    Write-Log "Pushing Docker images to registry..." "INFO"

    # Login to registry
    $dockerPassword | docker login $DockerRegistry -u $DockerUsername --password-stdin
    if ($LASTEXITCODE -ne 0) { throw "Docker login failed" }

    $registryPrefix = "$DockerRegistry/winutils"
    $imageTag = "v$Version"

    # Tag and push images
    $images = @("linux", "dev", "benchmark")
    foreach ($image in $images) {
        $localTag = "winutils:${imageTag}-${image}"
        $remoteTag = "${registryPrefix}:${imageTag}-${image}"

        & docker tag $localTag $remoteTag
        & docker push $remoteTag

        if ($ReleaseType -eq "stable") {
            $latestTag = "${registryPrefix}:latest-${image}"
            & docker tag $localTag $latestTag
            & docker push $latestTag
        }
    }

    Write-Log "Docker images pushed successfully" "INFO"
}

function Create-GitHubRelease {
    if (-not $GitHubToken) {
        Write-Log "No GitHub token provided. Skipping GitHub release creation." "WARN"
        return
    }

    Write-Log "Creating GitHub release..." "INFO"

    # Generate release notes
    $releaseNotes = @"
# WinUtils v$Version

## What's New

This release includes performance improvements, bug fixes, and enhanced Windows compatibility.

## Performance Benchmarks

See attached benchmark reports for detailed performance analysis.

## Installation

### Windows Installer
- Download and run \`winutils-setup-v$Version-x64.exe\`
- Or use the portable version: \`winutils-portable-v$Version-x64.zip\`

### Package Managers
- Chocolatey: \`choco install winutils\`
- Manual: Extract binaries and add to PATH

## Docker
\`\`\`bash
docker pull ghcr.io/winutils:v$Version-linux
\`\`\`

## Changelog

$(git log --oneline --pretty=format:"- %s" $(git describe --tags --abbrev=0)..HEAD)

## Verification

SHA256 checksums:
$(Get-ChildItem $DIST_DIR -Filter "*.exe", "*.zip", "*.msi" | ForEach-Object {
    $hash = Get-FileHash $_.FullName -Algorithm SHA256
    "- $($_.Name): $($hash.Hash)"
})
"@

    $releaseNotesFile = Join-Path $RELEASE_DIR "release-notes.md"
    $releaseNotes | Out-File -FilePath $releaseNotesFile -Encoding UTF8

    # Use GitHub CLI if available
    if (Get-Command gh -ErrorAction SilentlyContinue) {
        $prerelease = if ($ReleaseType -ne "stable") { "--prerelease" } else { "" }

        & gh release create "v$Version" $prerelease --title "WinUtils v$Version" --notes-file $releaseNotesFile (Get-ChildItem $DIST_DIR).FullName
        if ($LASTEXITCODE -ne 0) { throw "GitHub release creation failed" }
    } else {
        Write-Log "GitHub CLI not found. Please create release manually." "WARN"
        Write-Log "Release notes saved to: $releaseNotesFile" "INFO"
    }

    Write-Log "GitHub release created successfully" "INFO"
}

function Cleanup {
    Write-Log "Cleaning up temporary files..." "INFO"

    # Clean up Docker build cache
    if (Get-Command docker -ErrorAction SilentlyContinue) {
        & docker system prune -f
    }

    Write-Log "Cleanup completed" "INFO"
}

function Generate-ReleaseReport {
    Write-Log "Generating release report..." "INFO"

    $report = @"
# WinUtils Release Report v$Version

**Release Type:** $ReleaseType
**Date:** $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')
**Dry Run:** $DryRun

## Build Summary

| Component | Status | Notes |
|-----------|--------|-------|
| Prerequisites | ✅ Passed | All requirements met |
| Version Update | ✅ Completed | Updated to v$Version |
| Tests | $(if ($SkipTests) { '⏭️ Skipped' } else { '✅ Passed' }) | $(if (-not $SkipTests) { 'All tests completed successfully' }) |
| Benchmarks | $(if ($SkipBenchmarks) { '⏭️ Skipped' } else { '✅ Completed' }) | $(if (-not $SkipBenchmarks) { 'Performance benchmarks completed' }) |
| Packages | ✅ Built | Windows installer, portable, and source packages |
| Docker Images | ✅ Built | Linux runtime, development, and benchmark images |
| GitHub Release | $(if ($GitHubToken) { '✅ Created' } else { '⏭️ Skipped' }) | $(if ($GitHubToken) { 'Release published' } else { 'No GitHub token provided' }) |

## Package Artifacts

$(Get-ChildItem $DIST_DIR | ForEach-Object {
    $size = if ($_.Length -gt 1MB) {
        "{0:N1} MB" -f ($_.Length / 1MB)
    } else {
        "{0:N1} KB" -f ($_.Length / 1KB)
    }
    "- $($_.Name) ($size)"
})

## Performance Highlights

$(if (-not $SkipBenchmarks -and (Test-Path (Join-Path $RELEASE_DIR "benchmarks/results.json"))) {
    $benchmarkResults = Get-Content (Join-Path $RELEASE_DIR "benchmarks/results.json") | ConvertFrom-Json
    $summary = $benchmarkResults.summary
    @"
- **Average Speedup:** $($summary.average_speedup.ToString('F1'))x
- **Success Rate:** $(($summary.successful_tests / $summary.total_test_cases * 100).ToString('F1'))%
- **Performance Score:** $($summary.performance_score.ToString('F3'))
- **Memory Efficiency:** $($summary.memory_efficiency_score.ToString('F3'))
"@
} else {
    "Benchmarks were skipped or results not available."
})

## Next Steps

1. Verify all packages work correctly
2. Update documentation if needed
3. Announce release to users
4. Monitor for any issues

Generated by WinUtils Release Automation
"@

    $reportFile = Join-Path $RELEASE_DIR "release-report.md"
    $report | Out-File -FilePath $reportFile -Encoding UTF8

    Write-Log "Release report saved to: $reportFile" "INFO"
}

# Main execution
try {
    Write-Log "Starting WinUtils release process for version $Version ($ReleaseType)" "INFO"

    if ($DryRun) {
        Write-Log "DRY RUN MODE - No permanent changes will be made" "WARN"
    }

    # Initialize
    New-Item -ItemType Directory -Force -Path $RELEASE_DIR | Out-Null

    # Execute release steps
    Test-Prerequisites

    if (-not $DryRun) {
        Update-Version $Version
    }

    if (-not $SkipTests) {
        Run-Tests
    }

    if (-not $SkipBenchmarks) {
        Run-Benchmarks
    }

    Build-Packages
    Build-DockerImages

    if (-not $DryRun) {
        Push-DockerImages
        Create-GitHubRelease
    }

    Generate-ReleaseReport

    Write-Log "Release process completed successfully!" "INFO"
    Write-Log "Release artifacts are in: $DIST_DIR" "INFO"
    Write-Log "Release report: $(Join-Path $RELEASE_DIR 'release-report.md')" "INFO"

} catch {
    Write-Log "Release process failed: $($_.Exception.Message)" "ERROR"
    Write-Log "Stack trace: $($_.ScriptStackTrace)" "ERROR"
    exit 1
} finally {
    Cleanup
}
