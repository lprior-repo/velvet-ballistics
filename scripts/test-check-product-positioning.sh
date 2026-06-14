#!/usr/bin/env bash
# test-check-product-positioning: self-test for the product-positioning
# scanner. Verifies that:
#   [1/3] the positive fixture passes (exit 0, no active findings),
#   [2/3] the negative fixture fails (exit 1, file:line finding),
#   [3/3] the real repository scan passes (exit 0, no active residue).
#
# Exits 0 on success, exits 1 on any failed assertion.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

GATE="$ROOT/scripts/check-product-positioning.sh"
POSITIVE="$ROOT/fixtures/product-positioning/positive.md"
NEGATIVE="$ROOT/fixtures/product-positioning/negative.md"

if [[ ! -x "$GATE" ]]; then
  echo "AssertionFailed: gate script is missing or not executable: $GATE" >&2
  exit 1
fi
for fixture in "$POSITIVE" "$NEGATIVE"; do
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

run_gate_capture() {
  set +e
  local output
  output="$(bash "$GATE" "$@" 2>&1)"
  local exit_code=$?
  set -e
  GATE_OUTPUT="$output"
  GATE_EXIT=$exit_code
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

printf '[1/3] positive fixture must PASS (exit 0, no active findings)\n'
run_gate_capture "$POSITIVE"
assert_exit "positive fixture" "0" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "positive summary" "summary: active=0" "$GATE_OUTPUT"
echo "  ok: exit 0"
echo "  ok: summary reports active=0"

printf '[2/3] negative fixture must FAIL (exit 1, file:line finding)\n'
run_gate_capture "$NEGATIVE"
assert_exit "negative fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "negative file:line" \
  "fixtures/product-positioning/negative.md:" "$GATE_OUTPUT"
assert_output_contains "negative summary" "summary: active=" "$GATE_OUTPUT"
echo "  ok: exit 1 with file:line finding"

printf '[3/3] real repository scan must PASS (exit 0, no active residue)\n'
run_gate_capture
assert_exit "real repository scan" "0" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "real repository summary" "summary: active=0" "$GATE_OUTPUT"
assert_output_omits "real repository active line" " POSITIONING:" "$GATE_OUTPUT"
echo "  ok: exit 0"
echo "  ok: summary reports active=0"
echo "  ok: no POSITIONING line in output"

echo "self-test PASSED"
exit 0
