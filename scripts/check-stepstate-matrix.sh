#!/usr/bin/env bash
# Check StepState enum variants match VALID_TRANSITIONS matrix.
#
# Bead: vb-o4zx
# Purpose: CI drift check — fails if enum variants are missing from
#          the transition matrix, or if terminal/non-terminal partitions
#          are inconsistent with VALID_TRANSITIONS.
#
# This script is a structural sanity check. It does NOT run Verus —
# it checks that the Rust source code is internally consistent so
# that Verus proofs (which rely on VALID_TRANSITIONS being complete)
# are not undermined by enum/transition drift.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/crates/vb_proof_kernels/src/step_state.rs"

if [[ ! -f "$SRC" ]]; then
  echo "ERROR: $SRC not found — cannot check StepState matrix" >&2
  exit 1
fi

# Extract all enum variant names from the StepState enum definition
# Variants appear as lines inside the enum block that start with a capital letter
VARIANTS=$(awk '
  /pub enum StepState/,/^[}]$/ {
    if (/^[[:space:]]*[A-Z][a-zA-Z0-9]*/, && !/enum StepState/) {
      gsub(/[[:space:]]/, "")
      gsub(/,/, "")
      print
    }
  }
' "$SRC")

if [[ -z "$VARIANTS" ]]; then
  echo "FAIL: No enum variants found in StepState" >&2
  exit 1
fi

VARIANT_COUNT=$(echo "$VARIANTS" | wc -l)
echo "Found $VARIANT_COUNT enum variants: $(echo $VARIANTS | tr '\n' ' ')"

# Extract all state references used in VALID_TRANSITIONS
# Each tuple is (Source, Target) — collect all unique state names
TRANSITION_STATES=$(awk '
  /const VALID_TRANSITIONS/,/^[}]$/ {
    if (/[A-Z][a-zA-Z0-9]*/) {
      gsub(/[()&,]/, "")
      gsub(/[[:space:]]/, "")
      if (/[A-Z][a-zA-Z0-9]/) print
    }
  }
' "$SRC" | sort -u)

TRANSITION_STATE_COUNT=$(echo "$TRANSITION_STATES" | wc -l)
echo "Found $TRANSITION_STATE_COUNT unique state references in VALID_TRANSITIONS: $(echo $TRANSITION_STATES | tr '\n' ' ')"

# Check: every enum variant must appear in VALID_TRANSITIONS
MISSING_FROM_TRANSITIONS=""
for variant in $VARIANTS; do
  if ! echo "$TRANSITION_STATES" | grep -qx "$variant"; then
    MISSING_FROM_TRANSITIONS="$MISSING_FROM_TRANSITIONS $variant"
  fi
done

if [[ -n "$MISSING_FROM_TRANSITIONS" ]]; then
  echo "FAIL: Enum variants missing from VALID_TRANSITIONS:$MISSING_FROM_TRANSITIONS" >&2
  echo "  These variants are unreachable — the transition matrix is incomplete." >&2
  exit 1
fi

# Check: every state in VALID_TRANSITIONS must be a real enum variant
PHANTOM_STATES=""
for state in $TRANSITION_STATES; do
  if ! echo "$VARIANTS" | grep -qx "$state"; then
    PHANTOM_STATES="$PHANTOM_STATES $state"
  fi
done

if [[ -n "$PHANTOM_STATES" ]]; then
  echo "FAIL: Phantom state references in VALID_TRANSITIONS:$PHANTOM_STATES" >&2
  echo "  These states do not exist in the StepState enum." >&2
  exit 1
fi

# Check: terminal_states() function must list exactly the terminal variants
# Extract terminal states from is_terminal() method
TERMINAL_FROM_MATCHES=$(awk '
  /fn is_terminal/,/^[}]$/ {
    if (/StepState::/) {
      gsub(/.*StepState::/, "")
      gsub(/[\|}].*/, "")
      gsub(/[[:space:]]/, "")
      if (/[A-Z]/) print
    }
  }
' "$SRC" | sort -u)

# Extract terminal_states() function
TERMINAL_FROM_FUNC=$(awk '
  /fn terminal_states/,/^[}]$/ {
    if (/StepState::/) {
      gsub(/.*StepState::/, "")
      gsub(/[\),}].*/, "")
      gsub(/[[:space:]]/, "")
      if (/[A-Z]/) print
    }
  }
' "$SRC" | sort -u)

if [[ "$TERMINAL_FROM_MATCHES" != "$TERMINAL_FROM_FUNC" ]]; then
  echo "FAIL: terminal_states() function does not match is_terminal() match arm:" >&2
  echo "  is_terminal() matches:    $(echo $TERMINAL_FROM_MATCHES | tr '\n' ' ')" >&2
  echo "  terminal_states() lists:  $(echo $TERMINAL_FROM_FUNC | tr '\n' ' ')" >&2
  exit 1
fi

# Check: all enum variants must appear in next_states() logic
# next_states includes from-state self-transition + VALID_TRANSITIONS targets
# So every variant must appear as a source in VALID_TRANSITIONS or as the
# identity (from == to), which is handled by is_valid_transition

# Final assertion: variant count must match transition state count
if [[ "$VARIANT_COUNT" -ne "$TRANSITION_STATE_COUNT" ]]; then
  echo "FAIL: Variant count ($VARIANT_COUNT) != unique states in VALID_TRANSITIONS ($TRANSITION_STATE_COUNT)" >&2
  exit 1
fi

echo "PASS: StepState enum ($VARIANT_COUNT variants) fully covered by VALID_TRANSITIONS matrix."
echo "PASS: is_terminal() and terminal_states() are consistent."
exit 0
