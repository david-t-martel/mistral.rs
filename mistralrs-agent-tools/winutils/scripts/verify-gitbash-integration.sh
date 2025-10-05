#!/bin/bash
# verify-gitbash-integration.sh
# Verifies Git Bash path handling across all utilities

echo -e "\033[36mGit Bash Integration Verification\033[0m"
echo "=================================="

BUILD_DIR="${1:-target/x86_64-pc-windows-msvc/release}"
PASS_COUNT=0
FAIL_COUNT=0
MISSING_COUNT=0

# Test individual utility
test_utility() {
    local util_path="$1"
    local util_name="$2"

    if [ ! -f "$util_path" ]; then
        echo -e "  \033[31m✗\033[0m $util_name - MISSING"
        ((MISSING_COUNT++))
        return 1
    fi

    # Check for winpath symbols
    if strings "$util_path" 2>/dev/null | grep -q "winpath\|normalize_path"; then
        echo -e "  \033[32m✓\033[0m $util_name - winpath integrated"
        ((PASS_COUNT++))
        return 0
    else
        echo -e "  \033[31m✗\033[0m $util_name - no winpath"
        ((FAIL_COUNT++))
        return 1
    fi
}

# Test path formats
test_path_formats() {
    local util="$1"
    echo "Testing $util with different path formats:"

    # Windows path
    echo -n "  Windows (C:\Users): "
    if "$BUILD_DIR/$util" 'C:\Users' >/dev/null 2>&1; then
        echo -e "\033[32mPASS\033[0m"
    else
        echo -e "\033[31mFAIL\033[0m"
    fi

    # Git Bash path
    echo -n "  Git Bash (/c/Users): "
    if "$BUILD_DIR/$util" '/c/Users' >/dev/null 2>&1; then
        echo -e "\033[32mPASS\033[0m"
    else
        echo -e "\033[31mFAIL\033[0m"
    fi

    # Mixed path
    echo -n "  Mixed (C:/Users): "
    if "$BUILD_DIR/$util" 'C:/Users' >/dev/null 2>&1; then
        echo -e "\033[32mPASS\033[0m"
    else
        echo -e "\033[31mFAIL\033[0m"
    fi
}

echo -e "\n\033[33mChecking Core Utilities:\033[0m"
for exe in $BUILD_DIR/uu_*.exe; do
    if [ -f "$exe" ]; then
        name=$(basename "$exe" .exe | sed 's/^uu_//')
        test_utility "$exe" "$name"
    fi
done

echo -e "\n\033[33mChecking Derive Utilities:\033[0m"
for util in where which tree; do
    test_utility "$BUILD_DIR/$util.exe" "$util"
done

echo -e "\n\033[36mSummary:\033[0m"
echo "Total checked: $((PASS_COUNT + FAIL_COUNT + MISSING_COUNT))"
echo -e "  \033[32m✓ Integrated: $PASS_COUNT\033[0m"
echo -e "  \033[31m✗ Failed: $FAIL_COUNT\033[0m"
echo -e "  \033[31m✗ Missing: $MISSING_COUNT\033[0m"

# Test actual path handling if ls is available
if [ -f "$BUILD_DIR/uu_ls.exe" ]; then
    echo -e "\n\033[36mPath Format Tests (using ls):\033[0m"
    test_path_formats "uu_ls.exe"
fi

if [ $FAIL_COUNT -eq 0 ] && [ $MISSING_COUNT -eq 0 ]; then
    echo -e "\n\033[32m✓ All utilities have winpath integration!\033[0m"
    exit 0
else
    echo -e "\n\033[33mRecommendations:\033[0m"
    echo "1. Rebuild with: make clean && make release"
    echo "2. Verify winpath in dependencies"
    echo "3. Run: make verify-winpath-integration"
    exit 1
fi
