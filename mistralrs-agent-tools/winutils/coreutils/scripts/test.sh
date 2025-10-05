#!/bin/bash
# Basic smoke tests for Windows-optimized coreutils

set -e

echo "🧪 Testing Windows-optimized coreutils..."

RELEASE_DIR="C:/Users/david/.cargo/shared-target/release"

# Check if we have at least some utilities built
BUILT_COUNT=$(find "$RELEASE_DIR" -maxdepth 1 -name "*.exe" | wc -l)
echo "📊 Found $BUILT_COUNT built utilities"

if [ "$BUILT_COUNT" -lt 5 ]; then
    echo "❌ Too few utilities built. Run 'cargo build --release --workspace' first."
    exit 1
fi

# Test basic functionality
echo "📁 Creating test files..."
cd "$(dirname "$0")/.."
echo "Hello, Windows!" > test_file.txt
echo -e "Line 1\nLine 2\nLine 3\nLine 4\nLine 5" > lines.txt

echo "🔧 Testing basic utilities..."

# Test echo
if [ -f "$RELEASE_DIR/echo.exe" ]; then
    OUTPUT=$("$RELEASE_DIR/echo.exe" "Windows coreutils test")
    if [ "$OUTPUT" = "Windows coreutils test" ]; then
        echo "✅ echo: PASS"
    else
        echo "❌ echo: FAIL"
    fi
fi

# Test cat
if [ -f "$RELEASE_DIR/cat.exe" ]; then
    OUTPUT=$("$RELEASE_DIR/cat.exe" test_file.txt)
    if [ "$OUTPUT" = "Hello, Windows!" ]; then
        echo "✅ cat: PASS"
    else
        echo "❌ cat: FAIL"
    fi
fi

# Test head
if [ -f "$RELEASE_DIR/head.exe" ]; then
    OUTPUT=$("$RELEASE_DIR/head.exe" -n 2 lines.txt)
    EXPECTED=$'Line 1\nLine 2'
    if [ "$OUTPUT" = "$EXPECTED" ]; then
        echo "✅ head: PASS"
    else
        echo "❌ head: FAIL"
    fi
fi

# Test pwd
if [ -f "$RELEASE_DIR/pwd.exe" ]; then
    OUTPUT=$("$RELEASE_DIR/pwd.exe")
    if [[ "$OUTPUT" == *"coreutils"* ]]; then
        echo "✅ pwd: PASS"
    else
        echo "❌ pwd: FAIL"
    fi
fi

# Test seq
if [ -f "$RELEASE_DIR/seq.exe" ]; then
    OUTPUT=$("$RELEASE_DIR/seq.exe" 1 3)
    EXPECTED=$'1\n2\n3'
    if [ "$OUTPUT" = "$EXPECTED" ]; then
        echo "✅ seq: PASS"
    else
        echo "❌ seq: FAIL"
    fi
fi

# Test Windows path handling with cat
if [ -f "$RELEASE_DIR/cat.exe" ]; then
    WINDOWS_PATH="T:\\projects\\coreutils\\winutils\\coreutils\\test_file.txt"
    OUTPUT=$("$RELEASE_DIR/cat.exe" "$WINDOWS_PATH")
    if [ "$OUTPUT" = "Hello, Windows!" ]; then
        echo "✅ Windows path handling: PASS"
    else
        echo "❌ Windows path handling: FAIL"
    fi
fi

echo "🧹 Cleaning up test files..."
rm -f test_file.txt lines.txt

echo ""
echo "✅ Basic smoke tests completed!"
echo "💡 To build all utilities: cargo build --release --workspace"
echo "📦 Built utilities are in: $RELEASE_DIR"
