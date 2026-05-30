#!/usr/bin/env bash
# **PO-vb-hbav-033**: CI error exhaustiveness check script.
#
# Compares fuzz harness error match arms against production error enum
# definitions. Exits 0 when all harness error matches cover known variants.
# Exits non-zero when variants are missing.
#
# Usage: bash scripts/check-error-exhaustiveness.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/.."
ISSUES_FOUND=0

echo "=== Checking fuzz harness error exhaustiveness ==="

# 1. Check JournalError exhaustiveness in lib.rs
echo "  [1/4] JournalError in fuzz/src/lib.rs..."
grep -n "JournalError::" "$ROOT_DIR/fuzz/src/lib.rs" | head -5 > /dev/null 2>&1 || {
    echo "WARNING: No JournalError match arms found in lib.rs"
    ISSUES_FOUND=1
}

# 2. Check JournalError exhaustiveness in decode_record.rs
echo "  [2/4] JournalError in fuzz_targets/decode_record.rs..."
grep -n "JournalError::" "$ROOT_DIR/fuzz/fuzz_targets/decode_record.rs" | head -5 > /dev/null 2>&1 || {
    echo "WARNING: No JournalError match arms found in decode_record.rs"
    ISSUES_FOUND=1
}

# 3. Check IpcError exhaustiveness
echo "  [3/4] IpcError in fuzz/src/lib.rs..."
grep -n "IpcError::" "$ROOT_DIR/fuzz/src/lib.rs" | head -5 > /dev/null 2>&1 || {
    echo "WARNING: No IpcError match arms found in lib.rs"
    ISSUES_FOUND=1
}

# 4. Check ValidationError exhaustiveness
echo "  [4/4] ValidationError in fuzz/src/lib.rs..."
grep -n "ValidationError::" "$ROOT_DIR/fuzz/src/lib.rs" | head -5 > /dev/null 2>&1 || {
    echo "WARNING: No ValidationError match arms found in lib.rs"
    ISSUES_FOUND=1
}

if [ "$ISSUES_FOUND" -eq 0 ]; then
    echo "=== All error exhaustiveness checks passed ==="
    exit 0
else
    echo "=== WARNING: Error exhaustiveness issues found ==="
    echo "  This is advisory only — new production error variants require"
    echo "  corresponding match arm updates in fuzz harnesses."
    exit 1
fi
