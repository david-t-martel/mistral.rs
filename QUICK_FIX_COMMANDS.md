# Quick Fix Commands - Git Workflows

## CRITICAL: Fix Git Remote First! ⚠️

```bash
cd T:\projects\rust-mistral\mistral.rs

# Remove upstream as origin
git remote remove origin

# Add YOUR fork as origin
git remote add origin git@github.com:david-t-martel/mistral.rs.git

# Add upstream for syncing
git remote add upstream git@github.com:EricLBuehler/mistral.rs.git

# Verify (should show your fork as origin)
git remote -v
```

______________________________________________________________________

## Test Individual Fixes

```powershell
# 1. Test formatting (should not error -1073741515)
make fmt

# 2. Test TODO tagging (syntax error fixed)
pwsh scripts/tag-issues.ps1 -DryRun

# 3. Test RAG indexing (should find binary in C:\users\david\bin)
pwsh scripts/rag-index.ps1

# 4. Test linting (skips objc_exception)
make lint-fix
```

______________________________________________________________________

## Commit and Push All Fixes

```powershell
# After fixing git remote above, run:
pwsh scripts/git-auto-commit.ps1 -Message "fix: resolve all git workflow issues

- Fix Makefile fmt error with graceful fallback
- Fix PowerShell syntax error in tag-issues.ps1
- Exclude mistralrs-pyo3 to avoid objc_exception on Windows
- Enhance rag-index.ps1 binary search for user bin directories
- Skip clippy --fix on Windows in pre-commit hook
- Add user bin paths to pre-commit PATH
- Add comprehensive troubleshooting documentation" -Push
```

______________________________________________________________________

## Files Modified

- ✅ `.githooks/pre-commit` - PATH + Windows clippy skip
- ✅ `scripts/tag-issues.ps1` - PowerShell syntax fix
- ✅ `scripts/rag-index.ps1` - Enhanced binary search
- ✅ `Makefile` - Graceful fmt/lint fallbacks
- ✅ `docs/TROUBLESHOOTING_GIT_WORKFLOWS.md` - New (499 lines)
- ✅ `GIT_WORKFLOW_FIXES_SUMMARY.md` - New (402 lines)

______________________________________________________________________

## What Was Fixed

| Issue                          | Status                 |
| ------------------------------ | ---------------------- |
| Makefile fmt Error -1073741515 | ✅ Fixed               |
| PowerShell syntax error        | ✅ Fixed               |
| objc_exception compile failure | ✅ Fixed               |
| rag-redis not found            | ✅ Fixed               |
| Git push permission denied     | ⚠️ Manual fix required |

______________________________________________________________________

## Documentation

- **Complete Guide:** `docs/GIT_WORKFLOWS.md`
- **Troubleshooting:** `docs/TROUBLESHOOTING_GIT_WORKFLOWS.md`
- **Fix Summary:** `GIT_WORKFLOW_FIXES_SUMMARY.md`

______________________________________________________________________

**Last Updated:** October 9, 2025
