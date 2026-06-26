#!/usr/bin/env bash
# Check StepState enum variants match VALID_TRANSITIONS matrix.
#
# Bead: vb-o4zx
# Purpose: CI drift check — fails if enum variants are missing from
#          the transition matrix, or if terminal/non-terminal partitions
#          are inconsistent with VALID_TRANSITIONS.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/crates/vb_core/src/proof_kernels/step_state.rs"

if [[ ! -f "$SRC" ]]; then
  echo "ERROR: $SRC not found" >&2
  exit 1
fi

# Extract enum variant names from "pub enum StepState {" block
VARIANTS=$(sed -n '/pub enum StepState/,/^}/p' "$SRC" | \
  grep -E '^[[:space:]]*[A-Z][a-zA-Z0-9]*' | \
  grep -v 'enum StepState' | \
  sed 's/^[[:space:]]*//' | \
  sed 's/,$//' | \
  sort -u)

VARIANT_COUNT=$(echo "$VARIANTS" | wc -l)
echo "Found $VARIANT_COUNT enum variants: $(echo "$VARIANTS" | tr '\n' ' ')"

# Extract state references from VALID_TRANSITIONS
TRANSITION_STATES=$(sed -n '/const VALID_TRANSITIONS/,/^\];/p' "$SRC" | \
  grep -oE 'StepState::[A-Za-z_]+' | \
  sed 's/StepState:://' | \
  sort -u)

TRANSITION_STATE_COUNT=$(echo "$TRANSITION_STATES" | wc -l)
echo "Found $TRANSITION_STATE_COUNT unique states in VALID_TRANSITIONS: $(echo "$TRANSITION_STATES" | tr '\n' ' ')"

# Check: every enum variant appears in VALID_TRANSITIONS
MISSING=""
for variant in $VARIANTS; do
  if ! echo "$TRANSITION_STATES" | grep -qx "$variant"; then
    MISSING="$MISSING $variant"
  fi
done

if [[ -n "$MISSING" ]]; then
  echo "FAIL: Enum variants missing from VALID_TRANSITIONS:$MISSING" >&2
  exit 1
fi

# Check: every state in VALID_TRANSITIONS is a real enum variant
PHANTOM=""
for state in $TRANSITION_STATES; do
  if ! echo "$VARIANTS" | grep -qx "$state"; then
    PHANTOM="$PHANTOM $state"
  fi
done

if [[ -n "$PHANTOM" ]]; then
  echo "FAIL: Phantom state references:$PHANTOM" >&2
  exit 1
fi

# Extract function body for a given function name using line numbers
extract_fn_states() {
  local fn_name="$1"
  local start_line=$(grep -n "^pub fn ${fn_name}" "$SRC" | head -1 | cut -d: -f1)
  if [[ -z "$start_line" ]]; then
    start_line=$(grep -n "^fn ${fn_name}" "$SRC" | head -1 | cut -d: -f1)
  fi
  # If not a top-level fn, look for it inside impl block
  if [[ -z "$start_line" ]]; then
    start_line=$(grep -n "fn ${fn_name}" "$SRC" | head -1 | cut -d: -f1)
  fi
  
  if [[ -z "$start_line" ]]; then
    return
  fi
  
  # Extract from function start to next function signature or closing brace at column 0
  sed -n "${start_line},/^}/p" "$SRC" | \
    grep -oE 'StepState::[A-Za-z_]+' | \
    sed 's/StepState:://' | \
    sort -u
}

TERMINAL_FROM_MATCHES=$(extract_fn_states "is_terminal" | sort -u)
TERMINAL_FROM_FUNC=$(extract_fn_states "terminal_states" | sort -u)

if [[ "$TERMINAL_FROM_MATCHES" != "$TERMINAL_FROM_FUNC" ]]; then
  echo "FAIL: is_terminal() and terminal_states() inconsistent:" >&2
  echo "  is_terminal() matches:    $(echo "$TERMINAL_FROM_MATCHES" | tr '\n' ' ')" >&2
  echo "  terminal_states() lists:  $(echo "$TERMINAL_FROM_FUNC" | tr '\n' ' ')" >&2
  exit 1
fi

NON_TERMINAL_FROM_MATCHES=$(echo "$VARIANTS" | while read -r v; do
  if ! echo "$TERMINAL_FROM_MATCHES" | grep -qx "$v"; then
    echo "$v"
  fi
done | sort -u)

NON_TERMINAL_FROM_FUNC=$(extract_fn_states "non_terminal_states" | sort -u)

if [[ "$NON_TERMINAL_FROM_MATCHES" != "$NON_TERMINAL_FROM_FUNC" ]]; then
  echo "FAIL: non_terminal_states() inconsistent:" >&2
  echo "  Derived from enum:        $(echo "$NON_TERMINAL_FROM_MATCHES" | tr '\n' ' ')" >&2
  echo "  non_terminal_states() fn: $(echo "$NON_TERMINAL_FROM_FUNC" | tr '\n' ' ')" >&2
  exit 1
fi

# Final: variant count must equal transition state count
if [[ "$VARIANT_COUNT" -ne "$TRANSITION_STATE_COUNT" ]]; then
  echo "FAIL: Variant count ($VARIANT_COUNT) != transition states ($TRANSITION_STATE_COUNT)" >&2
  exit 1
fi

echo "PASS: StepState enum ($VARIANT_COUNT variants) fully covered by VALID_TRANSITIONS matrix."
echo "PASS: is_terminal() and terminal_states() consistent."
echo "PASS: non_terminal_states() consistent."
exit 0
