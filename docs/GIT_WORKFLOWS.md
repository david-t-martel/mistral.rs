# Enhanced Git Workflows and CI/CD Documentation

**Last Updated:** October 9, 2025
**Status:** ✅ Complete

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Git Workflow Scripts](#git-workflow-scripts)
- [Makefile Targets](#makefile-targets)
- [GitHub CI/CD Workflows](#github-cicd-workflows)
- [Pre-Commit Hooks](#pre-commit-hooks)
- [RAG-Redis Integration](#rag-redis-integration)
- [Troubleshooting](#troubleshooting)

## Overview

This project includes comprehensive automation for code quality, formatting, linting, and deployment. All workflows are designed to maintain high code quality standards while minimizing manual intervention.

### Key Features

- ✅ **Auto-formatting**: Automatic code formatting with `cargo fmt`
- ✅ **Auto-fixing**: Automatic lint fixes with `cargo clippy --fix`
- ✅ **Issue Tagging**: Automatic tagging of TODO/FIXME with @codex/@gemini for external review
- ✅ **Semantic Indexing**: RAG-Redis integration for semantic code search
- ✅ **CI/CD Automation**: Comprehensive GitHub Actions workflows
- ✅ **Pre-commit Hooks**: Quality checks before every commit

## Quick Start

### Basic Workflow

```powershell
# 1. Make your changes
# ... edit files ...

# 2. Run the enhanced git workflow (formats, fixes, tags, commits, and pushes)
pwsh scripts/git-auto-commit.ps1 -Message "feat: add new feature" -Push

# That's it! The script handles:
# - cargo fmt (formatting)
# - cargo clippy --fix (auto-fixes)
# - TODO/FIXME tagging (@codex/@gemini)
# - RAG-Redis semantic indexing
# - git commit
# - git push
```

### Using Makefile Targets

```bash
# Format code
make fmt

# Auto-fix lint issues
make lint-fix

# Tag TODO/FIXME comments
make tag-issues

# Full workflow (format + fix + tag + index)
make workflow-full

# Then commit manually
git commit -m "your message"
git push
```

## Git Workflow Scripts

### 1. Enhanced Git Auto-Commit Script

**Location:** `scripts/git-auto-commit.ps1`

**Purpose:** Comprehensive automation of the entire git workflow from formatting to pushing.

**Usage:**

```powershell
# Basic usage - commit and push
pwsh scripts/git-auto-commit.ps1 -Message "feat: new feature" -Push

# Commit only (no push)
pwsh scripts/git-auto-commit.ps1 -Message "fix: bug fix"

# Skip certain steps
pwsh scripts/git-auto-commit.ps1 -Message "chore: update" -SkipFormat -SkipTagging

# Bypass pre-commit hooks (use when hooks fail)
pwsh scripts/git-auto-commit.ps1 -Message "hotfix: critical" -NoVerify -Push
```

**Parameters:**

- `-Message` (required): Commit message
- `-Push`: Automatically push after commit
- `-NoVerify`: Skip pre-commit hooks
- `-SkipFormat`: Skip cargo fmt step
- `-SkipClippy`: Skip cargo clippy --fix step
- `-SkipTagging`: Skip TODO/FIXME tagging
- `-SkipIndex`: Skip RAG-Redis indexing

**Workflow Steps:**

1. **Format Code**: Runs `cargo fmt` to format all Rust code
1. **Auto-Fix Lints**: Runs `cargo clippy --fix` to automatically fix lint issues
1. **Tag Issues**: Tags TODO/FIXME/XXX/HACK comments with @codex/@gemini
1. **Semantic Index**: Creates RAG-Redis semantic index (if available)
1. **Show Status**: Displays git status with changes
1. **Commit**: Commits all staged changes
1. **Push** (optional): Pushes to remote origin

### 2. Issue Tagging Script

**Location:** `scripts/tag-issues.ps1`

**Purpose:** Automatically tag TODO/FIXME/XXX/HACK comments with @codex or @gemini for external code review.

**Usage:**

```powershell
# Tag all issues
pwsh scripts/tag-issues.ps1

# Dry run (preview without changes)
pwsh scripts/tag-issues.ps1 -DryRun

# Tag additional patterns
pwsh scripts/tag-issues.ps1 -Pattern "URGENT,REVIEW,OPTIMIZE"
```

**Features:**

- Scans all Rust files (`.rs`) in the repository
- Skips `target/`, `.cargo/`, `.git/` directories
- Alternates between @codex and @gemini tags for load balancing
- Only tags comments that don't already have annotations
- Provides detailed summary of changes

**Example Output:**

```
Tagging outstanding issues for external review
Found 142 Rust source files to scan
  [mistralrs-core/src/pipeline/mod.rs:112] Tagged TODO with @codex
  [mistralrs-server/src/main.rs:509] Tagged TODO with @gemini
...
Total tags added: 24
Files modified: 8
```

### 3. RAG-Redis Semantic Indexing Script

**Location:** `scripts/rag-index.ps1`

**Purpose:** Create a semantic index of the codebase using RAG-Redis for enhanced code search and analysis.

**Usage:**

```powershell
# Create semantic index
pwsh scripts/rag-index.ps1

# Specify RAG-Redis binary location
pwsh scripts/rag-index.ps1 -RagRedisPath "C:\tools\rag-redis.exe"

# Custom server URL
pwsh scripts/rag-index.ps1 -ServerUrl "http://localhost:8080"
```

**Features:**

- Indexes Rust source files (`.rs`)
- Indexes documentation (`.md`, `.txt`)
- Indexes configuration files (`Cargo.toml`, `.yml`, `.json`)
- Skips large files (> 1MB)
- Creates `.rag-index.json` in repository root
- Automatically starts/stops RAG-Redis server if needed

**Index File Structure:**

```json
{
  "version": "1.0",
  "timestamp": "2025-10-09T12:00:00Z",
  "repository": "mistral.rs",
  "total_files": 487,
  "files": [
    {
      "path": "mistralrs-core/src/lib.rs",
      "type": ".rs",
      "size": 24567,
      "last_modified": "2025-10-09T10:30:00Z",
      "content_preview": "...",
      "metadata": {
        "line_count": 834,
        "has_todos": true,
        "has_tests": true
      }
    }
  ]
}
```

## Makefile Targets

### Standard Development Targets

```bash
make check              # Quick compilation check
make build              # Build all workspace members
make build-release      # Build release binaries
make test               # Run all tests
make fmt                # Format code
make fmt-check          # Check formatting
make lint               # Run clippy
make lint-fix           # Auto-fix clippy issues
make clean              # Clean build artifacts
```

### Enhanced Git Workflow Targets

```bash
make tag-issues         # Tag TODO/FIXME comments
make tag-issues-dry-run # Preview tagging (no changes)
make rag-index          # Create semantic index
make pre-commit         # Format + lint-fix
make workflow-prepare   # Format + fix + tag
make workflow-full      # Format + fix + tag + index
```

### CI/CD Targets

```bash
make ci                 # Run CI checks (format-check + check + lint + test)
make ci-auto-fix        # CI with auto-fix (formats and fixes issues)
```

### Coverage Targets

```bash
make test-coverage      # Generate HTML coverage report
make test-coverage-open # Generate and open coverage report
make test-coverage-lcov # Generate LCOV report
make test-coverage-ci   # Generate coverage for CI
```

### Help

```bash
make help               # Show all available targets with descriptions
```

## GitHub CI/CD Workflows

### 1. Auto-Format and Fix Workflow

**File:** `.github/workflows/auto-format-fix.yml`

**Triggers:**

- Push to `main`, `master`, `develop`, `chore/*`, `feat/*`, `fix/*` branches
- Pull requests to `main`, `master`, `develop`
- Manual trigger via `workflow_dispatch`
- Daily schedule (2 AM UTC)

**Jobs:**

#### Job 1: Auto-Format and Fix

- Runs `cargo fmt` to format code
- Runs `cargo clippy --fix` to auto-fix lint issues
- Commits and pushes changes automatically
- Creates pull request for protected branches

#### Job 2: Tag Issues

- Tags TODO/FIXME/XXX/HACK comments
- Commits and pushes tag changes
- Runs after auto-format-fix job

**Permissions Required:**

- `contents: write` - To commit and push changes
- `pull-requests: write` - To create pull requests

**Configuration:**

```yaml
# Enable/disable in repository settings
# Settings > Actions > General > Workflow permissions
# ✅ Read and write permissions
```

### 2. Rust CI/CD Pipeline

**File:** `.github/workflows/rust-ci.yml`

**Triggers:**

- Push to `main`, `master`, `develop`
- Pull requests to `main`, `master`, `develop`
- Manual trigger

**Jobs:**

- `quick-check`: Fast compilation check
- `format-check`: Verify formatting
- `lint`: Run clippy linter
- `test`: Full test suite on multiple platforms
- `build-release`: Platform-specific release builds

## Pre-Commit Hooks

### Enhanced Pre-Commit Hook

**Location:** `.githooks/pre-commit`

**Features:**

- Automatically formats code with `cargo fmt`
- Auto-fixes lint issues with `cargo clippy --fix`
- Runs quick compilation check
- Optional TODO/FIXME tagging
- Optional RAG-Redis semantic indexing
- Stages all fixed files automatically

**Installation:**

```powershell
# Install git hooks
pwsh scripts/setup/install-git-hooks.ps1

# Or manually
git config core.hooksPath .githooks
```

**Configuration:**

```bash
# Environment variables (set in your shell profile)

# Skip TODO/FIXME tagging
export SKIP_TAG_ISSUES=1

# Skip RAG-Redis indexing (default: skipped)
export SKIP_RAG_INDEX=0  # Set to 0 to enable
```

**Workflow:**

1. **[1/5] Formatting code**

   - Runs `cargo fmt`
   - Stages formatted files

1. **[2/5] Auto-fixing lint issues**

   - Runs `cargo clippy --fix`
   - Stages fixed files
   - Non-blocking (continues even if some issues remain)

1. **[3/5] Quick compilation check**

   - Runs `cargo check`
   - Blocks commit on compilation errors

1. **[4/5] Tagging TODO/FIXME comments** (optional)

   - Runs `scripts/tag-issues.ps1`
   - Stages tagged files
   - Non-blocking

1. **[5/5] Creating semantic index** (optional, default: skipped)

   - Runs `scripts/rag-index.ps1`
   - Stages `.rag-index.json`
   - Non-blocking

**Bypassing Hooks:**

```bash
# Commit without running hooks
git commit --no-verify -m "message"

# Or use the auto-commit script with -NoVerify
pwsh scripts/git-auto-commit.ps1 -Message "message" -NoVerify -Push
```

## RAG-Redis Integration

### Overview

RAG-Redis (Retrieval-Augmented Generation with Redis) provides semantic search capabilities for the codebase. It creates embeddings of code and documentation, enabling intelligent code search and analysis.

### Prerequisites

```powershell
# Option 1: Install from binary
# Download from: https://github.com/your-org/rag-redis/releases
# Extract and add to PATH

# Option 2: Build from source
git clone https://github.com/your-org/rag-redis
cd rag-redis
cargo build --release
# Binary at: target/release/rag-redis.exe
```

### Usage

```powershell
# Create semantic index
pwsh scripts/rag-index.ps1

# Index is saved to .rag-index.json in repo root
```

### Querying the Index

```powershell
# Start RAG-Redis server
rag-redis server --port 6379

# Query via HTTP API
curl http://localhost:6379/search?q="implement authentication"

# Or use programmatically
# See: docs/RAG_REDIS_API.md
```

### Integration Points

1. **Git Workflow**: Automatically updates index on commit (optional)
1. **Pre-Commit Hook**: Creates index before commit (optional, default: off)
1. **CI/CD**: Generates index in deployment pipeline
1. **IDE Integration**: Use index for enhanced code search

## Troubleshooting

### Common Issues

#### 1. Pre-commit hook fails

**Problem:** Hook exits with error

**Solutions:**

```bash
# Check hook permissions
chmod +x .githooks/pre-commit

# Reinstall hooks
pwsh scripts/setup/install-git-hooks.ps1

# Bypass hooks for this commit
git commit --no-verify -m "message"
```

#### 2. Clippy auto-fix introduces errors

**Problem:** `cargo clippy --fix` creates compilation errors

**Solutions:**

```bash
# Run without auto-fix to see issues
make lint

# Fix manually, then commit
git add -A
git commit -m "fix: resolve clippy issues"

# Or skip clippy in workflow
pwsh scripts/git-auto-commit.ps1 -Message "fix" -SkipClippy
```

#### 3. RAG-Redis not found

**Problem:** `rag-redis` binary not in PATH

**Solutions:**

```powershell
# Install RAG-Redis
# See: Prerequisites section above

# Or skip indexing
pwsh scripts/git-auto-commit.ps1 -Message "feat" -SkipIndex

# Or set path explicitly
pwsh scripts/rag-index.ps1 -RagRedisPath "C:\path\to\rag-redis.exe"
```

#### 4. CI auto-commit fails

**Problem:** GitHub Actions can't push commits

**Solutions:**

```yaml
# Ensure workflow has write permissions
# In .github/workflows/auto-format-fix.yml:
permissions:
  contents: write
  pull-requests: write

# For protected branches, the workflow creates a PR instead
# Merge the auto-generated PR to apply fixes
```

#### 5. Issue tagging conflicts

**Problem:** Multiple developers tag the same issues differently

**Solutions:**

```bash
# Use dry-run to preview
make tag-issues-dry-run

# Pull latest changes before tagging
git pull origin main
make tag-issues

# Coordinate with team on tagging policy
```

### Debug Mode

Enable verbose output for troubleshooting:

```powershell
# Enable PowerShell debug output
$DebugPreference = "Continue"

# Run script
pwsh scripts/git-auto-commit.ps1 -Message "test" -Debug
```

### Getting Help

```bash
# Show all Makefile targets
make help

# Script help
pwsh scripts/git-auto-commit.ps1 -Help
pwsh scripts/tag-issues.ps1 -Help
pwsh scripts/rag-index.ps1 -Help
```

## Best Practices

### 1. Commit Frequency

- Commit early and often
- Use descriptive commit messages
- Follow [Conventional Commits](https://www.conventionalcommits.org/)

```bash
# Good commit messages
feat: add authentication middleware
fix: resolve memory leak in parser
chore: update dependencies
docs: improve README
```

### 2. Pre-Commit Workflow

- Let the pre-commit hook do its job
- Don't bypass hooks unless necessary
- Review auto-fixes before pushing

### 3. CI/CD Integration

- Monitor auto-format-fix workflow runs
- Review and merge auto-generated PRs promptly
- Keep dependencies up to date

### 4. Issue Tagging

- Review tagged issues periodically
- Address @codex/@gemini annotations
- Remove tags when issues are resolved

### 5. RAG-Redis Indexing

- Keep index up to date
- Regenerate after major refactors
- Use index for code review and onboarding

## Advanced Usage

### Custom Workflows

Create custom workflows by combining scripts:

```powershell
# Custom workflow: format, fix, and create PR
pwsh scripts/git-auto-commit.ps1 -Message "refactor: cleanup" -SkipTagging -SkipIndex
# Then create PR manually

# Batch tagging with custom patterns
pwsh scripts/tag-issues.ps1 -Pattern "URGENT,CRITICAL,SECURITY"

# Incremental indexing (future feature)
# pwsh scripts/rag-index.ps1 -Incremental
```

### CI/CD Customization

Modify `.github/workflows/auto-format-fix.yml` to customize behavior:

```yaml
# Change trigger branches
on:
  push:
    branches: [ main, develop, feature/* ]

# Adjust schedule
schedule:
  - cron: '0 0 * * 0'  # Weekly on Sunday

# Modify commit message format
git commit -m "style: $CUSTOM_MESSAGE"
```

### Integration with Other Tools

```bash
# Pre-push validation
git config --local core.hooksPath .githooks
# Create .githooks/pre-push with validation logic

# Integration with VS Code
# Add to .vscode/tasks.json:
{
  "label": "Format and Fix",
  "type": "shell",
  "command": "pwsh scripts/git-auto-commit.ps1 -Message 'auto-fix' -SkipIndex"
}
```

## Changelog

### Version 1.0.0 (2025-10-09)

- ✅ Initial implementation of enhanced git workflows
- ✅ Auto-format and auto-fix scripts
- ✅ Issue tagging system
- ✅ RAG-Redis semantic indexing
- ✅ GitHub CI/CD workflows
- ✅ Enhanced pre-commit hooks
- ✅ Comprehensive Makefile targets
- ✅ Full documentation

______________________________________________________________________

**Questions or Issues?**

- Open a GitHub issue
- Check troubleshooting section above
- Review existing issues and discussions
- Consult the team lead

**Last Updated:** October 9, 2025
