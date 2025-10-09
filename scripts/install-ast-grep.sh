#!/usr/bin/env bash
set -euo pipefail

# install-ast-grep.sh
# Installs the ast-grep CLI via cargo-binstall if available, otherwise falls back
# to cargo install. The script is idempotent and reuses existing binaries.

BIN="ast-grep"

if command -v "${BIN}" >/dev/null 2>&1; then
  echo "ast-grep already installed at $(command -v ${BIN})"
  exit 0
fi

echo "Installing ast-grep..."

if command -v cargo-binstall >/dev/null 2>&1; then
  if cargo binstall ast-grep -y --locked; then
    exit 0
  fi
  echo "cargo-binstall failed, falling back to cargo install"
fi

cargo install ast-grep --locked
echo "ast-grep installed successfully."
