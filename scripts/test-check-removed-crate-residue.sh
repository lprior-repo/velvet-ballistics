#!/usr/bin/env bash
# test-check-removed-crate-residue: self-test for the removed-crate scanner.
# Verifies that:
#   [1/4] the positive fixture passes (exit 0, no active findings),
#   [2/4] the negative fixture fails (exit 1, all removed tokens fire),
#   [3/4] the bare-makepad fixture fails (exit 1, file:line finding), and
#   [4/4] the real repository scan passes (exit 0, no active residue).
#
# Exits 0 on success, exits 1 on any failed assertion.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

GATE="$ROOT/scripts/check-removed-crate-residue.sh"
POSITIVE="$ROOT/fixtures/removed-crate-residue/positive.md"
NEGATIVE="$ROOT/fixtures/removed-crate-residue/negative.md"
NEGATIVE_MAKEPAD="$ROOT/fixtures/removed-crate-residue/negative_makepad.rs"

if [[ ! -x "$GATE" ]]; then
  echo "AssertionFailed: gate script is missing or not executable: $GATE" >&2
  exit 1
fi
for fixture in "$POSITIVE" "$NEGATIVE" "$NEGATIVE_MAKEPAD"; do
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

printf '[1/4] positive fixture must PASS (exit 0, no active findings)\n'
run_gate_capture "$POSITIVE"
assert_exit "positive fixture" "0" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "positive summary" "summary: active=0" "$GATE_OUTPUT"
echo "  ok: exit 0"
echo "  ok: summary reports active=0"

printf '[2/4] negative fixture must FAIL (exit 1, all removed tokens fire)\n'
run_gate_capture "$NEGATIVE"
assert_exit "negative fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "negative file:line" \
  "fixtures/removed-crate-residue/negative.md:" "$GATE_OUTPUT"
for token in vb_codegen vb_ui_model vb_ui_makepad makepad-widgets makepad-draw; do
  assert_output_contains "negative token ${token}" "$token" "$GATE_OUTPUT"
done
assert_output_contains "negative summary" "summary: active=" "$GATE_OUTPUT"
echo "  ok: exit 1 with file:line finding"
echo "  ok: summary reports active > 0"
echo "  ok: every removed-token banner appears"

printf '[3/4] negative makepad fixture must FAIL (exit 1, bare token)\n'
run_gate_capture "$NEGATIVE_MAKEPAD"
assert_exit "negative makepad fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "negative makepad file:line" \
  "fixtures/removed-crate-residue/negative_makepad.rs:" "$GATE_OUTPUT"
assert_output_contains "negative makepad token" "makepad" "$GATE_OUTPUT"
echo "  ok: exit 1 with makepad finding"

printf '[4/4] real repository scan must PASS (exit 0, no active residue)\n'
run_gate_capture
assert_exit "real repository scan" "0" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "real repository summary" "summary: active=0" "$GATE_OUTPUT"
assert_output_omits "real repository active line" " REMOVED-CRATE:" "$GATE_OUTPUT"
echo "  ok: exit 0"
echo "  ok: summary reports active=0"
echo "  ok: no REMOVED-CRATE line in output"

echo "self-test PASSED"
exit 0
