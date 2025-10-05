#!/usr/bin/env bash
# Install Git hooks for winutils project

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOOK_DIR="$PROJECT_ROOT/.git/hooks"

echo "🔧 Installing Git hooks for winutils..."

# Create hooks directory if needed
mkdir -p "$HOOK_DIR"

# Install pre-commit hook
cat > "$HOOK_DIR/pre-commit" << 'HOOK'
#!/usr/bin/env bash
# WinUtils Pre-commit Quality Gate
set -e

echo "🚀 Running pre-commit quality checks..."
echo ""

# Phase 1: Format Check
echo "📝 Phase 1/4: Checking code formatting..."
if ! cargo fmt --all --check; then
    echo "❌ Code formatting failed!"
    echo "💡 Run 'cargo fmt --all' to fix formatting"
    exit 1
fi
echo "✅ Format check passed"
echo ""

# Phase 2: Clippy Lints
echo "🔍 Phase 2/4: Running Clippy lints..."
if ! cargo clippy --workspace --all-targets -- -D warnings; then
    echo "❌ Clippy found issues!"
    echo "💡 Run 'cargo clippy --workspace --all-targets --fix' to auto-fix"
    exit 1
fi
echo "✅ Clippy checks passed"
echo ""

# Phase 3: Security Audit (warn only)
echo "🔒 Phase 3/4: Running security audit..."
if ! cargo audit 2>/dev/null; then
    echo "⚠️  Security vulnerabilities found (warnings only)"
fi
echo "✅ Security audit complete"
echo ""

# Phase 4: Quick Tests
echo "🧪 Phase 4/4: Running quick tests..."
if ! cargo test --lib --workspace --target x86_64-pc-windows-msvc; then
    echo "❌ Tests failed!"
    echo "💡 Fix failing tests before committing"
    exit 1
fi
echo "✅ Tests passed"
echo ""

echo "🎉 All pre-commit checks passed!"
HOOK

chmod +x "$HOOK_DIR/pre-commit"

echo "✅ Git hooks installed successfully!"
echo ""
echo "📋 Installed hooks:"
echo "  - pre-commit: Format, clippy, audit, tests"
echo ""
echo "💡 To bypass hooks: git commit --no-verify"
