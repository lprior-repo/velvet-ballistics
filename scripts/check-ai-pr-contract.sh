#!/usr/bin/env bash
# check-ai-pr-contract.sh
# Validates a handoff/PR description against the 14 required fields from
# MASTER.md Section 43. Optionally checks for automatic rejection triggers.
#
# Usage:
#   bash scripts/check-ai-pr-contract.sh <file>
#   bash scripts/check-ai-pr-contract.sh --check-rejection-triggers <file>
#   bash scripts/check-ai-pr-contract.sh --skip-rejection-check <file>
#
# Exit 0: all required fields present (and no rejection triggers if checked)
# Exit 1: missing fields or rejection triggers detected
# Exit 64: usage error
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"

REQUIRED_FIELDS=(
  "Phase implemented"
  "Beads touched"
  "Files changed"
  "New public functions/types"
  "Error model"
  "Resource bounds"
  "Allocation behavior"
  "Hot-path behavior"
  "Fjall persistence behavior if touched"
  "IPC behavior if touched"
  "Tests added"
  "Benchmarks added"
  "Commands run"
  "Remaining follow-up work filed as beads"
)

usage() {
  echo "Usage: $0 [--check-rejection-triggers|--skip-rejection-check] <file>" >&2
  exit 64
}

FILE=""
CHECK_REJECTION=false
SKIP_REJECTION=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check-rejection-triggers) CHECK_REJECTION=true; shift ;;
    --skip-rejection-check) SKIP_REJECTION=true; shift ;;
    --*) echo "Unknown option: $1" >&2; usage ;;
    *) FILE="$1"; shift ;;
  esac
done

if [[ -z "$FILE" ]]; then
  echo "Error: no file specified" >&2
  usage
fi

if [[ ! -f "$FILE" ]]; then
  echo "Error: file not found: $FILE" >&2
  exit 64
fi

if [[ "$SKIP_REJECTION" == true ]]; then
  CHECK_REJECTION=false
fi

status=0
missing_fields=()

for field in "${REQUIRED_FIELDS[@]}"; do
  if ! rg -q -i -F "$field" "$FILE"; then
    missing_fields+=("$field")
  fi
done

if [[ ${#missing_fields[@]} -gt 0 ]]; then
  status=1
  echo "PR CONTRACT FAILED: missing required fields in: $FILE" >&2
  echo "" >&2
  for f in "${missing_fields[@]}"; do
    echo "  MISSING: $f" >&2
  done
  echo "" >&2
  echo "All 14 required fields must be present. See velvet-ballistics-MASTER.md Section 43." >&2
fi

if [[ "$CHECK_REJECTION" == true ]]; then
  rejection_found=false

  check_trigger() {
    local label="$1"
    local pattern="$2"
    if rg -q -i -F "$pattern" "$FILE"; then
      echo "  REJECTION TRIGGER: $label (matches: '$pattern')" >&2
      rejection_found=true
    fi
  }

  check_trigger "unsafe" "unsafe"
  check_trigger "unwrap" "unwrap"
  check_trigger "expect" "expect"
  check_trigger "panic" "panic"
  check_trigger "todo" "todo"
  check_trigger "unimplemented" "unimplemented"
  check_trigger "dbg" "dbg"
  check_trigger "unchecked indexing/slicing" "unchecked indexing"
  check_trigger "unchecked arithmetic/casts" "unchecked arithmetic"
  check_trigger "ignored Result" "ignored Result"
  check_trigger "unbounded queue/loop/retry/fanout" "unbounded"
  check_trigger "YAML interpreted at runtime" "YAML interpreted at runtime"
  check_trigger "JSON inserted into runtime core" "JSON inserted into runtime core"
  check_trigger "HTTP inserted into runtime core" "HTTP inserted into runtime core"
  check_trigger "HashMap<String, Value> runtime state" "HashMap"
  check_trigger "one task per step" "one task per step"
  check_trigger "no tests for new code" "no tests for new code"
  check_trigger "speed claim without real benchmark" "speed claim"
  check_trigger "new velvet-ballistics spelling" "velvet_ballistics"

  if [[ "$rejection_found" == true ]]; then
    status=1
    echo "" >&2
    echo "PR CONTRACT FAILED: rejection triggers detected. Fix these before landing." >&2
  fi
fi

exit "$status"
