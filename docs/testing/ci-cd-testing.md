# CI/CD Testing Guide

## Overview

The mistral.rs project uses a comprehensive CI/CD pipeline that combines GitHub Actions workflows, local git hooks, and automated validation to ensure code quality and reliability across all platforms.

## CI/CD Architecture

### Pipeline Stages

```
Developer Machine          GitHub                    Deployment
    │                        │                           │
    ├─> Pre-commit ─────────├─> PR Checks ─────────────├─> Release
    │   - Format             │   - Build all platforms │   - Tag version
    │   - Lint               │   - Run tests           │   - Build artifacts
    │   - Quick tests        │   - MCP validation      │   - Upload to releases
    │                        │   - Coverage reports    │   - Deploy docs
    └─> Pre-push ───────────└─> Main Branch ──────────└─> Publish
        - Full tests             - Integration tests        - crates.io
        - Build validation       - Performance bench        - Docker Hub
                                - Security scan            - PyPI
```

## GitHub Actions Workflows

### Main CI Workflow

**File**: `.github/workflows/ci.yml`

```yaml
name: Continuous Integration

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]
  schedule:
    - cron: '0 0 * * 0'  # Weekly on Sunday

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -D warnings
  RUST_BACKTRACE: 1

jobs:
  # 1. Format and Lint Check
  check:
    name: Format and Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache
        uses: Swatinem/rust-cache@v2

      - name: Check formatting
        run: make fmt-check

      - name: Run Clippy
        run: make lint-check

  # 2. Build Matrix
  build:
    name: Build ${{ matrix.os }} ${{ matrix.features }}
    needs: check
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            features: ""
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            features: "cuda,flash-attn"
            target: x86_64-unknown-linux-gnu
          - os: windows-latest
            features: "cuda,flash-attn,cudnn"
            target: x86_64-pc-windows-msvc
          - os: macos-latest
            features: "metal"
            target: x86_64-apple-darwin

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          target: ${{ matrix.target }}

      - name: Setup CUDA (Linux)
        if: matrix.os == 'ubuntu-latest' && contains(matrix.features, 'cuda')
        uses: Jimver/cuda-toolkit@v0.2.11
        with:
          cuda: '12.2.0'

      - name: Setup CUDA (Windows)
        if: matrix.os == 'windows-latest' && contains(matrix.features, 'cuda')
        shell: powershell
        run: |
          choco install cuda --version=12.2.0

      - name: Cache
        uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.os }}-${{ matrix.features }}

      - name: Build
        run: |
          if [ -n "${{ matrix.features }}" ]; then
            cargo build --release --features "${{ matrix.features }}"
          else
            cargo build --release
          fi
        shell: bash

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: mistralrs-${{ matrix.os }}-${{ matrix.target }}
          path: |
            target/release/mistralrs-server${{ matrix.os == 'windows-latest' && '.exe' || '' }}

  # 3. Test Suite
  test:
    name: Test Suite
    needs: build
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]

    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache
        uses: Swatinem/rust-cache@v2

      - name: Download test models
        run: |
          # Download smallest test model
          curl -L -o test-model.gguf \
            https://huggingface.co/Qwen/Qwen2.5-1.5B-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf

      - name: Run tests
        run: make test
        env:
          TEST_MODEL_PATH: ./test-model.gguf

      - name: Generate coverage
        if: matrix.os == 'ubuntu-latest'
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --workspace --out xml

      - name: Upload coverage
        if: matrix.os == 'ubuntu-latest'
        uses: codecov/codecov-action@v3
        with:
          files: ./cobertura.xml

  # 4. Integration Tests
  integration:
    name: Integration Tests
    needs: build
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest]

    steps:
      - uses: actions/checkout@v4

      - name: Download artifacts
        uses: actions/download-artifact@v3
        with:
          name: mistralrs-${{ matrix.os }}-${{ matrix.os == 'windows-latest' && 'x86_64-pc-windows-msvc' || 'x86_64-unknown-linux-gnu' }}

      - name: Make binary executable
        if: matrix.os != 'windows-latest'
        run: chmod +x mistralrs-server

      - name: Run integration tests
        shell: pwsh
        run: |
          ./tests/run-all-tests.ps1 -Suite integration -CI

      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: integration-results-${{ matrix.os }}
          path: tests/results/

  # 5. Documentation
  docs:
    name: Documentation
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@nightly

      - name: Build documentation
        run: cargo doc --all-features --no-deps

      - name: Deploy to GitHub Pages
        if: github.ref == 'refs/heads/main'
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./target/doc
```

### MCP Validation Workflow

**File**: `.github/workflows/mcp-validation.yml`

```yaml
name: MCP Server Validation

on:
  push:
    paths:
      - 'mistralrs-mcp/**'
      - 'tests/mcp/**'
      - '.github/workflows/mcp-validation.yml'
  pull_request:
    paths:
      - 'mistralrs-mcp/**'

jobs:
  validate-mcp:
    name: Validate MCP Servers
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest]
        node: [18, 20]

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: ${{ matrix.node }}

      - name: Install MCP servers
        run: |
          npm install -g @modelcontextprotocol/server-memory
          npm install -g @modelcontextprotocol/server-filesystem
          npm install -g @modelcontextprotocol/server-github

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Build mistralrs
        run: cargo build --release --features mcp

      - name: Test MCP configuration
        shell: pwsh
        run: |
          ./tests/run-all-tests.ps1 -Suite mcp -CI

      - name: Validate with MCP Inspector
        run: |
          npx @modelcontextprotocol/inspector --cli --config tests/mcp/MCP_CONFIG.json

      - name: Upload MCP logs
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: mcp-logs-${{ matrix.os }}-node${{ matrix.node }}
          path: tests/results/mcp-*.log
```

### PowerShell Test Workflow

**File**: `.github/workflows/powershell-tests.yml`

```yaml
name: PowerShell Test Scripts

on:
  push:
    paths:
      - 'tests/**/*.ps1'
      - 'scripts/**/*.ps1'
  pull_request:
    paths:
      - 'tests/**/*.ps1'

jobs:
  validate-scripts:
    name: Validate PowerShell Scripts
    runs-on: windows-latest

    steps:
      - uses: actions/checkout@v4

      - name: Install PSScriptAnalyzer
        shell: pwsh
        run: |
          Install-Module -Name PSScriptAnalyzer -Force -Scope CurrentUser

      - name: Run PSScriptAnalyzer
        shell: pwsh
        run: |
          $results = Invoke-ScriptAnalyzer -Path . -Recurse -Include *.ps1
          if ($results) {
            $results | Format-Table -AutoSize
            throw "PSScriptAnalyzer found issues"
          }

      - name: Validate test runner
        shell: pwsh
        run: |
          ./tests/validate-test-runner.ps1

      - name: Test script syntax
        shell: pwsh
        run: |
          Get-ChildItem -Path . -Filter *.ps1 -Recurse | ForEach-Object {
            $null = [System.Management.Automation.PSParser]::Tokenize(
              (Get-Content $_.FullName -Raw), [ref]$null
            )
            Write-Host "✓ $($_.Name) syntax valid" -ForegroundColor Green
          }
```

### Release Workflow

**File**: `.github/workflows/release.yml`

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write

jobs:
  create-release:
    name: Create Release
    runs-on: ubuntu-latest
    outputs:
      upload_url: ${{ steps.create_release.outputs.upload_url }}

    steps:
      - uses: actions/checkout@v4

      - name: Create Release
        id: create_release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref }}
          release_name: Release ${{ github.ref }}
          draft: false
          prerelease: ${{ contains(github.ref, 'beta') || contains(github.ref, 'rc') }}
          body: |
            # Release ${{ github.ref }}

            ## Changes
            See [CHANGELOG.md](CHANGELOG.md) for details.

            ## Downloads
            Binaries are available below for:
            - Windows (x64) with CUDA support
            - Linux (x64) with CUDA support
            - macOS (x64/ARM64) with Metal support

  build-release:
    name: Build Release ${{ matrix.os }}
    needs: create-release
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            features: cuda,flash-attn
            artifact: mistralrs-linux-x64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            features: cuda,flash-attn,cudnn
            artifact: mistralrs-windows-x64
          - os: macos-latest
            target: x86_64-apple-darwin
            features: metal
            artifact: mistralrs-macos-x64
          - os: macos-latest
            target: aarch64-apple-darwin
            features: metal
            artifact: mistralrs-macos-arm64

    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          target: ${{ matrix.target }}

      - name: Build release binary
        run: |
          cargo build --release --target ${{ matrix.target }} --features "${{ matrix.features }}"

      - name: Package binary
        shell: bash
        run: |
          cd target/${{ matrix.target }}/release
          if [[ "${{ matrix.os }}" == "windows-latest" ]]; then
            7z a ../../../${{ matrix.artifact }}.zip mistralrs-server.exe
          else
            tar czf ../../../${{ matrix.artifact }}.tar.gz mistralrs-server
          fi

      - name: Upload Release Asset
        uses: actions/upload-release-asset@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          upload_url: ${{ needs.create-release.outputs.upload_url }}
          asset_path: ./${{ matrix.artifact }}.${{ matrix.os == 'windows-latest' && 'zip' || 'tar.gz' }}
          asset_name: ${{ matrix.artifact }}.${{ matrix.os == 'windows-latest' && 'zip' || 'tar.gz' }}
          asset_content_type: ${{ matrix.os == 'windows-latest' && 'application/zip' || 'application/gzip' }}
```

## Local Git Hooks

### Pre-commit Hook

**File**: `.git/hooks/pre-commit`

```bash
#!/bin/bash
set -e

echo "Running pre-commit checks..."

# 1. Format check
echo "Checking formatting..."
make fmt-check || {
    echo "Format check failed. Run 'make fmt' to fix."
    exit 1
}

# 2. Lint check
echo "Running clippy..."
make lint-check || {
    echo "Clippy found issues. Run 'make lint-fix' to fix."
    exit 1
}

# 3. Quick compilation check
echo "Checking compilation..."
make check || {
    echo "Compilation check failed."
    exit 1
}

# 4. Check for large files
echo "Checking file sizes..."
files=$(git diff --cached --name-only)
for file in $files; do
    if [ -f "$file" ]; then
        size=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null)
        if [ "$size" -gt 10485760 ]; then  # 10MB
            echo "Error: $file is larger than 10MB"
            exit 1
        fi
    fi
done

# 5. Check for secrets
echo "Checking for secrets..."
if command -v detect-secrets >/dev/null 2>&1; then
    detect-secrets scan --baseline .secrets.baseline
fi

echo "✓ Pre-commit checks passed"
```

### Pre-push Hook

**File**: `.git/hooks/pre-push`

```bash
#!/bin/bash
set -e

echo "Running pre-push checks..."

# 1. Run tests
echo "Running tests..."
make test || {
    echo "Tests failed. Fix before pushing."
    exit 1
}

# 2. Check documentation
echo "Building documentation..."
cargo doc --no-deps --quiet || {
    echo "Documentation build failed."
    exit 1
}

# 3. Security audit
echo "Running security audit..."
cargo audit || {
    echo "Security vulnerabilities found. Run 'cargo audit fix' or update dependencies."
    exit 1
}

# 4. Verify binary builds
echo "Verifying release build..."
make check-release || {
    echo "Release build check failed."
    exit 1
}

echo "✓ Pre-push checks passed"
```

### Hook Installation Script

**File**: `scripts/hooks/install-hooks.sh`

```bash
#!/bin/bash

HOOKS_DIR=".git/hooks"
SCRIPTS_DIR="scripts/hooks"

echo "Installing git hooks..."

# Make hooks directory if it doesn't exist
mkdir -p "$HOOKS_DIR"

# Copy hooks
for hook in pre-commit pre-push commit-msg; do
    if [ -f "$SCRIPTS_DIR/$hook" ]; then
        cp "$SCRIPTS_DIR/$hook" "$HOOKS_DIR/$hook"
        chmod +x "$HOOKS_DIR/$hook"
        echo "✓ Installed $hook hook"
    fi
done

echo "Git hooks installed successfully"
```

## Local CI Simulation

### Running CI Locally

**Script**: `scripts/ci/run-ci-local.ps1`

```powershell
<#
.SYNOPSIS
    Simulate CI pipeline locally before pushing
#>

param(
    [switch]$Quick,  # Skip slow tests
    [switch]$Full    # Run everything including release builds
)

$ErrorActionPreference = "Stop"

Write-Host "=== LOCAL CI SIMULATION ===" -ForegroundColor Cyan

# Track timing
$startTime = Get-Date
$results = @{}

function Run-Step {
    param(
        [string]$Name,
        [scriptblock]$Action
    )

    Write-Host "`n[$($results.Count + 1)] $Name" -ForegroundColor Yellow
    $stepStart = Get-Date

    try {
        & $Action
        $results[$Name] = @{
            Status = "PASSED"
            Duration = (Get-Date) - $stepStart
        }
        Write-Host "✓ $Name passed" -ForegroundColor Green
    } catch {
        $results[$Name] = @{
            Status = "FAILED"
            Duration = (Get-Date) - $stepStart
            Error = $_
        }
        Write-Host "✗ $Name failed: $_" -ForegroundColor Red
        if (-not $Quick) {
            throw
        }
    }
}

# 1. Environment check
Run-Step "Environment Check" {
    cargo --version
    rustc --version
    node --version
    git --version
}

# 2. Format check
Run-Step "Format Check" {
    make fmt-check
}

# 3. Lint check
Run-Step "Lint Check" {
    make lint-check
}

# 4. Compilation check
Run-Step "Compilation Check" {
    make check
}

# 5. Unit tests
if (-not $Quick) {
    Run-Step "Unit Tests" {
        make test-unit
    }
}

# 6. Integration tests
Run-Step "Integration Tests" {
    ./tests/run-all-tests.ps1 -Suite integration -CI
}

# 7. MCP tests
if (-not $Quick) {
    Run-Step "MCP Tests" {
        ./tests/run-all-tests.ps1 -Suite mcp -CI
    }
}

# 8. Documentation
Run-Step "Documentation Build" {
    cargo doc --no-deps --quiet
}

# 9. Security audit
Run-Step "Security Audit" {
    cargo audit
}

# 10. Release build
if ($Full) {
    Run-Step "Release Build" {
        make build-cuda-full
    }

    Run-Step "Binary Validation" {
        ./scripts/build/test-build-validation.ps1
    }
}

# Summary
$duration = (Get-Date) - $startTime
Write-Host "`n=== CI SUMMARY ===" -ForegroundColor Cyan
Write-Host "Total Duration: $([math]::Round($duration.TotalMinutes, 2)) minutes"

$passed = ($results.Values | Where-Object { $_.Status -eq "PASSED" }).Count
$failed = ($results.Values | Where-Object { $_.Status -eq "FAILED" }).Count

foreach ($step in $results.GetEnumerator()) {
    $status = $step.Value.Status
    $color = if ($status -eq "PASSED") { "Green" } else { "Red" }
    $symbol = if ($status -eq "PASSED") { "✓" } else { "✗" }
    $time = [math]::Round($step.Value.Duration.TotalSeconds, 2)
    Write-Host "$symbol $($step.Key): $status ($time s)" -ForegroundColor $color
}

Write-Host "`nPassed: $passed, Failed: $failed" -ForegroundColor $(
    if ($failed -eq 0) { "Green" } else { "Red"
)

if ($failed -gt 0) {
    Write-Host "`n⚠ CI would fail with current changes" -ForegroundColor Red
    exit 1
} else {
    Write-Host "`n✓ CI would pass with current changes" -ForegroundColor Green
    exit 0
}
```

## Artifact Management

### Build Artifacts

```yaml
# Store build artifacts between jobs
- name: Cache Rust artifacts
  uses: actions/cache@v3
  with:
    path: |
      ~/.cargo/registry
      ~/.cargo/git
      target
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

# Upload binary artifacts
- name: Upload binary
  uses: actions/upload-artifact@v3
  with:
    name: mistralrs-${{ matrix.os }}
    path: target/release/mistralrs-server*
    retention-days: 7

# Download in another job
- name: Download binary
  uses: actions/download-artifact@v3
  with:
    name: mistralrs-${{ matrix.os }}
```

### Test Results

```yaml
# Upload test results
- name: Upload test results
  if: always()
  uses: actions/upload-artifact@v3
  with:
    name: test-results-${{ matrix.os }}
    path: |
      tests/results/*.json
      tests/results/*.html
      tests/results/*.log

# Publish test report
- name: Publish test report
  uses: dorny/test-reporter@v1
  if: success() || failure()
  with:
    name: Test Results
    path: 'tests/results/*.json'
    reporter: jest-json
```

## Performance Monitoring

### Benchmark Tracking

```yaml
# Run benchmarks and track performance
- name: Run benchmarks
  run: |
    cargo bench --bench inference -- --output-format bencher | tee output.txt

- name: Store benchmark result
  uses: benchmark-action/github-action-benchmark@v1
  with:
    tool: 'cargo'
    output-file-path: output.txt
    github-token: ${{ secrets.GITHUB_TOKEN }}
    auto-push: true
    alert-threshold: '150%'
    comment-on-alert: true
    alert-comment-cc-users: '@maintainers'
```

### Build Time Tracking

```powershell
# Track build times over commits
function Track-BuildTime {
    param([string]$Commit = "HEAD")

    $result = @{
        Commit = git rev-parse --short $Commit
        Date = Get-Date
        BuildTime = 0
    }

    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    make build
    $stopwatch.Stop()

    $result.BuildTime = $stopwatch.Elapsed.TotalMinutes

    # Append to tracking file
    $trackingFile = "build-times.csv"
    if (-not (Test-Path $trackingFile)) {
        "Commit,Date,BuildTime" | Out-File $trackingFile
    }

    "$($result.Commit),$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss'),$($result.BuildTime)" |
        Add-Content $trackingFile

    Write-Host "Build time for $($result.Commit): $([math]::Round($result.BuildTime, 2)) minutes"
}
```

## Security Scanning

### Dependency Scanning

```yaml
# Automated security scanning
- name: Security audit
  run: |
    cargo audit --deny warnings

- name: License check
  run: |
    cargo install cargo-license
    cargo license --json > licenses.json

- name: SBOM generation
  run: |
    cargo install cargo-sbom
    cargo sbom --output-format spdx > sbom.json
```

### Secret Scanning

```yaml
# Detect secrets in code
- name: Detect secrets
  uses: trufflesecurity/trufflehog@main
  with:
    path: ./
    base: ${{ github.event.repository.default_branch }}
    head: HEAD
```

## Deployment Automation

### Docker Deployment

```dockerfile
# Multi-stage Docker build
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

# Build with all features
RUN cargo build --release --features cuda,flash-attn,cudnn

# Runtime image
FROM nvidia/cuda:12.2.0-runtime-ubuntu22.04

COPY --from=builder /app/target/release/mistralrs-server /usr/local/bin/

EXPOSE 8080
CMD ["mistralrs-server", "--port", "8080"]
```

### Kubernetes Deployment

```yaml
# k8s deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mistralrs
spec:
  replicas: 3
  selector:
    matchLabels:
      app: mistralrs
  template:
    metadata:
      labels:
        app: mistralrs
    spec:
      containers:
      - name: mistralrs
        image: mistralrs:latest
        ports:
        - containerPort: 8080
        resources:
          requests:
            memory: "8Gi"
            nvidia.com/gpu: 1
          limits:
            memory: "16Gi"
            nvidia.com/gpu: 1
```

## Monitoring and Alerts

### GitHub Actions Status Badge

```markdown
[![CI](https://github.com/EricLBuehler/mistral.rs/workflows/CI/badge.svg)](https://github.com/EricLBuehler/mistral.rs/actions)
[![Coverage](https://codecov.io/gh/EricLBuehler/mistral.rs/branch/main/graph/badge.svg)](https://codecov.io/gh/EricLBuehler/mistral.rs)
```

### Slack Notifications

```yaml
# Send Slack notifications
- name: Slack Notification
  if: failure()
  uses: 8398a7/action-slack@v3
  with:
    status: ${{ job.status }}
    text: 'Build failed on ${{ github.ref }}'
    webhook_url: ${{ secrets.SLACK_WEBHOOK }}
```

## Best Practices

### 1. Fast Feedback

- Run quick checks first (format, lint)
- Fail fast on critical errors
- Parallelize independent jobs

### 2. Caching Strategy

```yaml
# Effective caching
- uses: actions/cache@v3
  with:
    path: |
      ~/.cargo/registry/index
      ~/.cargo/registry/cache
      ~/.cargo/git
      target
    key: ${{ runner.os }}-cargo-${{ hashFiles('Cargo.lock') }}-${{ hashFiles('rust-toolchain.toml') }}
    restore-keys: |
      ${{ runner.os }}-cargo-${{ hashFiles('Cargo.lock') }}-
      ${{ runner.os }}-cargo-
```

### 3. Matrix Testing

- Test multiple OS versions
- Test multiple Rust versions
- Test feature combinations
- Test with different CUDA versions

### 4. Artifact Retention

```yaml
# Manage artifact storage
- uses: actions/upload-artifact@v3
  with:
    retention-days: 7  # PR artifacts
    retention-days: 30 # Release artifacts
    retention-days: 1  # Temporary artifacts
```

## Troubleshooting CI Issues

### Debugging Failed Builds

```yaml
# Enable debug logging
- name: Debug build
  env:
    ACTIONS_STEP_DEBUG: true
    CARGO_VERBOSE: true
  run: |
    cargo build -vv
```

### SSH into Runner

```yaml
# Debug with tmate
- name: Setup tmate session
  if: ${{ failure() }}
  uses: mxschmitt/action-tmate@v3
  timeout-minutes: 15
```

### Rerun Failed Jobs

```bash
# Using GitHub CLI
gh run rerun <run-id> --failed
```

## Next Steps

- Review [Integration Testing](integration-testing.md) for test details
- See [Build Testing](build-testing.md) for compilation validation
- Check [MCP Testing](mcp-testing.md) for protocol testing
- Read [Testing Migration](../development/testing-migration.md) for updates

---

*Last Updated: 2025*
*Version: 1.0.0*
*Component: CI/CD Pipeline*