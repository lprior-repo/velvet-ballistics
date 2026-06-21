#!/usr/bin/env bash
# jj-dispatch.sh — round-N review/fix dispatch
#
# Validates that all 5 review artifacts exist for the given round (1-40), then
# reports the total CRITICAL count across the 4 slice reviews. Used as a guard
# between the review step (1) and the bead-filing step (2) in the loop protocol.
#
# Usage: ./test-review/jj-dispatch.sh <round_number>
#   e.g., ./test-review/jj-dispatch.sh 2

set -euo pipefail

ROUND="${1:?usage: $0 <round_number>}"
REPO="/home/lewis/src/velvet-ballistics"
EVIDENCE_DIR="$REPO/.evidence/test-review"

if ! [[ "$ROUND" =~ ^[0-9]+$ ]] || [ "$ROUND" -lt 1 ] || [ "$ROUND" -gt 40 ]; then
  echo "ERROR: round must be 1-40" >&2
  exit 1
fi

echo "=== Round $ROUND dispatch ==="
echo "Evidence dir: $EVIDENCE_DIR"

# Check if all 4 slice reviews exist for this round.
# Round 1 used un-suffixed names; rounds 2-40 use "-review-N".
# Slice ID prefix per round:
#   Round 1 used "slice-N-NAME-review.md" (e.g. slice-1-core-runtime-review.md).
#   Rounds 2-40 use "slice-NAME-review-N.md"  (e.g. slice-core-runtime-review-2.md).
SLICES=("core-runtime" "storage-workspace" "compile-cli-validate-proof" "misc")
for s in "${SLICES[@]}"; do
  if [ "$ROUND" = "1" ]; then
    # Round 1 uses the N-prefixed form; pick the matching one by index.
    case "$s" in
      core-runtime)              N=1 ;;
      storage-workspace)         N=2 ;;
      compile-cli-validate-proof) N=3 ;;
      misc)                       N=4 ;;
    esac
    CANDIDATE="$EVIDENCE_DIR/slice-${N}-${s}-review.md"
  else
    CANDIDATE="$EVIDENCE_DIR/slice-${s}-review-${ROUND}.md"
  fi
  if [ ! -f "$CANDIDATE" ]; then
    echo "MISSING: $CANDIDATE" >&2
    echo "Run the 4 review subagents (prompt-slice-{1..4}.md) first." >&2
    exit 1
  fi
done

# Check master review.
if [ "$ROUND" = "1" ]; then
  MASTER="$REPO/test-suite-review.md"
else
  MASTER="$REPO/test-suite-review-${ROUND}.md"
fi
if [ ! -f "$MASTER" ]; then
  echo "MISSING: $MASTER" >&2
  echo "Synthesize the master review after the 4 slice subagents finish." >&2
  exit 1
fi

echo "All artifacts present. Ready to file fix-test beads."

# Count CRITICAL findings across all 4 slice reviews.
TOTAL_CRITICAL=0
TOTAL_HIGH=0
TOTAL_MEDIUM=0
TOTAL_LOW=0
for s in "${SLICES[@]}"; do
  if [ "$ROUND" = "1" ]; then
    case "$s" in
      core-runtime)              N=1 ;;
      storage-workspace)         N=2 ;;
      compile-cli-validate-proof) N=3 ;;
      misc)                       N=4 ;;
    esac
    F="$EVIDENCE_DIR/slice-${N}-${s}-review.md"
  else
    F="$EVIDENCE_DIR/slice-${s}-review-${ROUND}.md"
  fi
  C=$(rg -c '^\| .* \| CRITICAL \|' "$F" 2>/dev/null || echo 0)
  H=$(rg -c '^\| .* \| HIGH \|'     "$F" 2>/dev/null || echo 0)
  M=$(rg -c '^\| .* \| MEDIUM \|'   "$F" 2>/dev/null || echo 0)
  L=$(rg -c '^\| .* \| LOW \|'      "$F" 2>/dev/null || echo 0)
  TOTAL_CRITICAL=$((TOTAL_CRITICAL + ${C:-0}))
  TOTAL_HIGH=$((TOTAL_HIGH + ${H:-0}))
  TOTAL_MEDIUM=$((TOTAL_MEDIUM + ${M:-0}))
  TOTAL_LOW=$((TOTAL_LOW + ${L:-0}))
  echo "  slice-${s}: CRITICAL=${C:-0} HIGH=${H:-0} MEDIUM=${M:-0} LOW=${L:-0}"
done

echo ""
echo "Round $ROUND totals:"
echo "  CRITICAL: $TOTAL_CRITICAL"
echo "  HIGH:     $TOTAL_HIGH"
echo "  MEDIUM:   $TOTAL_MEDIUM"
echo "  LOW:      $TOTAL_LOW"

# Convergence gate (informational only — does not block).
case "$ROUND" in
  10) echo "Convergence target R10: all CRITICALs CLOSED (currently $TOTAL_CRITICAL new this round)." ;;
  20) echo "Convergence target R20: all HIGHs CLOSED (currently $TOTAL_HIGH new this round)." ;;
  30) echo "Convergence target R30: <10 MEDIUMs (currently $TOTAL_MEDIUM new this round)." ;;
  40) echo "Convergence target R40: APPROVED, observation-only (currently $((TOTAL_CRITICAL + TOTAL_HIGH + TOTAL_MEDIUM + TOTAL_LOW)) total)." ;;
esac

# Bead filing is done by the orchestrator (infrastructure-builder / test-reviewer),
# not by this script. This script only validates artifacts.

echo "=== Round $ROUND dispatch validation complete ==="
