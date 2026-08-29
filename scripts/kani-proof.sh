#!/usr/bin/env bash
set -euo pipefail

# kani-proof.sh — Resource-controlled Kani proof runner for proof lanes.
#
# This script wraps cargo kani with bounded resource controls:
#   - KANI_DEFAULT_UNWIND: per-harness loop-unwind bound (overrides #[kani::unwind])
#   - KANI_HARNESS_TIMEOUT: per-harness timeout (e.g., 30m, 1800s, 3h)
#   - KANI_JOBS: parallel verification threads (default: 1 for safety)
#   - KANI_MEMORY_LIMIT: per-harness memory cap in MiB (passed via --cbmc-args)
#   - KANI_TIMEOUT: wall-clock timeout for the entire proof run (e.g., 120m)
#   - KANI_FEATURES: comma-separated Cargo features to activate
#   - KANI_CBMC_ARGS: additional space-separated flags forwarded to CBMC
#
# Usage:
#   bash scripts/kani-proof.sh <package> <harness-filter>
#   bash scripts/kani-proof.sh vb_core kani_step_budget_try_take_arbitrary
#   bash scripts/kani-proof.sh vb_core 'kani_idempotency_gates::'
#   KANI_DEFAULT_UNWIND=16 KANI_HARNESS_TIMEOUT=15m bash scripts/kani-proof.sh vb_core my_harness
#
# Evidence is written to .evidence/kani/proofs/<package>/ for reproducibility.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"
EVIDENCE_DIR="${ROOT}/.evidence/kani/proofs"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"

usage() {
  printf 'usage: bash scripts/kani-proof.sh <package> <harness-filter>\n' >&2
  printf '\n' >&2
  printf 'Resource-controlled Kani proof runner.\n' >&2
  printf '\n' >&2
  printf 'Environment variables:\n' >&2
  printf '  KANI_DEFAULT_UNWIND  Loop-unwind bound (overrides #[kani::unwind]). Default: harness-level.\n' >&2
  printf '  KANI_HARNESS_TIMEOUT Per-harness timeout with suffix (s/m/h). Default: no limit.\n' >&2
  printf '  KANI_JOBS            Parallel threads. Default: 1 (safe).\n' >&2
  printf '  KANI_MEMORY_LIMIT    Per-harness CBMC memory cap in MiB (via --cbmc-args). Default: no limit.\n' >&2
  printf '  KANI_TIMEOUT         Wall-clock timeout for entire run (s/m/h). Default: no limit.\n' >&2
  printf '  KANI_FEATURES        Comma-separated Cargo features.\n' >&2
  printf '  KANI_CBMC_ARGS       Additional space-separated flags forwarded to CBMC.\n' >&2
  exit 2
}

if [ "$#" -lt 2 ]; then
  usage
fi

PACKAGE="$1"
HARNESS_FILTER="$2"

# ── Validate dependencies ───────────────────────────────────────────────
if ! cargo kani --version >/dev/null 2>&1; then
  printf 'ERROR: cargo kani is required on PATH.\n' >&2
  exit 1
fi

if ! rustup run nightly-2026-04-28 cargo kani --version >/dev/null 2>&1; then
  printf 'ERROR: nightly-2026-04-28 toolchain is missing or cargo kani is not installed for it.\n' >&2
  exit 1
fi

# ── Resource defaults ───────────────────────────────────────────────────
JOBS="${KANI_JOBS:-1}"
DEFAULT_UNWIND="${KANI_DEFAULT_UNWIND:-}"
HARNESS_TIMEOUT="${KANI_HARNESS_TIMEOUT:-}"
MEMORY_LIMIT="${KANI_MEMORY_LIMIT:-}"
WALL_TIMEOUT="${KANI_TIMEOUT:-}"
FEATURES="${KANI_FEATURES:-}"
CBMC_ARGS="${KANI_CBMC_ARGS:-}"

# ── Build kani command ─────────────────────────────────────────────────
cmd=(rustup run nightly-2026-04-28 cargo kani)

# Package selection
cmd+=("--lib" "-p" "$PACKAGE")

# Features
if [ -n "$FEATURES" ]; then
  cmd+=(--features "$FEATURES")
fi

# Parallelism — always default to 1 for resource safety
cmd+=(-j "$JOBS")

# Default unwind (overrides per-harness #[kani::unwind] annotations)
if [ -n "$DEFAULT_UNWIND" ]; then
  cmd+=(--default-unwind "$DEFAULT_UNWIND")
fi

# Harness filter
cmd+=(--harness "$HARNESS_FILTER")

# Unstable options needed for harness-timeout and cbmc-args
cmd+=(-Z unstable-options)

# Per-harness timeout (experimental)
if [ -n "$HARNESS_TIMEOUT" ]; then
  cmd+=(--harness-timeout "$HARNESS_TIMEOUT")
fi

# Memory limit — forwarded via --cbmc-args to CBMC
if [ -n "$MEMORY_LIMIT" ]; then
  cmd+=(--cbmc-args "--memory-limit" "$MEMORY_LIMIT")
fi

# Extra CBMC flags (caller responsibility for safety)
if [ -n "$CBMC_ARGS" ]; then
  cmd+=(--cbmc-args $CBMC_ARGS)
fi

# Output format
cmd+=(--output-format=regular)

# ── Evidence directory ─────────────────────────────────────────────────
PROOF_DIR="$EVIDENCE_DIR/$PACKAGE/$TIMESTAMP"
LOG_FILE="$PROOF_DIR/kani-proof.log"
mkdir -p "$PROOF_DIR"

# ── Write harness inventory before execution ───────────────────────────
INVENTORY_FILE="$PROOF_DIR/harness-inventory.json"
(
  cd "$ROOT"
  kani_list_args=(--lib "-p" "$PACKAGE")
  if [ -n "$FEATURES" ]; then
    kani_list_args+=(--features "$FEATURES")
  fi
  if [ -n "$DEFAULT_UNWIND" ]; then
    kani_list_args+=(--default-unwind "$DEFAULT_UNWIND")
  fi
  rustup run nightly-2026-04-28 cargo kani "${kani_list_args[@]}" list --format json \
    > "$INVENTORY_FILE" 2>"$PROOF_DIR/harness-inventory-stderr.log"
)

# ── Execute with optional wall-clock timeout ───────────────────────────
printf '[kani-proof] package=%s harness=%s jobs=%s\n' "$PACKAGE" "$HARNESS_FILTER" "$JOBS"
printf '[kani-proof] default_unwind=%s harness_timeout=%s memory_limit=%s wall_timeout=%s\n' \
  "${DEFAULT_UNWIND:-<unset>}" "${HARNESS_TIMEOUT:-<unset>}" "${MEMORY_LIMIT:-<unset>}" "${WALL_TIMEOUT:-<unset>}"
printf '[kani-proof] evidence_dir=%s\n' "$PROOF_DIR"
printf '[kani-proof] command: %s\n\n' "${cmd[*]}"

if [ -n "$WALL_TIMEOUT" ]; then
  # Parse timeout to seconds for timeout command
  timeout_secs=""
  case "$WALL_TIMEOUT" in
    *h) timeout_secs="${WALL_TIMEOUT%h}" ; timeout_secs=$((timeout_secs * 3600)) ;;
    *m) timeout_secs="${WALL_TIMEOUT%m}" ; timeout_secs=$((timeout_secs * 60)) ;;
    *s) timeout_secs="${WALL_TIMEOUT%s}" ;;
    *) timeout_secs="$WALL_TIMEOUT" ;;
  esac
  printf '[kani-proof] wall-clock timeout: %ss\n' "$timeout_secs"
  timeout --signal=TERM "$timeout_secs" "${cmd[@]}" 2>&1 | tee "$LOG_FILE"
  exit_code=${PIPESTATUS[0]}
else
  "${cmd[@]}" 2>&1 | tee "$LOG_FILE"
  exit_code=$?
fi

# ── Post-flight analysis ───────────────────────────────────────────────
printf '\n[kani-proof] --- post-flight ---\n'
printf '[kani-proof] exit_code=%s\n' "$exit_code"
printf '[kani-proof] log=%s\n' "$LOG_FILE"

if [ -f "$INVENTORY_FILE" ]; then
  harness_count=$(python3 -c "
import json, sys
with open('$INVENTORY_FILE') as f:
    data = json.load(f)
total = sum(len(v) for v in data.get('standard-harnesses', {}).values())
total += len(data.get('contract-harnesses', {}).get('contracts', []))
print(total)
" 2>/dev/null || echo "unknown")
  printf '[kani-proof] harnesses_found=%s\n' "$harness_count"
fi

if grep -q 'VERIFICATION:- SUCCESSFUL' "$LOG_FILE" 2>/dev/null; then
  printf '[kani-proof] status=PASS\n'
elif grep -q 'VERIFICATION:- FAILED' "$LOG_FILE" 2>/dev/null; then
  printf '[kani-proof] status=FAIL\n'
else
  printf '[kani-proof] status=UNKNOWN (check log)\n'
fi

exit $exit_code
