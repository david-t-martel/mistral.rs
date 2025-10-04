# mistral.rs Testing Infrastructure Refactoring - Context Summary

**Status**: COMPLETE (2025-10-03) | **Version**: refactoring-complete-v1.0
**Repository**: https://github.com/david-t-martel/mistral.rs (fork)
**Commit**: d8a0dee02 "refactor(testing): reorganize testing infrastructure with CI/CD integration"

## Quick Reference - Key Files

```powershell
# Master test runner - ALL tests go through here
.\tests\run-all-tests.ps1 -Suite all           # Run everything
.\tests\run-all-tests.ps1 -Suite quick         # Quick validation (<1 min)
.\tests\run-all-tests.ps1 -Suite integration   # Integration tests
.\tests\run-all-tests.ps1 -Suite mcp           # MCP server tests

# Path resolution utility - ALWAYS use this
. "scripts\utils\Get-ProjectPaths.ps1"
$paths = Get-AllProjectPaths
$binary = $paths.Binary  # Never hardcode paths!

# Model finder - Replaces all download scripts
.\scripts\download\hf-model-finder.ps1 -Action list
.\scripts\download\hf-model-finder.ps1 -Action download -ModelPattern "qwen"
```

## Critical Rules

1. **NEVER use bare `cargo`** - ALWAYS use `make`:

   ```bash
   make build          # NOT cargo build
   make test           # NOT cargo test
   make test-ps1       # Run PowerShell tests
   ```

1. **NEVER hardcode paths** - ALWAYS use Get-ProjectPaths.ps1:

   ```powershell
   # WRONG
   $binary = "T:\projects\rust-mistral\mistral.rs\target\release\mistralrs-server.exe"

   # RIGHT
   . "$PSScriptRoot\..\utils\Get-ProjectPaths.ps1"
   $paths = Get-AllProjectPaths
   $binary = $paths.Binary
   ```

1. **NEVER expose tokens** - ALWAYS redact in logs:

   ```powershell
   # Test token exists, don't display
   if (Test-HFToken) {
       Write-Host "HuggingFace token: [REDACTED]" -ForegroundColor Green
   }
   ```

## Project Structure (NEW)

````
mistral.rs/
├── tests/                      # All test files
│   ├── integration/           # Integration tests
│   ├── mcp/                   # MCP server tests
│   ├── results/               # Test output/logs
│   └── run-all-tests.ps1     # Master runner
├── scripts/                   # All automation
│   ├── build/                # Build scripts
│   ├── download/              # Model downloaders
│   ├── launch/               # Model launchers
│   ├── setup/                # Setup/install
│   └── utils/                # Shared utilities
├── docs/                      # Documentation
│   └── testing/              # Testing guides
├── .github/                   # CI/CD
│   └── workflows/            # GitHub Actions
└── .githooks/                # Git hooks

## Completed Work Summary

✅ **76 files reorganized** (20,757+ lines)
✅ **Master test runner** with 4 output formats (text/json/xml/html)
✅ **Unified HF model finder** (replaced 3 scripts)
✅ **3 CI/CD workflows** (Rust, MCP, PowerShell)
✅ **Git hooks** with Makefile integration
✅ **Path resolution utility** (no more hardcoded paths)
✅ **6 documentation guides** (3,000+ lines)

## Known Issues (TO FIX)

1. **Hardcoded paths remain in**:
   - download-gemma2-gguf.ps1
   - download-gemma3.ps1
   - download-more-models.ps1
   - test-phase2-mcp-servers.ps1 (line 375)

2. **MCP server startup** needs timeout (10 seconds)

3. **Git hooks** may auto-stage (should fail instead)

## Next Agent Tasks

**For python-pro**:
- Update remaining scripts to use Get-ProjectPaths.ps1
- Implement MCP server timeout in Start-MCPServers function

**For devops-troubleshooter**:
- Fix git hook behavior (fmt-check vs fmt + git add)
- Optimize CI cache strategy with sccache

**For code-reviewer**:
- Validate fixes for known issues
- Security audit of token handling

## Validation Commands

```powershell
# Quick validation
.\tests\validate-test-runner.ps1

# Run minimal test suite
make test-ps1-quick

# Check for hardcoded paths
rg -i "c:\\users\\david" --type ps1

# Install git hooks
.\scripts\setup\install-git-hooks.ps1
````

## Environment Variables

```bash
# .env file (create from .env.example)
BINARY_PATH=auto           # auto-detect or specify
PROJECT_ROOT=auto           # auto-detect or specify
HF_TOKEN=hf_xxxxx          # HuggingFace token
GITHUB_TOKEN=ghp_xxxxx     # GitHub token
```

## Performance Targets

- Quick suite: < 1 minute
- Integration suite: 8-10 minutes
- MCP suite: 5-7 minutes
- Full test: 15-20 minutes (target: 8-10 with parallelization)

## Agent Coordination Pattern

1. **sequential-thinking**: Plan the approach
1. **python-pro/devops-troubleshooter**: Implement changes
1. **code-reviewer**: Validate implementation
1. **desktop-commander**: File operations
1. **memory**: Context persistence

______________________________________________________________________

**Remember**: This is a COMPLETE refactoring. All major work is DONE. Only minor fixes remain.
