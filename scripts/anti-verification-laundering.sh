#!/usr/bin/env bash
set -euo pipefail

echo "=========================================================================="
echo "🛡️ ANTI-VERIFICATION LAUNDERING SHIELD ACTIVATED 🛡️"
echo "=========================================================================="
echo "Hunting for AI-generated cheating tactics across the codebase..."

FAIL=0
WARN=0

# 1. KANI CHEAT SCAN
echo "[1/6] Scanning Kani harnesses for shallow bounds and vacuous assumptions..."
KANI_SHALLOW=$(rg -n 'kani::unwind\([0-2]\)' crates/ verification/ || true)
KANI_ASSUME=$(rg -n 'kani::assume\(false\)' crates/ verification/ || true)

if [ -n "$KANI_SHALLOW" ]; then
    echo "⚠️  WARNING: Kani shallow bounds detected; registry proof review owns existing harness triage." >&2
    echo "$KANI_SHALLOW" >&2
    WARN=1
fi

if [ -n "$KANI_ASSUME" ]; then
    echo "⚠️  WARNING: Kani assume(false) paths detected; registry proof review owns existing harness triage." >&2
    echo "$KANI_ASSUME" >&2
    WARN=1
fi

# 2. FLUX CHEAT SCAN
echo "[2/6] Scanning Flux annotations for trust bypasses..."
FLUX_BYPASS=$(rg -n '#\[flux::(trusted|ignore)\]' crates/ verification/ || true)
if [ -n "$FLUX_BYPASS" ]; then
    echo "❌ CRITICAL: Flux verification bypass detected." >&2
    echo "$FLUX_BYPASS" >&2
    echo "Do NOT use #[flux::trusted] or #[flux::ignore] without explicit approval." >&2
    FAIL=1
fi

# 3. LOOM CHEAT SCAN
echo "[3/6] Scanning Loom models for starved state spaces..."
LOOM_STARVED=$(rg -n 'max_branches\([0-1]\)|max_preemptions\([0-1]\)' crates/ verification/ || true)
if [ -n "$LOOM_STARVED" ]; then
    echo "❌ CRITICAL: Loom state-space starvation detected." >&2
    echo "$LOOM_STARVED" >&2
    echo "Do NOT restrict max_branches or max_preemptions to 0 or 1. Let Loom explore." >&2
    FAIL=1
fi

# 4. MIRI CHEAT SCAN
echo "[4/6] Scanning for Miri flag bypasses..."
MIRI_BYPASS=$(rg -n --glob '!scripts/anti-verification-laundering.sh' --glob '!.moon/cache/**' 'Zmiri-disable' .moon/ scripts/ crates/ || true)
if [ -n "$MIRI_BYPASS" ]; then
    echo "❌ CRITICAL: Miri isolation/borrow checking disabled." >&2
    echo "$MIRI_BYPASS" >&2
    echo "You MUST run Miri with full Strict Provenance and Isolation." >&2
    FAIL=1
fi

# 5. TLA+ CHEAT SCAN
echo "[5/6] Scanning TLA+ configs for deadlock dodging..."
TLA_DODGE=$(rg -n --glob '*.cfg' '^[[:space:]]*CHECK_DEADLOCK[[:space:]]+FALSE' verification/tla specs crates/vb_core/src/verification/tla || true)
if [ -n "$TLA_DODGE" ]; then
    echo "❌ CRITICAL: TLA+ Deadlock checking disabled." >&2
    echo "$TLA_DODGE" >&2
    echo "You MUST enable CHECK_DEADLOCK TRUE for your distributed/concurrent models." >&2
    FAIL=1
fi

# 6. TEST SUITE CHEAT SCAN
echo "[6/6] Scanning tests for silent early returns and tautological assertions..."
TEST_TAUT=$(rg -n --glob '*.rs' 'assert!\(true\)|assert_eq!\(true, true\)' crates/ || true)
TEST_EARLY_RETURN=$(rg -n --glob '*.rs' 'return Ok\(\(\)\)' crates/*/tests/ crates/workspace_tests/ || true)

if [ -n "$TEST_TAUT" ]; then
    echo "⚠️  WARNING: Tautological assertions detected; existing test-integrity gate owns full triage." >&2
    WARN=1
fi

if [ -n "$TEST_EARLY_RETURN" ]; then
    echo "⚠️  WARNING: Silent early returns in tests detected; existing test-integrity gate owns full triage." >&2
    WARN=1
fi

# VERUS SCAN (Already in verify-verus.sh, but good to duplicate here for the global audit).
# This shield blocks obviously vacuous proof constructs. Existing Verus
# `external_body` declarations are governed by the registry-driven Verus lane;
# failing this global gate on every trusted boundary would prevent the stricter
# verifier lane from running at all.
VERUS_CHEAT=$(rg -n --glob '*.rs' '(^|[^A-Za-z0-9_])(assume\s*\(\s*(false|true)\s*\)|admit\s*\(|axiom\s+)' verification/verus || true)
if [ -n "$VERUS_CHEAT" ]; then
    echo "❌ CRITICAL: Verus verification laundering detected (vacuous assume/admit/axiom)." >&2
    echo "$VERUS_CHEAT" >&2
    echo "You MUST bind to production code directly." >&2
    FAIL=1
fi

if [ "$FAIL" -eq 1 ]; then
    echo "" >&2
    echo "==========================================================================" >&2
    echo "💥 VERIFICATION LAUNDERING EXPOSED. BUILD TERMINATED." >&2
    echo "==========================================================================" >&2
    exit 1
else
    echo ""
    if [ "$WARN" -eq 1 ]; then
        echo "⚠️  No blocking verification laundering detected. Existing warning-class debt is delegated to registry gates."
    else
        echo "✅ No verification laundering detected. Shield holds."
    fi
    exit 0
fi
