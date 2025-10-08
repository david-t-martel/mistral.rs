# CLAUDE.md Improvements Summary

**Date**: 2025-10-03
**Task**: Enhanced CLAUDE.md with project-specific context and development state

## Changes Made

### 1. Added Version Information

- Current version: 0.6.0
- Rust version requirement: 1.86+

### 2. Enhanced Build Instructions

**Added Windows-specific CUDA build section**:

- Included NVCC_CCBIN environment variable setup (critical for Windows)
- Platform-specific instructions (Windows/Linux/macOS)
- Documented output path: `target\release\mistralrs-server.exe`

**Improved command examples**:

- Separate sections for Windows PowerShell vs bash
- Project-specific launch scripts documented
- Multi-model and MCP integration examples

### 3. Expanded Testing Section

**Added development workflow**:

- Pre-change verification steps
- During-development formatting/linting
- Post-change validation workflow
- New model integration checklist

**Enhanced testing guidance**:

- Reference to MODEL_INVENTORY.json for test models
- Recommended smallest model for testing (Qwen2.5-1.5B, 940MB)
- Added `cargo check` emphasis (always run before committing)

### 4. New MCP Integration Section

**Comprehensive MCP documentation**:

- MCP Client usage (connecting to external tools)
- MCP Server usage (serving mistral.rs via MCP)
- List of available MCP servers from project's MCP_CONFIG.json
- Example configurations

### 5. Expanded Common Pitfalls

**Added critical Windows/project-specific pitfalls**:

- NVCC_CCBIN requirement for Windows CUDA builds
- PyO3 bindings requiring Python 3.x
- Model format command selection (run vs diffusion vs speech)
- MCP stdio protocol explanation

### 6. New Project-Specific Notes Section

**Current environment documentation**:

- Hardware: NVIDIA GeForce RTX 5060 Ti (16GB VRAM)
- CUDA versions available (12.9 primary, others: 12.1, 12.6, 12.8, 13.0)
- cuDNN 9.8
- Platform: Windows 11 with PowerShell
- Build tools: Visual Studio 2022, Rust 1.89.0

**Development state tracking**:

- Binary build status
- Testing phase progress (Phase 1 complete, Phase 2 partial)
- PyO3 bindings status

**Available models list** (from MODEL_INVENTORY.json):

- 5 models with sizes and recommendations
- Fastest model highlighted for testing

**Testing scripts inventory**:

- MCP server validation scripts
- Launch scripts for quick model testing

## Why These Changes Matter

1. **Reduces onboarding time**: New developers (or AI assistants) can immediately understand:

   - The specific Windows/CUDA build requirements
   - Which model to use for testing
   - Current state of development

1. **Prevents common errors**:

   - NVCC_CCBIN not set → build failure
   - Wrong subcommand for model type → runtime failure
   - MCP server stdio confusion → integration issues

1. **Captures institutional knowledge**:

   - Testing progress and scripts available
   - Hardware configuration and limitations
   - Model inventory with specific sizes/formats

1. **Enables better AI assistance**:

   - AI agents can reference exact model paths and sizes
   - Clear workflow for testing changes
   - MCP integration examples for tool-calling features

## What Was Preserved

All original content retained:

- VarBuilder explanation for model integration
- Architecture overview and workspace structure
- Key design patterns
- Adding new features workflow
- Important files to know
- All original pitfalls

## Verification

```bash
# Verified project still compiles after changes
cargo check --package mistralrs-server
# Status: ✓ Compiling successfully
```

## Next Steps for Future Updates

Consider adding when relevant:

1. **Benchmark results** - When performance testing completes
1. **Python API examples** - If PyO3 bindings are built and tested
1. **Multi-model workflows** - When multi-model testing completes
1. **Production deployment** - When moving beyond development
1. **Vision/diffusion examples** - When those model types are tested

## Files Modified

- `CLAUDE.md` - Enhanced with 6 major sections of improvements
- `CLAUDE_MD_IMPROVEMENTS.md` - This summary document

## Testing Status Reference

Based on project documentation:

- Phase 0: ✅ Workspace bootstrap complete
- Phase 1: ✅ Pre-flight verification complete
- Phase 1.1: ✅ MODEL_INVENTORY.json generated
- Phase 2: ⚠️ MCP testing partial (2/9 servers validated)
- Phase 3: 🔄 TUI testing in progress
- Phase 4: 🔄 PyO3 bindings unchecked
- Phase 5: 🔄 Configuration review partial
- Phase 6+: ⏳ Pending

______________________________________________________________________

**Summary**: CLAUDE.md now serves as both a general development guide AND a project-specific reference, capturing the current state, available resources, and known working configurations.
