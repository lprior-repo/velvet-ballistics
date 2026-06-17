#!/usr/bin/env bash
# Check StepState enum variants match VALID_TRANSITIONS matrix.
#
# Bead: vb-o4zx
# Purpose: CI drift check — fails if enum variants are missing from
#          the transition matrix, or if terminal/non-terminal partitions
#          are inconsistent with VALID_TRANSITIONS.
#
# Parser notes:
#   The source file declares `pub enum StepState` twice (once inside the
#   `verus!` spec block, once inside the `cargo_kernel` runtime module).
#   A naive `sed /start/,/end/p` range cycles across both occurrences
#   because GNU sed re-enters the range on every start match, which
#   would capture spec-function bodies and other unrelated tokens
#   (e.g. `Err("invalid_state_transition")`, `Ok(to)`).
#
#   This script uses awk with explicit brace-depth tracking so each
#   enum/function/const block is bounded by its own matching delimiter,
#   independent of indentation or later re-declarations.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/crates/vb_proof_kernels/src/step_state.rs"

if [[ ! -f "$SRC" ]]; then
  echo "ERROR: $SRC not found" >&2
  exit 1
fi

# extract_enum_variants <file>
#   Emit bare variant identifiers from every `pub enum StepState { ... }`
#   block, deduped. Brace-depth tracking ensures we never leak into
#   surrounding spec/function bodies.
extract_enum_variants() {
  awk '
    /pub enum StepState/ {
      in_enum = 1
      depth = 0
      n_open = gsub(/\{/, "{")
      n_close = gsub(/\}/, "}")
      depth = n_open - n_close
      next
    }
    in_enum {
      n_open = gsub(/\{/, "{")
      n_close = gsub(/\}/, "}")
      depth += n_open - n_close
      if (depth <= 0) {
        in_enum = 0
        next
      }
      if (match($0, /^[[:space:]]+[A-Z][A-Za-z0-9_]*/)) {
        s = substr($0, RSTART, RLENGTH)
        sub(/^[[:space:]]+/, "", s)
        sub(/,$/, "", s)
        print s
      }
    }
  ' "$1" | sort -u
}

# extract_const_block_lines <file> <anchor-regex>
#   Print the inclusive line range of a `const NAME: ... = &[ ... ];`
#   block: from the anchor line to the first `];` (at any indent) that
#   closes the array literal.
extract_const_block_lines() {
  local file="$1"
  local anchor="$2"
  awk -v anchor="$anchor" '
    $0 ~ anchor { found = 1; print; next }
    found && /^[[:space:]]*\];/ { print; exit }
    found { print }
  ' "$file"
}

# extract_fn_block <file> <fn-name>
#   Print the inclusive line range of the FIRST `pub fn NAME` or
#   `fn NAME` definition, bounded by matching brace depth so nested
#   blocks (if/else/match/for) do not terminate the range early.
extract_fn_block() {
  local file="$1"
  local fn="$2"
  awk -v fn="$fn" '
    BEGIN { depth = 0; started = 0 }
    {
      line = $0
      # detect start: a function-signature line mentioning `fn <name>(`
      # optionally prefixed with `pub ` / `pub open spec ` / etc.
      if (!started && line ~ "(^|[[:space:]])fn[ \t]+" fn "[ \t]*\\(") {
        started = 1
        print
        n_open = gsub(/\{/, "{", line)
        n_close = gsub(/\}/, "}", line)
        depth = n_open - n_close
        if (depth <= 0) { started = 0 }
        next
      }
      if (started) {
        print
        n_open = gsub(/\{/, "{", line)
        n_close = gsub(/\}/, "}", line)
        depth += n_open - n_close
        if (depth <= 0) { started = 0 }
      }
    }
  ' "$file"
}

# extract_stepstate_refs <block-text>
#   From a block of source lines, emit the bare identifier portion of
#   every `StepState::X` reference, deduped.
extract_stepstate_refs() {
  printf '%s\n' "$1" | grep -oE 'StepState::[A-Za-z_]+' | sed 's/StepState:://' | sort -u
}

VARIANTS=$(extract_enum_variants "$SRC")
VARIANT_COUNT=$(printf '%s\n' "$VARIANTS" | grep -c . || true)
echo "Found $VARIANT_COUNT enum variants: $(printf '%s\n' "$VARIANTS" | tr '\n' ' ')"

VT_BLOCK=$(extract_const_block_lines "$SRC" 'const VALID_TRANSITIONS')
TRANSITION_STATES=$(extract_stepstate_refs "$VT_BLOCK")
TRANSITION_STATE_COUNT=$(printf '%s\n' "$TRANSITION_STATES" | grep -c . || true)
echo "Found $TRANSITION_STATE_COUNT unique states in VALID_TRANSITIONS: $(printf '%s\n' "$TRANSITION_STATES" | tr '\n' ' ')"

# Check: every enum variant appears in VALID_TRANSITIONS
MISSING=""
for variant in $VARIANTS; do
  if ! printf '%s\n' "$TRANSITION_STATES" | grep -qx "$variant"; then
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
  if ! printf '%s\n' "$VARIANTS" | grep -qx "$state"; then
    PHANTOM="$PHANTOM $state"
  fi
done

if [[ -n "$PHANTOM" ]]; then
  echo "FAIL: Phantom state references:$PHANTOM" >&2
  exit 1
fi

IS_TERMINAL_BLOCK=$(extract_fn_block "$SRC" "is_terminal")
TERMINAL_FROM_MATCHES=$(extract_stepstate_refs "$IS_TERMINAL_BLOCK")

TERMINAL_STATES_BLOCK=$(extract_fn_block "$SRC" "terminal_states")
TERMINAL_FROM_FUNC=$(extract_stepstate_refs "$TERMINAL_STATES_BLOCK")

if [[ "$TERMINAL_FROM_MATCHES" != "$TERMINAL_FROM_FUNC" ]]; then
  echo "FAIL: is_terminal() and terminal_states() inconsistent:" >&2
  echo "  is_terminal() matches:    $(printf '%s\n' "$TERMINAL_FROM_MATCHES" | tr '\n' ' ')" >&2
  echo "  terminal_states() lists:  $(printf '%s\n' "$TERMINAL_FROM_FUNC" | tr '\n' ' ')" >&2
  exit 1
fi

NON_TERMINAL_FROM_MATCHES=$(printf '%s\n' "$VARIANTS" | while read -r v; do
  if [[ -n "$v" ]] && ! printf '%s\n' "$TERMINAL_FROM_MATCHES" | grep -qx "$v"; then
    echo "$v"
  fi
done | sort -u)

NON_TERMINAL_STATES_BLOCK=$(extract_fn_block "$SRC" "non_terminal_states")
NON_TERMINAL_FROM_FUNC=$(extract_stepstate_refs "$NON_TERMINAL_STATES_BLOCK")

if [[ "$NON_TERMINAL_FROM_MATCHES" != "$NON_TERMINAL_FROM_FUNC" ]]; then
  echo "FAIL: non_terminal_states() inconsistent:" >&2
  echo "  Derived from enum:        $(printf '%s\n' "$NON_TERMINAL_FROM_MATCHES" | tr '\n' ' ')" >&2
  echo "  non_terminal_states() fn: $(printf '%s\n' "$NON_TERMINAL_FROM_FUNC" | tr '\n' ' ')" >&2
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
