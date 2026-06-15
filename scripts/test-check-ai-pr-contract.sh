#!/usr/bin/env bash
# test-check-ai-pr-contract.sh: self-test for the AI PR contract gate.
# Verifies:
#   [1/5] positive fixture passes (exit 0)
#   [2/5] negative fixture fails (exit 1, missing fields 3, 7, 12)
#   [3/5] rejection triggers fixture catches "unwrap" with --check-rejection-triggers
#   [4/5] clean rejection fixture passes with --check-rejection-triggers (exit 0)
#   [5/5] positive fixture with --skip-rejection-check passes (exit 0)
#
# Exits 0 on success, exits 1 on any failed assertion.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

GATE="$ROOT/scripts/check-ai-pr-contract.sh"
FIXTURES="$ROOT/fixtures/ai-pr-contract"
POSITIVE="$FIXTURES/positive_handoff.md"
NEGATIVE="$FIXTURES/negative_missing_fields.md"
HAS_UNWRAP="$FIXTURES/negative_has_unwrap.md"
CLEAN="$FIXTURES/positive_clean_rejection.md"

if [[ ! -x "$GATE" ]]; then
  echo "AssertionFailed: gate script is missing or not executable: $GATE" >&2
  exit 1
fi

for fixture in "$POSITIVE" "$NEGATIVE" "$HAS_UNWRAP" "$CLEAN"; do
  if [[ ! -f "$fixture" ]]; then
    echo "AssertionFailed: fixture missing: $fixture" >&2
    exit 1
  fi
done

assert_exit() {
  local label="$1"
  local expected="$2"
  local actual="$3"
  local output="$4"
  if [[ "$expected" != "$actual" ]]; then
    printf 'AssertionFailed: %s expected exit %s, got %s\nOutput:\n%s\n' \
      "$label" "$expected" "$actual" "$output" >&2
    exit 1
  fi
}

assert_output_contains() {
  local label="$1"
  local needle="$2"
  local haystack="$3"
  case "$haystack" in
    *"$needle"*) ;;
    *)
      printf 'AssertionFailed: %s missing %s\nOutput:\n%s\n' \
        "$label" "$needle" "$haystack" >&2
      exit 1
    ;;
  esac
}

assert_output_omits() {
  local label="$1"
  local needle="$2"
  local haystack="$3"
  case "$haystack" in
    *"$needle"*)
      printf 'AssertionFailed: %s unexpectedly contained %s\nOutput:\n%s\n' \
        "$label" "$needle" "$haystack" >&2
      exit 1
    ;;
  esac
}

run_gate() {
  set +e
  local output
  output="$(bash "$GATE" "$@" 2>&1)"
  local exit_code=$?
  set -e
  GATE_OUTPUT="$output"
  GATE_EXIT=$exit_code
}

printf '[1/5] Positive fixture must PASS (exit 0)\n'
run_gate "$POSITIVE"
assert_exit "positive fixture" "0" "$GATE_EXIT" "$GATE_OUTPUT"
echo "  ok: exit 0"

printf '[2/5] Negative fixture must FAIL (exit 1, missing fields 3, 7, 12)\n'
run_gate "$NEGATIVE"
assert_exit "negative fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "negative missing Files changed" "MISSING: Files changed" "$GATE_OUTPUT"
assert_output_contains "negative missing Allocation behavior" "MISSING: Allocation behavior" "$GATE_OUTPUT"
assert_output_contains "negative missing Benchmarks added" "MISSING: Benchmarks added" "$GATE_OUTPUT"
assert_output_omits "negative should not mention Phase implemented" "MISSING: Phase implemented" "$GATE_OUTPUT"
echo "  ok: exit 1"
echo "  ok: lists missing Files changed"
echo "  ok: lists missing Allocation behavior"
echo "  ok: lists missing Benchmarks added"
echo "  ok: does not list fields that are present"

printf '[3/5] Has-unwrap fixture with --check-rejection-triggers must FAIL (exit 1)\n'
run_gate --check-rejection-triggers "$HAS_UNWRAP"
assert_exit "has-unwrap fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "has-unwrap unwrap trigger" "REJECTION TRIGGER: unwrap" "$GATE_OUTPUT"
assert_output_contains "has-unwrap unsafe trigger" "REJECTION TRIGGER: unsafe" "$GATE_OUTPUT"
echo "  ok: exit 1"
echo "  ok: catches unwrap trigger"
echo "  ok: catches unsafe trigger"

printf '[4/5] Clean rejection fixture with --check-rejection-triggers must PASS (exit 0)\n'
run_gate --check-rejection-triggers "$CLEAN"
assert_exit "clean rejection fixture" "0" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_omits "clean should not mention REJECTION TRIGGER" "REJECTION TRIGGER" "$GATE_OUTPUT"
echo "  ok: exit 0"
echo "  ok: no rejection triggers reported"

printf '[5/5] Positive fixture with --skip-rejection-check must PASS (exit 0)\n'
run_gate --skip-rejection-check "$POSITIVE"
assert_exit "positive skip-rejection" "0" "$GATE_EXIT" "$GATE_OUTPUT"
echo "  ok: exit 0"

echo ""
echo "self-test PASSED"
exit 0
