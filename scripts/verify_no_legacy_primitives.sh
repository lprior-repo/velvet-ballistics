#!/bin/bash
# =============================================================================
# Meta-lint check: STEP_PRIMITIVES and ALLOWED_STEP_FIELDS must NOT contain
# legacy primitive names "parallel" or "aggregate".
#
# This script verifies PO-013 for vb-xi2f.16:
# - STEP_PRIMITIVES must NOT contain "parallel" or "aggregate"
# - ALLOWED_STEP_FIELDS must NOT contain "parallel" or "aggregate"
#
# Usage:
#   bash scripts/verify_no_legacy_primitives.sh
#
# Exit codes:
#   0 - PASS: No legacy primitives found in constants
#   1 - FAIL: Legacy primitives found (bug exists)
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ERRORS=0

echo "=============================================="
echo "Meta-lint: Legacy Primitive Name Check (vb-xi2f.16)"
echo "=============================================="
echo ""

# Check vb_validate/src/schema.rs
echo "Checking vb_validate/src/schema.rs..."
SCHEMA_FILE="$REPO_ROOT/crates/vb_validate/src/schema.rs"

if [ ! -f "$SCHEMA_FILE" ]; then
    echo -e "${RED}ERROR: $SCHEMA_FILE not found${NC}"
    exit 1
fi

# Check STEP_PRIMITIVES constant for "parallel"
if grep -n 'STEP_PRIMITIVES' "$SCHEMA_FILE" > /dev/null 2>&1; then
    # Extract the STEP_PRIMITIVES array and check for legacy names
    if grep -A 20 'const STEP_PRIMITIVES' "$SCHEMA_FILE" | grep -q '"parallel"'; then
        echo -e "${RED}FAIL: STEP_PRIMITIVES in schema.rs contains 'parallel' (should use 'together')${NC}"
        grep -n '"parallel"' "$SCHEMA_FILE" || true
        ERRORS=$((ERRORS + 1))
    else
        echo -e "${GREEN}PASS: STEP_PRIMITIVES in schema.rs does not contain 'parallel'${NC}"
    fi

    if grep -A 20 'const STEP_PRIMITIVES' "$SCHEMA_FILE" | grep -q '"aggregate"'; then
        echo -e "${RED}FAIL: STEP_PRIMITIVES in schema.rs contains 'aggregate' (should use 'reduce')${NC}"
        grep -n '"aggregate"' "$SCHEMA_FILE" || true
        ERRORS=$((ERRORS + 1))
    else
        echo -e "${GREEN}PASS: STEP_PRIMITIVES in schema.rs does not contain 'aggregate'${NC}"
    fi
else
    echo -e "${YELLOW}WARNING: STEP_PRIMITIVES not found in schema.rs${NC}"
fi

# Check ALLOWED_STEP_FIELDS constant for "parallel"
if grep -n 'ALLOWED_STEP_FIELDS' "$SCHEMA_FILE" > /dev/null 2>&1; then
    if grep -A 30 'const ALLOWED_STEP_FIELDS' "$SCHEMA_FILE" | grep -q '"parallel"'; then
        echo -e "${RED}FAIL: ALLOWED_STEP_FIELDS in schema.rs contains 'parallel'${NC}"
        ERRORS=$((ERRORS + 1))
    else
        echo -e "${GREEN}PASS: ALLOWED_STEP_FIELDS in schema.rs does not contain 'parallel'${NC}"
    fi

    if grep -A 30 'const ALLOWED_STEP_FIELDS' "$SCHEMA_FILE" | grep -q '"aggregate"'; then
        echo -e "${RED}FAIL: ALLOWED_STEP_FIELDS in schema.rs contains 'aggregate'${NC}"
        ERRORS=$((ERRORS + 1))
    else
        echo -e "${GREEN}PASS: ALLOWED_STEP_FIELDS in schema.rs does not contain 'aggregate'${NC}"
    fi
fi

echo ""

echo "schema_fields.rs retired; consolidated vocabulary lives in vb_validate/src/schema.rs"

echo ""
echo "=============================================="

if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}All checks PASSED: No legacy primitives found${NC}"
    echo "=============================================="
    exit 0
else
    echo -e "${RED}FAILED: $ERRORS legacy primitive(s) found${NC}"
    echo "=============================================="
    exit 1
fi
