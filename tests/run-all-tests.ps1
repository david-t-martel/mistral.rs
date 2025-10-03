<#
.SYNOPSIS
    Master test runner for mistral.rs project - Single entry point for all testing workflows

.DESCRIPTION
    Discovers and executes all test scripts from organized hierarchy:
    - tests/integration/*.ps1
    - tests/mcp/*.ps1
    - scripts/build/test-*.ps1

    Provides unified reporting, MCP server lifecycle management, and CI integration.

.PARAMETER Suite
    Test suite to run: all, integration, mcp, build, quick

.PARAMETER OutputFormat
    Output format: console, json, markdown, html

.PARAMETER OutputFile
    Output file path (without extension, will be added based on format)

.PARAMETER Verbose
    Enable verbose output

.PARAMETER FailFast
    Stop on first failure

.PARAMETER Coverage
    Generate coverage report

.PARAMETER CI
    CI mode (stricter checks, no interactive prompts)

.PARAMETER Parallel
    Run tests in parallel where safe

.EXAMPLE
    .\run-all-tests.ps1 -Suite quick
    .\run-all-tests.ps1 -Suite all -OutputFormat json -OutputFile results.json
    .\run-all-tests.ps1 -Suite mcp -Verbose -FailFast
#>

param(
    [ValidateSet('all', 'integration', 'mcp', 'build', 'quick')]
    [string]$Suite = 'all',

    [ValidateSet('json', 'markdown', 'html', 'console')]
    [string]$OutputFormat = 'console',

    [string]$OutputFile = "tests/results/run-all-tests-$(Get-Date -Format 'yyyyMMdd-HHmmss')",

    [switch]$Verbose,
    [switch]$FailFast,
    [switch]$Coverage,
    [switch]$CI,
    [switch]$Parallel
)

$ErrorActionPreference = "Continue"
$StartTime = Get-Date

# Color output helpers
function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$Color = "White"
    )
    if ($CI) {
        Write-Host $Message
    } else {
        Write-Host $Message -ForegroundColor $Color
    }
}

function Write-Section {
    param([string]$Title)
    Write-ColorOutput "`n$('=' * 80)" "Cyan"
    Write-ColorOutput "  $Title" "Cyan"
    Write-ColorOutput "$('=' * 80)`n" "Cyan"
}

function Write-Success {
    param([string]$Message)
    Write-ColorOutput "✓ $Message" "Green"
}

function Write-Failure {
    param([string]$Message)
    Write-ColorOutput "✗ $Message" "Red"
}

function Write-Warning {
    param([string]$Message)
    Write-ColorOutput "⚠ $Message" "Yellow"
}

function Write-Info {
    param([string]$Message)
    Write-ColorOutput "ℹ $Message" "Cyan"
}

# Test result tracking
$script:TestResults = @{
    Tests = @()
    Summary = @{
        Total = 0
        Passed = 0
        Failed = 0
        Skipped = 0
        Warnings = 0
        Duration = 0
    }
    StartTime = $StartTime
    EndTime = $null
    Suite = $Suite
    Environment = @{
        OS = $PSVersionTable.OS
        PowerShell = $PSVersionTable.PSVersion.ToString()
        Hostname = $env:COMPUTERNAME
        User = $env:USERNAME
    }
}

# MCP server management
$script:MCPServers = @()

function Start-MCPServers {
    Write-Section "Starting MCP Servers"

    $configPath = "tests/mcp/MCP_CONFIG.json"
    if (-not (Test-Path $configPath)) {
        Write-Warning "MCP config not found: $configPath"
        return @()
    }

    try {
        $config = Get-Content $configPath -Raw | ConvertFrom-Json
        $servers = @()

        foreach ($server in $config.mcpServers.PSObject.Properties) {
            $serverName = $server.Name
            $serverConfig = $server.Value

            Write-Info "Starting MCP server: $serverName"

            $processArgs = @{
                FilePath = $serverConfig.command
                ArgumentList = $serverConfig.args
                NoNewWindow = $true
                PassThru = $true
                RedirectStandardOutput = "tests/results/mcp-$serverName.out"
                RedirectStandardError = "tests/results/mcp-$serverName.err"
            }

            if ($serverConfig.env) {
                $env:NODE_PATH = $serverConfig.env.NODE_PATH
            }

            try {
                $process = Start-Process @processArgs

                # Wait for server to be ready (max 5 seconds)
                Start-Sleep -Milliseconds 500
                if (-not $process.HasExited) {
                    Write-Success "Started $serverName (PID: $($process.Id))"
                    $servers += @{
                        Name = $serverName
                        Process = $process
                        PID = $process.Id
                    }
                } else {
                    Write-Failure "Failed to start $serverName (exited immediately)"
                }
            } catch {
                Write-Failure "Error starting $serverName : $_"
            }
        }

        $script:MCPServers = $servers
        return $servers
    } catch {
        Write-Failure "Failed to read MCP config: $_"
        return @()
    }
}

function Stop-MCPServers {
    param([array]$Servers = $script:MCPServers)

    if ($Servers.Count -eq 0) {
        return
    }

    Write-Section "Stopping MCP Servers"

    foreach ($server in $Servers) {
        try {
            if ($server.Process -and -not $server.Process.HasExited) {
                Write-Info "Stopping $($server.Name) (PID: $($server.PID))"

                # Try graceful shutdown first
                $server.Process.CloseMainWindow() | Out-Null
                $stopped = $server.Process.WaitForExit(3000)

                # Force kill if still running
                if (-not $stopped) {
                    Write-Warning "Force killing $($server.Name)"
                    Stop-Process -Id $server.PID -Force -ErrorAction SilentlyContinue
                }

                Write-Success "Stopped $($server.Name)"
            }
        } catch {
            Write-Warning "Error stopping $($server.Name): $_"
        }
    }
}

function Test-PreFlightChecks {
    Write-Section "Pre-Flight Checks"

    $checks = @{
        "Makefile exists" = Test-Path "Makefile"
        "Binary exists" = Test-Path "target/release/mistralrs-server.exe"
        "Tests directory exists" = Test-Path "tests"
        "Results directory exists" = Test-Path "tests/results"
    }

    $allPassed = $true
    foreach ($check in $checks.GetEnumerator()) {
        if ($check.Value) {
            Write-Success $check.Key
        } else {
            Write-Failure $check.Key
            $allPassed = $false
        }
    }

    # Create results directory if missing
    if (-not $checks["Results directory exists"]) {
        New-Item -ItemType Directory -Path "tests/results" -Force | Out-Null
        Write-Info "Created tests/results directory"
    }

    # Check for running MCP servers
    $runningServers = Get-Process -Name "node" -ErrorAction SilentlyContinue |
        Where-Object { $_.CommandLine -like "*@modelcontextprotocol*" }

    if ($runningServers) {
        Write-Warning "Found $($runningServers.Count) running MCP servers"
        if (-not $CI) {
            $response = Read-Host "Stop them? (y/n)"
            if ($response -eq 'y') {
                $runningServers | Stop-Process -Force
                Write-Success "Stopped existing MCP servers"
            }
        }
    }

    if (-not $allPassed -and $CI) {
        throw "Pre-flight checks failed in CI mode"
    }

    return $allPassed
}

function Find-TestScripts {
    param([string]$Suite)

    Write-Section "Discovering Test Scripts"

    $tests = @()

    # Define test categories and their paths
    $testCategories = @{
        integration = @{
            Path = "tests/integration"
            Pattern = "*.ps1"
            Priority = 2
        }
        mcp = @{
            Path = "tests/mcp"
            Pattern = "test-*.ps1"
            Priority = 3
        }
        build = @{
            Path = "scripts/build"
            Pattern = "test-*.ps1"
            Priority = 1
        }
    }

    # Quick suite special case
    if ($Suite -eq 'quick') {
        Write-Info "Quick suite: Running fast checks only"
        return @(
            @{
                Name = "Quick Build Check"
                Path = "make"
                Args = @("check")
                Category = "quick"
                Priority = 1
                EstimatedDuration = 60
            }
        )
    }

    # Discover tests based on suite selection
    foreach ($category in $testCategories.GetEnumerator()) {
        if ($Suite -ne 'all' -and $Suite -ne $category.Key) {
            continue
        }

        $path = $category.Value.Path
        if (-not (Test-Path $path)) {
            Write-Warning "Path not found: $path"
            continue
        }

        $scripts = Get-ChildItem -Path $path -Filter $category.Value.Pattern -File

        foreach ($script in $scripts) {
            $tests += @{
                Name = $script.BaseName
                Path = $script.FullName
                Category = $category.Key
                Priority = $category.Value.Priority
                EstimatedDuration = 120  # Default 2 minutes
            }
            Write-Info "Found: $($script.Name) [$($category.Key)]"
        }
    }

    # Sort by priority (lower first)
    $tests = $tests | Sort-Object Priority, Name

    Write-Info "Total tests discovered: $($tests.Count)"
    return $tests
}

function Invoke-TestScript {
    param(
        [hashtable]$Test,
        [int]$Index,
        [int]$Total
    )

    $testStart = Get-Date
    $testName = $Test.Name
    $progress = "[$Index/$Total]"

    Write-ColorOutput "`n$progress Running: $testName" "Cyan"
    Write-Info "Category: $($Test.Category) | Estimated: $($Test.EstimatedDuration)s"

    $result = @{
        Name = $testName
        Category = $Test.Category
        StartTime = $testStart
        EndTime = $null
        Duration = 0
        ExitCode = -1
        Status = "Failed"
        Output = ""
        ErrorOutput = ""
        Warnings = @()
    }

    try {
        # Special handling for make commands
        if ($Test.Path -eq "make") {
            $process = Start-Process -FilePath "make" -ArgumentList $Test.Args `
                -NoNewWindow -Wait -PassThru `
                -RedirectStandardOutput "tests/results/temp-stdout.txt" `
                -RedirectStandardError "tests/results/temp-stderr.txt"

            $result.Output = Get-Content "tests/results/temp-stdout.txt" -Raw -ErrorAction SilentlyContinue
            $result.ErrorOutput = Get-Content "tests/results/temp-stderr.txt" -Raw -ErrorAction SilentlyContinue
            $result.ExitCode = $process.ExitCode
        }
        # PowerShell scripts
        elseif ($Test.Path -like "*.ps1") {
            $scriptOutput = & $Test.Path -ErrorAction Continue 2>&1
            $result.ExitCode = $LASTEXITCODE
            $result.Output = $scriptOutput | Out-String

            # Try to parse JSON results if available
            $jsonResultPath = $Test.Path -replace '\.ps1$', '-results.json'
            if (Test-Path $jsonResultPath) {
                $jsonResult = Get-Content $jsonResultPath -Raw | ConvertFrom-Json
                if ($jsonResult.warnings) {
                    $result.Warnings = $jsonResult.warnings
                }
            }
        }

        # Determine status
        $result.Status = if ($result.ExitCode -eq 0) { "Passed" } else { "Failed" }

    } catch {
        $result.Status = "Error"
        $result.ErrorOutput = $_.Exception.Message
        Write-Failure "Exception: $($_.Exception.Message)"
    } finally {
        $result.EndTime = Get-Date
        $result.Duration = ($result.EndTime - $result.StartTime).TotalSeconds

        # Display result
        $durationStr = "{0:N2}s" -f $result.Duration
        if ($result.Status -eq "Passed") {
            Write-Success "$testName completed in $durationStr"
        } else {
            Write-Failure "$testName failed in $durationStr (Exit Code: $($result.ExitCode))"
            if ($result.ErrorOutput) {
                Write-ColorOutput "Error: $($result.ErrorOutput)" "Red"
            }
        }

        if ($result.Warnings.Count -gt 0) {
            Write-Warning "$($result.Warnings.Count) warnings"
        }
    }

    # Cleanup temp files
    Remove-Item "tests/results/temp-*.txt" -Force -ErrorAction SilentlyContinue

    return $result
}

function Invoke-TestsSequential {
    param([array]$Tests)

    $results = @()
    $index = 1

    foreach ($test in $Tests) {
        $result = Invoke-TestScript -Test $test -Index $index -Total $Tests.Count
        $results += $result

        # Update summary
        $script:TestResults.Summary.Total++
        switch ($result.Status) {
            "Passed" { $script:TestResults.Summary.Passed++ }
            "Failed" { $script:TestResults.Summary.Failed++ }
            "Skipped" { $script:TestResults.Summary.Skipped++ }
        }
        $script:TestResults.Summary.Warnings += $result.Warnings.Count

        # Fail fast check
        if ($FailFast -and $result.Status -eq "Failed") {
            Write-Warning "Stopping due to -FailFast"
            break
        }

        $index++
    }

    return $results
}

function Invoke-TestsParallel {
    param([array]$Tests)

    Write-Info "Parallel execution not fully implemented - falling back to sequential"
    # TODO: Implement true parallel execution with job management
    return Invoke-TestsSequential -Tests $Tests
}

function Export-Results {
    param(
        [array]$Results,
        [string]$Format,
        [string]$OutputPath
    )

    Write-Section "Generating Report"

    # Update final summary
    $script:TestResults.EndTime = Get-Date
    $script:TestResults.Summary.Duration = ($script:TestResults.EndTime - $script:TestResults.StartTime).TotalSeconds
    $script:TestResults.Tests = $Results

    switch ($Format) {
        "console" {
            Show-ConsoleReport -Results $script:TestResults
        }
        "json" {
            $jsonPath = "$OutputPath.json"
            $script:TestResults | ConvertTo-Json -Depth 10 | Set-Content $jsonPath
            Write-Success "JSON report: $jsonPath"
        }
        "markdown" {
            $mdPath = "$OutputPath.md"
            $md = Format-MarkdownReport -Results $script:TestResults
            $md | Set-Content $mdPath
            Write-Success "Markdown report: $mdPath"
        }
        "html" {
            $htmlPath = "$OutputPath.html"
            $html = Format-HtmlReport -Results $script:TestResults
            $html | Set-Content $htmlPath
            Write-Success "HTML report: $htmlPath"

            if (-not $CI) {
                Start-Process $htmlPath
            }
        }
    }
}

function Show-ConsoleReport {
    param([hashtable]$Results)

    Write-Section "Test Results Summary"

    # Summary table
    Write-ColorOutput "Duration: $("{0:N2}" -f $Results.Summary.Duration)s" "White"
    Write-ColorOutput "Total Tests: $($Results.Summary.Total)" "White"
    Write-ColorOutput "Passed: $($Results.Summary.Passed)" "Green"
    Write-ColorOutput "Failed: $($Results.Summary.Failed)" "Red"
    Write-ColorOutput "Skipped: $($Results.Summary.Skipped)" "Yellow"
    Write-ColorOutput "Warnings: $($Results.Summary.Warnings)" "Yellow"

    $passRate = if ($Results.Summary.Total -gt 0) {
        ($Results.Summary.Passed / $Results.Summary.Total) * 100
    } else { 0 }
    Write-ColorOutput "Pass Rate: $("{0:N1}" -f $passRate)%" $(if ($passRate -ge 90) { "Green" } else { "Yellow" })

    # Failed tests detail
    if ($Results.Summary.Failed -gt 0) {
        Write-Section "Failed Tests"
        foreach ($test in $Results.Tests | Where-Object { $_.Status -eq "Failed" }) {
            Write-Failure "$($test.Name) - Exit Code: $($test.ExitCode)"
            if ($test.ErrorOutput) {
                Write-ColorOutput "  Error: $($test.ErrorOutput)" "DarkRed"
            }
        }
    }

    # Slowest tests
    Write-Section "Slowest Tests"
    $slowest = $Results.Tests | Sort-Object Duration -Descending | Select-Object -First 5
    foreach ($test in $slowest) {
        Write-Info "$($test.Name): $("{0:N2}" -f $test.Duration)s"
    }
}

function Format-MarkdownReport {
    param([hashtable]$Results)

    $md = @"
# Test Results - $($Results.Suite)

**Generated**: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
**Duration**: $("{0:N2}" -f $Results.Summary.Duration)s
**Pass Rate**: $("{0:N1}" -f (($Results.Summary.Passed / $Results.Summary.Total) * 100))%

## Summary

| Metric | Count |
|--------|-------|
| Total Tests | $($Results.Summary.Total) |
| Passed | $($Results.Summary.Passed) ✓ |
| Failed | $($Results.Summary.Failed) ✗ |
| Skipped | $($Results.Summary.Skipped) ⊘ |
| Warnings | $($Results.Summary.Warnings) ⚠ |

## Test Details

| Test Name | Category | Status | Duration | Exit Code |
|-----------|----------|--------|----------|-----------|
"@

    foreach ($test in $Results.Tests) {
        $status = switch ($test.Status) {
            "Passed" { "✓" }
            "Failed" { "✗" }
            "Skipped" { "⊘" }
            default { "?" }
        }
        $md += "`n| $($test.Name) | $($test.Category) | $status | $("{0:N2}" -f $test.Duration)s | $($test.ExitCode) |"
    }

    # Failed tests detail
    if ($Results.Summary.Failed -gt 0) {
        $md += "`n`n## Failed Tests`n"
        foreach ($test in $Results.Tests | Where-Object { $_.Status -eq "Failed" }) {
            $md += "`n### $($test.Name)`n"
            $md += "- **Exit Code**: $($test.ExitCode)`n"
            $md += "- **Duration**: $("{0:N2}" -f $test.Duration)s`n"
            if ($test.ErrorOutput) {
                $md += "- **Error**: ``````$($test.ErrorOutput)```````n"
            }
        }
    }

    return $md
}

function Format-HtmlReport {
    param([hashtable]$Results)

    $passRate = ($Results.Summary.Passed / $Results.Summary.Total) * 100
    $statusColor = if ($passRate -ge 90) { "green" } elseif ($passRate -ge 70) { "orange" } else { "red" }

    $html = @"
<!DOCTYPE html>
<html>
<head>
    <title>Test Results - $($Results.Suite)</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; background: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        h1 { color: #333; border-bottom: 3px solid #4CAF50; padding-bottom: 10px; }
        .summary { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px; margin: 20px 0; }
        .stat-card { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 20px; border-radius: 8px; text-align: center; }
        .stat-card.passed { background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%); }
        .stat-card.failed { background: linear-gradient(135deg, #eb3349 0%, #f45c43 100%); }
        .stat-card.warnings { background: linear-gradient(135deg, #f7971e 0%, #ffd200 100%); }
        .stat-value { font-size: 36px; font-weight: bold; }
        .stat-label { font-size: 14px; opacity: 0.9; margin-top: 5px; }
        table { width: 100%; border-collapse: collapse; margin: 20px 0; }
        th { background: #4CAF50; color: white; padding: 12px; text-align: left; }
        td { padding: 10px; border-bottom: 1px solid #ddd; }
        tr:hover { background: #f5f5f5; }
        .status-passed { color: #4CAF50; font-weight: bold; }
        .status-failed { color: #f44336; font-weight: bold; }
        .status-skipped { color: #ff9800; font-weight: bold; }
        .progress-bar { width: 100%; height: 30px; background: #e0e0e0; border-radius: 15px; overflow: hidden; margin: 20px 0; }
        .progress-fill { height: 100%; background: $statusColor; transition: width 0.3s; display: flex; align-items: center; justify-content: center; color: white; font-weight: bold; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Test Results - $($Results.Suite)</h1>
        <p><strong>Generated:</strong> $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")</p>
        <p><strong>Duration:</strong> $("{0:N2}" -f $Results.Summary.Duration) seconds</p>

        <div class="progress-bar">
            <div class="progress-fill" style="width: $("{0:N1}" -f $passRate)%">
                $("{0:N1}" -f $passRate)% Pass Rate
            </div>
        </div>

        <div class="summary">
            <div class="stat-card">
                <div class="stat-value">$($Results.Summary.Total)</div>
                <div class="stat-label">Total Tests</div>
            </div>
            <div class="stat-card passed">
                <div class="stat-value">$($Results.Summary.Passed)</div>
                <div class="stat-label">Passed</div>
            </div>
            <div class="stat-card failed">
                <div class="stat-value">$($Results.Summary.Failed)</div>
                <div class="stat-label">Failed</div>
            </div>
            <div class="stat-card warnings">
                <div class="stat-value">$($Results.Summary.Warnings)</div>
                <div class="stat-label">Warnings</div>
            </div>
        </div>

        <h2>Test Details</h2>
        <table>
            <thead>
                <tr>
                    <th>Test Name</th>
                    <th>Category</th>
                    <th>Status</th>
                    <th>Duration</th>
                    <th>Exit Code</th>
                </tr>
            </thead>
            <tbody>
"@

    foreach ($test in $Results.Tests) {
        $statusClass = "status-" + $test.Status.ToLower()
        $statusSymbol = switch ($test.Status) {
            "Passed" { "✓" }
            "Failed" { "✗" }
            "Skipped" { "⊘" }
            default { "?" }
        }

        $html += @"
                <tr>
                    <td>$($test.Name)</td>
                    <td>$($test.Category)</td>
                    <td class="$statusClass">$statusSymbol $($test.Status)</td>
                    <td>$("{0:N2}" -f $test.Duration)s</td>
                    <td>$($test.ExitCode)</td>
                </tr>
"@
    }

    $html += @"
            </tbody>
        </table>
    </div>
</body>
</html>
"@

    return $html
}

function Invoke-Cleanup {
    Write-Section "Cleanup"

    # Stop MCP servers
    Stop-MCPServers

    # Archive old results (keep last 10)
    $resultsFiles = Get-ChildItem "tests/results" -Filter "run-all-tests-*" |
        Sort-Object LastWriteTime -Descending |
        Select-Object -Skip 10

    if ($resultsFiles) {
        Write-Info "Archiving $($resultsFiles.Count) old result files"
        $archivePath = "tests/results/archive"
        if (-not (Test-Path $archivePath)) {
            New-Item -ItemType Directory -Path $archivePath -Force | Out-Null
        }
        $resultsFiles | Move-Item -Destination $archivePath -Force
    }

    Write-Success "Cleanup complete"
}

# ============================================================================
# MAIN EXECUTION
# ============================================================================

try {
    Write-ColorOutput @"

╔════════════════════════════════════════════════════════════════════════════╗
║                   mistral.rs Master Test Runner                           ║
║                   Suite: $($Suite.PadRight(58)) ║
╚════════════════════════════════════════════════════════════════════════════╝

"@ "Cyan"

    # 1. Pre-flight checks
    $checksPass = Test-PreFlightChecks
    if (-not $checksPass -and $CI) {
        exit 1
    }

    # 2. Discover tests
    $tests = Find-TestScripts -Suite $Suite

    if ($tests.Count -eq 0) {
        Write-Warning "No tests found for suite: $Suite"
        exit 0
    }

    # Display test plan
    Write-Section "Test Execution Plan"
    Write-Info "Total tests: $($tests.Count)"
    Write-Info "Estimated duration: $([math]::Round(($tests | Measure-Object -Property EstimatedDuration -Sum).Sum / 60, 1)) minutes"

    if (-not $CI) {
        Write-Host "`nPress Enter to continue or Ctrl+C to cancel..." -ForegroundColor Yellow
        Read-Host
    }

    # 3. Start MCP servers if needed
    if ($Suite -eq 'all' -or $Suite -eq 'mcp') {
        Start-MCPServers
    }

    # 4. Execute tests
    Write-Section "Executing Tests"

    $results = if ($Parallel) {
        Invoke-TestsParallel -Tests $tests
    } else {
        Invoke-TestsSequential -Tests $tests
    }

    # 5. Generate report
    Export-Results -Results $results -Format $OutputFormat -OutputPath $OutputFile

    # 6. Final summary
    Write-Section "Final Summary"
    $exitCode = if ($script:TestResults.Summary.Failed -eq 0) { 0 } else { 1 }

    if ($exitCode -eq 0) {
        Write-Success "All tests passed! 🎉"
    } else {
        Write-Failure "$($script:TestResults.Summary.Failed) test(s) failed"
    }

    Write-Info "Total duration: $("{0:N2}" -f $script:TestResults.Summary.Duration)s"

    exit $exitCode

} catch {
    Write-Failure "Fatal error: $_"
    Write-ColorOutput $_.ScriptStackTrace "Red"
    exit 1
} finally {
    # Always cleanup
    Invoke-Cleanup
}
