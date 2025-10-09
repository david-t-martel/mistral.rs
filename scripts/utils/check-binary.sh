#!/usr/bin/env bash
set -euo pipefail

# check-binary.sh
# POSIX-friendly helper mirroring the PowerShell utility. Verifies that a
# release binary exists, prints metadata, and attempts to execute `--help`.

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN="${PROJECT_ROOT}/target/release/mistralrs-server"
RESULT_JSON="${PROJECT_ROOT}/BINARY_CHECK_RESULTS.json"

echo "=== Binary Dependency Check ==="
echo "Binary: ${BIN}"
echo

if [[ ! -f "${BIN}" ]]; then
  echo "✗ Binary not found!"
  exit 1
fi

SIZE_MB=$(awk "BEGIN {printf \"%.1f\", $(stat -c%s "${BIN}")/1024/1024}")
MODIFIED=$(stat -c %y "${BIN}")
echo "✓ Binary exists"
echo "  Size: ${SIZE_MB} MB"
echo "  Modified: ${MODIFIED}"
echo

echo "Environment Variables:"
echo "  CUDA_PATH: ${CUDA_PATH:-<unset>}"
echo "  CUDNN_PATH: ${CUDNN_PATH:-<unset>}"
echo

echo "Testing with --help flag..."
set +e
HELP_OUTPUT="$("${BIN}" --help 2>&1)"
HELP_EXIT_CODE=$?
set -e

if [[ ${HELP_EXIT_CODE} -eq 0 ]]; then
  echo "✓ Binary runs successfully!"
  echo
  echo "First 20 lines of help output:"
  echo "${HELP_OUTPUT}" | head -n 20
else
  printf "✗ Binary failed with exit code: %d (0x%X)\n" "${HELP_EXIT_CODE}" "${HELP_EXIT_CODE}"
  case ${HELP_EXIT_CODE} in
    3221225781) echo "  ERROR: 0xC0000135 - DLL dependency missing" ;;
    3221225501) echo "  ERROR: 0xC000007B - Architecture mismatch (32/64-bit)" ;;
    *)          echo "  Unknown error code" ;;
  esac
  echo
  echo "Common fixes:"
  echo "  1. Ensure CUDA 12.9 is installed and on PATH"
  echo "  2. Check cuDNN 9.8 libraries are accessible"
  echo "  3. Verify Visual C++ Redistributable is installed"
  echo "  4. Run 'scripts/setup/setup-dev-env.sh' (or .ps1) to configure environment"
  echo
  echo "Error output:"
  echo "${HELP_OUTPUT}"
fi

echo
echo "=== Check Complete ==="

cat <<JSON > "${RESULT_JSON}"
{
  "binary_exists": true,
  "binary_size_mb": ${SIZE_MB},
  "help_exit_code": ${HELP_EXIT_CODE},
  "help_works": $( [[ ${HELP_EXIT_CODE} -eq 0 ]] && echo true || echo false ),
  "cuda_path": "$(printf '%s' "${CUDA_PATH:-}")",
  "cudnn_path": "$(printf '%s' "${CUDNN_PATH:-}")",
  "timestamp": "$(date --iso-8601=seconds)"
}
JSON

echo "✓ Results saved to ${RESULT_JSON}"
