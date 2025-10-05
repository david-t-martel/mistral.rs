# Generate Software Bill of Materials (SBOM) for WinUtils
# Supports multiple SBOM formats for security compliance
# Usage: .\scripts\generate-sbom.ps1

param(
    [string]$OutputDir = "sbom-output",
    [switch]$All = $false,
    [switch]$Json = $false,
    [switch]$Spdx = $false,
    [switch]$CycloneDx = $false,
    [switch]$Html = $false
)

# Color output
function Write-Status($message) { Write-Host "[$([datetime]::Now.ToString('HH:mm:ss'))] " -NoNewline -ForegroundColor Blue; Write-Host $message }
function Write-Success($message) { Write-Host "✓ " -NoNewline -ForegroundColor Green; Write-Host $message }
function Write-Error($message) { Write-Host "✗ " -NoNewline -ForegroundColor Red; Write-Host $message }
function Write-Warning($message) { Write-Host "⚠ " -NoNewline -ForegroundColor Yellow; Write-Host $message }

# Check prerequisites
function Check-Prerequisites {
    Write-Status "Checking prerequisites..."

    $missing = @()

    # Check Rust
    if (!(Get-Command cargo -ErrorAction SilentlyContinue)) {
        $missing += "cargo (Rust)"
    }

    # Check for SBOM tools
    $tools = @{
        "cargo-sbom" = $false
        "cargo-cyclonedx" = $false
        "cargo-license" = $false
        "cargo-audit" = $false
    }

    foreach ($tool in $tools.Keys) {
        if (Get-Command $tool -ErrorAction SilentlyContinue) {
            $tools[$tool] = $true
            Write-Success "$tool is installed"
        } else {
            Write-Warning "$tool is not installed"
        }
    }

    if ($missing.Count -gt 0) {
        Write-Error "Missing required tools: $($missing -join ', ')"
        exit 1
    }

    return $tools
}

# Install missing SBOM tools
function Install-SbomTools {
    param($tools)

    Write-Status "Installing missing SBOM tools..."

    foreach ($tool in $tools.Keys) {
        if (!$tools[$tool]) {
            Write-Status "Installing $tool..."
            cargo install $tool --locked
            if ($LASTEXITCODE -eq 0) {
                Write-Success "$tool installed successfully"
                $tools[$tool] = $true
            } else {
                Write-Error "Failed to install $tool"
            }
        }
    }

    return $tools
}

# Generate basic SBOM with cargo-sbom
function Generate-CargoSbom {
    param($outputDir)

    Write-Status "Generating SBOM with cargo-sbom..."

    $sbomFile = Join-Path $outputDir "sbom-cargo.json"
    cargo sbom | Out-File -FilePath $sbomFile -Encoding UTF8

    if (Test-Path $sbomFile) {
        Write-Success "Generated: $sbomFile"

        # Generate summary
        $sbom = Get-Content $sbomFile | ConvertFrom-Json
        $packageCount = ($sbom.packages | Measure-Object).Count
        Write-Status "Found $packageCount packages"
    } else {
        Write-Error "Failed to generate cargo SBOM"
    }
}

# Generate CycloneDX format SBOM
function Generate-CycloneDxSbom {
    param($outputDir)

    Write-Status "Generating CycloneDX SBOM..."

    $formats = @("json", "xml")

    foreach ($format in $formats) {
        $sbomFile = Join-Path $outputDir "sbom-cyclonedx.$format"
        cargo cyclonedx --format $format | Out-File -FilePath $sbomFile -Encoding UTF8

        if (Test-Path $sbomFile) {
            Write-Success "Generated: $sbomFile"
        } else {
            Write-Error "Failed to generate CycloneDX $format"
        }
    }
}

# Generate license report
function Generate-LicenseReport {
    param($outputDir)

    Write-Status "Generating license report..."

    $licenseFile = Join-Path $outputDir "licenses.txt"
    cargo license | Out-File -FilePath $licenseFile -Encoding UTF8

    if (Test-Path $licenseFile) {
        Write-Success "Generated: $licenseFile"

        # Generate license summary
        $licenses = Get-Content $licenseFile
        $licenseTypes = @{}
        foreach ($line in $licenses) {
            if ($line -match '\(([^)]+)\)$') {
                $license = $matches[1]
                if ($licenseTypes.ContainsKey($license)) {
                    $licenseTypes[$license]++
                } else {
                    $licenseTypes[$license] = 1
                }
            }
        }

        Write-Status "License Summary:"
        foreach ($license in $licenseTypes.Keys | Sort-Object) {
            Write-Host "  $license : $($licenseTypes[$license]) packages" -ForegroundColor Cyan
        }
    } else {
        Write-Error "Failed to generate license report"
    }
}

# Generate dependency tree
function Generate-DependencyTree {
    param($outputDir)

    Write-Status "Generating dependency tree..."

    $treeFile = Join-Path $outputDir "dependency-tree.txt"
    cargo tree --all-features | Out-File -FilePath $treeFile -Encoding UTF8

    if (Test-Path $treeFile) {
        Write-Success "Generated: $treeFile"

        # Generate duplicate analysis
        $duplicatesFile = Join-Path $outputDir "duplicate-dependencies.txt"
        cargo tree --duplicates | Out-File -FilePath $duplicatesFile -Encoding UTF8

        if (Test-Path $duplicatesFile) {
            Write-Success "Generated: $duplicatesFile"
            $duplicates = Get-Content $duplicatesFile | Select-String -Pattern "^[a-zA-Z]" | Measure-Object
            if ($duplicates.Count -gt 0) {
                Write-Warning "Found $($duplicates.Count) duplicate dependencies"
            }
        }
    } else {
        Write-Error "Failed to generate dependency tree"
    }
}

# Generate vulnerability report
function Generate-VulnerabilityReport {
    param($outputDir)

    Write-Status "Generating vulnerability report..."

    $vulnFile = Join-Path $outputDir "vulnerabilities.txt"
    cargo audit 2>&1 | Out-File -FilePath $vulnFile -Encoding UTF8

    if (Test-Path $vulnFile) {
        Write-Success "Generated: $vulnFile"

        # Check for vulnerabilities
        $content = Get-Content $vulnFile -Raw
        if ($content -match "(\d+) vulnerability") {
            Write-Warning "Found $($matches[1]) vulnerabilities - review required!"
        } else {
            Write-Success "No known vulnerabilities found"
        }
    } else {
        Write-Error "Failed to generate vulnerability report"
    }
}

# Generate HTML report
function Generate-HtmlReport {
    param($outputDir)

    Write-Status "Generating HTML report..."

    $htmlFile = Join-Path $outputDir "sbom-report.html"

    # Collect all data
    $sbomData = @{
        Generated = [datetime]::Now.ToString("yyyy-MM-dd HH:mm:ss")
        Project = "WinUtils - Windows Coreutils"
        Path = (Get-Location).Path
    }

    # Read license data
    $licenseFile = Join-Path $outputDir "licenses.txt"
    if (Test-Path $licenseFile) {
        $sbomData.Licenses = Get-Content $licenseFile
    }

    # Read vulnerability data
    $vulnFile = Join-Path $outputDir "vulnerabilities.txt"
    if (Test-Path $vulnFile) {
        $sbomData.Vulnerabilities = Get-Content $vulnFile
    }

    # Generate HTML
    $html = @"
<!DOCTYPE html>
<html>
<head>
    <title>WinUtils SBOM Report</title>
    <style>
        body { font-family: 'Segoe UI', Arial, sans-serif; margin: 20px; background: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        h1 { color: #333; border-bottom: 2px solid #0078d4; padding-bottom: 10px; }
        h2 { color: #0078d4; margin-top: 30px; }
        .info { background: #f0f8ff; padding: 15px; border-radius: 4px; margin: 20px 0; }
        .success { color: #107c10; }
        .warning { color: #ff8c00; }
        .error { color: #e81123; }
        table { width: 100%; border-collapse: collapse; margin: 20px 0; }
        th, td { padding: 10px; text-align: left; border-bottom: 1px solid #ddd; }
        th { background: #0078d4; color: white; }
        tr:hover { background: #f5f5f5; }
        .timestamp { color: #666; font-size: 0.9em; }
        pre { background: #f4f4f4; padding: 10px; border-radius: 4px; overflow-x: auto; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🔒 WinUtils Software Bill of Materials (SBOM)</h1>
        <div class="info">
            <strong>Generated:</strong> $($sbomData.Generated)<br>
            <strong>Project:</strong> $($sbomData.Project)<br>
            <strong>Path:</strong> $($sbomData.Path)
        </div>

        <h2>📊 Summary</h2>
        <ul>
            <li>Total Dependencies: <strong>$(if($sbomData.Licenses) { ($sbomData.Licenses | Measure-Object).Count } else { "N/A" })</strong></li>
            <li>SBOM Formats Generated: JSON, XML, CycloneDX</li>
            <li>Security Scan: $(if($sbomData.Vulnerabilities -match "0 vulnerability") { "<span class='success'>✓ No vulnerabilities</span>" } else { "<span class='warning'>⚠ Review required</span>" })</li>
        </ul>

        <h2>📋 Available Reports</h2>
        <ul>
            <li><a href="sbom-cargo.json">Cargo SBOM (JSON)</a></li>
            <li><a href="sbom-cyclonedx.json">CycloneDX SBOM (JSON)</a></li>
            <li><a href="sbom-cyclonedx.xml">CycloneDX SBOM (XML)</a></li>
            <li><a href="licenses.txt">License Report</a></li>
            <li><a href="dependency-tree.txt">Dependency Tree</a></li>
            <li><a href="vulnerabilities.txt">Vulnerability Report</a></li>
            <li><a href="duplicate-dependencies.txt">Duplicate Dependencies</a></li>
        </ul>

        <h2>🔐 Security Status</h2>
        <pre>$($sbomData.Vulnerabilities | Select-Object -First 20 | Out-String)</pre>

        <p class="timestamp">Report generated on $($sbomData.Generated)</p>
    </div>
</body>
</html>
"@

    $html | Out-File -FilePath $htmlFile -Encoding UTF8

    if (Test-Path $htmlFile) {
        Write-Success "Generated: $htmlFile"
    } else {
        Write-Error "Failed to generate HTML report"
    }
}

# Main execution
function Main {
    Write-Host "`n" -NoNewline
    Write-Host "╔════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║    WinUtils SBOM Generator v1.0       ║" -ForegroundColor Cyan
    Write-Host "╚════════════════════════════════════════╝" -ForegroundColor Cyan
    Write-Host ""

    # Setup output directory
    if (!(Test-Path $OutputDir)) {
        New-Item -ItemType Directory -Path $OutputDir | Out-Null
        Write-Success "Created output directory: $OutputDir"
    }

    # Check and install tools
    $tools = Check-Prerequisites
    $tools = Install-SbomTools -tools $tools

    # Generate SBOMs based on parameters
    if ($All -or (!$Json -and !$Spdx -and !$CycloneDx -and !$Html)) {
        # Generate all formats by default
        if ($tools["cargo-sbom"]) { Generate-CargoSbom -outputDir $OutputDir }
        if ($tools["cargo-cyclonedx"]) { Generate-CycloneDxSbom -outputDir $OutputDir }
        if ($tools["cargo-license"]) { Generate-LicenseReport -outputDir $OutputDir }
        Generate-DependencyTree -outputDir $OutputDir
        if ($tools["cargo-audit"]) { Generate-VulnerabilityReport -outputDir $OutputDir }
        Generate-HtmlReport -outputDir $OutputDir
    } else {
        # Generate specific formats
        if ($Json -and $tools["cargo-sbom"]) { Generate-CargoSbom -outputDir $OutputDir }
        if ($CycloneDx -and $tools["cargo-cyclonedx"]) { Generate-CycloneDxSbom -outputDir $OutputDir }
        if ($Html) { Generate-HtmlReport -outputDir $OutputDir }
    }

    Write-Host ""
    Write-Success "SBOM generation complete!"
    Write-Status "Output directory: $OutputDir"

    # Open output directory
    if ($Host.UI.PromptForChoice("Open Results", "Open output directory?", @("&Yes", "&No"), 0) -eq 0) {
        Start-Process explorer.exe -ArgumentList (Resolve-Path $OutputDir).Path
    }
}

# Run main function
Main
