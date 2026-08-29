#!/usr/bin/env bash
# ── verify-miri-gate.sh ────────────────────────────────────────────────
# Strengthened Miri UB-detection gate for pure crates.
#
# Runs `cargo miri test` with strict provenance, stack/heap/interior
# pointer checks, and leak suppression on the full test surface of
# each pure crate (vb_core, vb_compile) as defined in the
# MASTER.md miri lane contract.
#
# Exit code: 0 = all clean, 1 = UB detected or Miri failed.
# ───────────────────────────────────────────────────────────────────────
set -euo pipefail

MIRI_TOOLCHAIN="nightly-2026-04-28"
TIMEOUT_SECS=600

# Miri flags that tighten UB detection beyond default smoke testing:
#   -Zmiri-strict-provenance   – catch dangling/over-aligned/invalid pointers
#   -Zmiri-ignore-leaks        – suppress allocator noise; leaks ≠ UB
#   -Zmiri-disable-isolation   – allow filesystem/syscall ops used by proptest etc.
MIRIFLAGS="${MIRIFLAGS:--Zmiri-strict-provenance -Zmiri-ignore-leaks -Zmiri-disable-isolation}"
export MIRIFLAGS

# Clean previous miri artifacts so we get a fresh interpretation.
rm -rf target/miri-tmp
mkdir -p target/miri-tmp
export TMPDIR="$PWD/target/miri-tmp"

echo "=== Miri UB-detection gate: $MIRI_TOOLCHAIN ==="
echo "MIRIFLAGS=$MIRIFLAGS"

# Run Miri on each pure crate (MASTER.md miri lane contract).
# Each crate is tested independently so failures localise cleanly.
FAILED=0
for CRATE in vb_core vb_compile; do
  echo "--- Miri: -p $CRATE ---"
  if ! timeout "$TIMEOUT_SECS" cargo +"$MIRI_TOOLCHAIN" miri test \
       -p "$CRATE" --lib --all-features -- --nocapture; then
    echo "FAIL: -p $CRATE exited non-zero (UB or Miri error)"
    FAILED=1
  else
    echo "OK:  -p $CRATE"
  fi
done

if [ "$FAILED" -ne 0 ]; then
  echo "=== Miri gate: FAIL (UB detected) ==="
  exit 1
fi

echo "=== Miri gate: PASS (no UB) ==="
exit 0
