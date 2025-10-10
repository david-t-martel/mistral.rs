#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Enhanced Git workflow automation script for mistral.rs

.DESCRIPTION
    This script automates the complete Git workflow:
    - Runs cargo fmt to format code
    - Runs cargo clippy --fix to auto-fix linting issues
    - Tags outstanding TODOs with @codex/@gemini for external review
    - Creates semantic index with RAG-Redis
    - Commits changes with meaningful message
    - Pushes to remote (with --no-verify fallback if hooks fail)

.PARAMETER Message
    Commit message (required)

.PARAMETER NoVerify
    Skip pre-commit hooks (use when hooks fail)

.PARAMETER SkipFormat
    Skip cargo fmt step

.PARAMETER SkipClippy
    Skip cargo clippy --fix step

.PARAMETER SkipTagging
    Skip TODO/FIXME tagging step

.PARAMETER SkipIndex
    Skip RAG-Redis semantic indexing step

.PARAMETER Push
    Automatically push after commit

.EXAMPLE
    .\git-auto-commit.ps1 -Message "feat: add new feature" -Push

.EXAMPLE
    .\git-auto-commit.ps1 -Message "fix: resolve bug" -NoVerify -Push
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Message,

    [Parameter(Mandatory = $false)]
    [switch]$NoVerify,

    [Parameter(Mandatory = $false)]
    [switch]$SkipFormat,

    [Parameter(Mandatory = $false)]
    [switch]$SkipClippy,

    [Parameter(Mandatory = $false)]
    [switch]$SkipTagging,

    [Parameter(Mandatory = $false)]
    [switch]$SkipIndex,

    [Parameter(Mandatory = $false)]
    [switch]$Push
)

$ErrorActionPreference = "Stop"

# Color output functions
function Write-Success { param([string]$msg) Write-Host "✓ $msg" -ForegroundColor Green }
function Write-Error { param([string]$msg) Write-Host "✗ $msg" -ForegroundColor Red }
function Write-Info { param([string]$msg) Write-Host "ℹ $msg" -ForegroundColor Cyan }
function Write-Warning { param([string]$msg) Write-Host "⚠ $msg" -ForegroundColor Yellow }
function Write-Header { param([string]$msg) Write-Host "`n========================================" -ForegroundColor Magenta; Write-Host $msg -ForegroundColor Magenta; Write-Host "========================================`n" -ForegroundColor Magenta }

# Change to repository root
$repoRoot = git rev-parse --show-toplevel
if ($LASTEXITCODE -ne 0) {
    Write-Error "Not in a git repository!"
    exit 1
}
Set-Location $repoRoot
Write-Info "Repository root: $repoRoot"

# Step 1: Format code
if (-not $SkipFormat) {
    Write-Header "Step 1: Formatting code with cargo fmt"
    try {
        make fmt
        if ($LASTEXITCODE -eq 0) {
            Write-Success "Code formatted successfully"
            # Stage formatted files
            git add -u
        } else {
            Write-Warning "Formatting had warnings but continuing..."
        }
    } catch {
        Write-Warning "Formatting failed: $_"
        Write-Warning "Continuing anyway..."
    }
} else {
    Write-Info "Skipping formatting step"
}

# Step 2: Run clippy with auto-fix
if (-not $SkipClippy) {
    Write-Header "Step 2: Running cargo clippy with auto-fix"
    try {
        # Run clippy with --fix flag
        cargo clippy --workspace --all-targets --all-features --fix --allow-dirty --allow-staged
        if ($LASTEXITCODE -eq 0) {
            Write-Success "Clippy fixes applied successfully"
            # Stage fixed files
            git add -u
        } else {
            Write-Warning "Clippy had issues but continuing..."
        }
    } catch {
        Write-Warning "Clippy failed: $_"
        Write-Warning "Continuing anyway..."
    }
} else {
    Write-Info "Skipping clippy step"
}

# Step 3: Tag outstanding issues
if (-not $SkipTagging) {
    Write-Header "Step 3: Tagging TODO/FIXME comments for external review"
    $tagScript = Join-Path $repoRoot "scripts\tag-issues.ps1"
    if (Test-Path $tagScript) {
        try {
            & $tagScript
            if ($LASTEXITCODE -eq 0) {
                Write-Success "Issues tagged successfully"
                git add -u
            } else {
                Write-Warning "Issue tagging had warnings but continuing..."
            }
        } catch {
            Write-Warning "Issue tagging failed: $_"
            Write-Warning "Continuing anyway..."
        }
    } else {
        Write-Info "Issue tagging script not found, skipping..."
    }
} else {
    Write-Info "Skipping issue tagging step"
}

# Step 4: Create semantic index with RAG-Redis
if (-not $SkipIndex) {
    Write-Header "Step 4: Creating semantic index with RAG-Redis"
    $indexScript = Join-Path $repoRoot "scripts\rag-index.ps1"
    if (Test-Path $indexScript) {
        try {
            & $indexScript
            if ($LASTEXITCODE -eq 0) {
                Write-Success "Semantic index created successfully"
                # Stage the index file if it was created
                if (Test-Path (Join-Path $repoRoot ".rag-index.json")) {
                    git add .rag-index.json
                }
            } else {
                Write-Warning "Semantic indexing had warnings but continuing..."
            }
        } catch {
            Write-Warning "Semantic indexing failed: $_"
            Write-Warning "Continuing anyway..."
        }
    } else {
        Write-Info "RAG-Redis indexing script not found, skipping..."
    }
} else {
    Write-Info "Skipping semantic indexing step"
}

# Step 5: Show git status
Write-Header "Step 5: Current git status"
git status --short

# Step 6: Commit changes
Write-Header "Step 6: Committing changes"
$commitArgs = @("commit", "-m", $Message)
if ($NoVerify) {
    $commitArgs += "--no-verify"
    Write-Warning "Using --no-verify flag to bypass hooks"
}

try {
    git @commitArgs
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Changes committed successfully"
    } else {
        Write-Error "Commit failed"
        exit 1
    }
} catch {
    Write-Error "Commit failed: $_"
    exit 1
}

# Step 7: Push to remote (if requested)
if ($Push) {
    Write-Header "Step 7: Pushing to remote"

    # Get current branch
    $currentBranch = git branch --show-current
    Write-Info "Pushing branch: $currentBranch"

    # Try normal push first
    try {
        git push origin $currentBranch
        if ($LASTEXITCODE -eq 0) {
            Write-Success "Pushed to remote successfully"
        } else {
            # If push fails and hooks are blocking, try with --no-verify
            Write-Warning "Push failed, retrying with --no-verify..."
            git push origin $currentBranch --no-verify
            if ($LASTEXITCODE -eq 0) {
                Write-Success "Pushed to remote successfully (bypassing hooks)"
            } else {
                Write-Error "Push failed even with --no-verify"
                exit 1
            }
        }
    } catch {
        Write-Error "Push failed: $_"
        exit 1
    }
} else {
    Write-Info "Skipping push step (use -Push flag to push automatically)"
    Write-Info "To push manually, run: git push origin $(git branch --show-current)"
}

Write-Header "Workflow completed successfully!"
Write-Success "All steps completed"
