# Context Restoration Guide

## Purpose

This guide helps restore the Windows Coreutils project context for new Claude sessions or after context loss.

## Quick Context Restoration

### Step 1: Load Memory Graph

```
Use mcp__memory__read_graph to load the project knowledge:
- Windows Coreutils Project
- Winutils Architecture
- Build Configuration
- Implementation Status
- Code Patterns
- Agent Coordination
- Future Roadmap
```

### Step 2: Review Context Files

```
Read the following files in order:
1. T:\projects\coreutils\winutils\.claude\context\project_context_20250122.json
2. T:\projects\coreutils\winutils\.claude\context\quick_reference.md
3. T:\projects\coreutils\winutils\FINAL_REPORT.md (for achievements)
```

### Step 3: Key Information to Remember

#### Project Identity

- **Name:** Windows Coreutils (winutils)
- **Location:** T:\\projects\\coreutils\\winutils\\
- **Author:** David Martel <david.martel@auricleinc.com>
- **Goal:** Windows-optimized GNU coreutils implementation

#### Technical Stack

- **Language:** Rust 1.89.0
- **Toolchain:** MSVC
- **Build:** Makefile + Cargo
- **Prefix:** wu- (avoid conflicts)

#### Current State

- 77 functional binaries
- Universal path handling
- Windows optimizations implemented
- Complete documentation

#### Known Issues

- Some binaries in deps/ folder
- External utilities excluded (by design)

### Step 4: Critical Commands

```bash
# Primary build command
make release

# Test everything
make test

# Direct Rust build
cargo build --release --workspace

# Check binary location
dir T:\projects\coreutils\winutils\target\release\*.exe
```

### Step 5: Agent Context

When delegating to agents, provide this context:

**For rust-pro:**

- Project uses dual workspace structure
- Static linking enabled
- 8MB stack size configured
- winpath library for path normalization

**For debugger:**

- 77 binaries to test
- Check both release/ and release/deps/
- Universal path handling must work

**For devops-troubleshooter:**

- Makefile with 40+ targets
- MSVC toolchain required
- Workspace configuration critical

**For docs-architect:**

- Three main docs: BUILD, STATUS, FINAL_REPORT
- Markdown format
- Located in project root

## Context Versioning

### Current Version: 1.0.0 (2025-01-22)

- Initial comprehensive context capture
- All 77 binaries functional
- Documentation complete

### To Create New Version:

1. Update timestamp in filename
1. Increment version number
1. Add changelog entry below

## Changelog

### v1.0.0 - 2025-01-22

- Initial context capture
- 77 functional binaries
- Complete documentation
- Known issues documented
- Future roadmap defined

______________________________________________________________________

## Emergency Recovery

If all context is lost:

1. **Check project exists:**

   ```bash
   cd T:\projects\coreutils\winutils
   dir
   ```

1. **Verify binaries:**

   ```bash
   dir target\release\*.exe | measure
   ```

1. **Read Cargo.toml:**

   ```bash
   type Cargo.toml
   ```

1. **Check documentation:**

   ```bash
   dir *.md
   ```

1. **Load memory graph:**
   Use mcp\_\_memory\_\_search_nodes with query "Windows Coreutils"

## Contact

Author: David Martel <david.martel@auricleinc.com>
Project: Windows Coreutils (winutils)
Location: T:\\projects\\coreutils\\winutils\\
