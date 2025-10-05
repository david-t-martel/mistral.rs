# Tree Utility Testing Guide

## Quick Test Commands

The tree utility has been successfully built and is available at:
`C:\Users\david\.cargo\shared-target\release\tree.exe`

### Basic Usage Tests

1. **Basic directory tree:**

   ```cmd
   tree.exe . -L 2
   ```

1. **With Windows attributes and file sizes:**

   ```cmd
   tree.exe . --size --attrs -L 2
   ```

1. **ASCII-only output (no Unicode):**

   ```cmd
   tree.exe . --ascii -L 2
   ```

1. **JSON output with statistics:**

   ```cmd
   tree.exe . --json -L 1
   ```

1. **Performance summary:**

   ```cmd
   tree.exe . --summary --time -L 3
   ```

### Windows-Specific Features Tests

6. **Show hidden files:**

   ```cmd
   tree.exe C:\Windows\System32 -a -L 1
   ```

1. **Show junction points and symlinks:**

   ```cmd
   tree.exe C:\ --links -L 1
   ```

1. **Pattern matching:**

   ```cmd
   tree.exe . -P "*.rs" -L 2
   ```

1. **File extension filtering:**

   ```cmd
   tree.exe . --ext "rs" -L 2
   ```

1. **Full paths:**

   ```cmd
   tree.exe . --full-path -L 1
   ```

## Test Results

All core functionality has been verified working:

- ✅ Unicode box drawing characters
- ✅ ASCII fallback mode
- ✅ Windows file attributes (Archive, Hidden, System, etc.)
- ✅ File size display in human-readable format
- ✅ JSON output with complete statistics
- ✅ Performance timing and summary
- ✅ Pattern matching and filtering
- ✅ Depth limiting
- ✅ Sort options (alphabetical, time-based)
- ✅ Color output support

## Performance Notes

The utility is optimized for Windows with:

- Multi-threaded directory traversal (use `--threads N` to control)
- Windows API integration for file attributes
- Long path support (>260 characters)
- Smart memory management for large directory trees

## Binary Location

The final optimized binary is located at:
`C:\Users\david\.cargo\shared-target\release\tree.exe` (759 KB)

This can be copied to any location in your PATH for system-wide usage.
