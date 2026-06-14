#!/usr/bin/env bash
# test-check-removed-feature-residue: self-test for the residue scanner.
# Verifies that:
#   [1/3] the positive fixture passes (exit 0, no active findings),
#   [2/3] the negative toml fixture fails (exit 1, file:line finding),
#   [2b/3] the negative profile fixture also fails (exit 1, file:line finding),
#   [3/3] the real repository scan passes (exit 0, no active residue).
#
# Exits 0 on success, exits 1 on any failed assertion.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

GATE="$ROOT/scripts/check-removed-feature-residue.sh"
POSITIVE="$ROOT/fixtures/removed-feature-residue/positive.toml"
NEGATIVE_TOML="$ROOT/fixtures/removed-feature-residue/negative.toml"
NEGATIVE_PROFILE="$ROOT/fixtures/removed-feature-residue/negative_profile.txt"

if [[ ! -x "$GATE" ]]; then
  echo "AssertionFailed: gate script is missing or not executable: $GATE" >&2
  exit 1
fi
for fixture in "$POSITIVE" "$NEGATIVE_TOML" "$NEGATIVE_PROFILE"; do
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

printf '[2/3] negative toml fixture must FAIL (exit 1, file:line finding)\n'
run_gate_capture "$NEGATIVE_TOML"
assert_exit "negative toml fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "negative toml file:line" \
  "fixtures/removed-feature-residue/negative.toml:" "$GATE_OUTPUT"
assert_output_contains "negative toml summary" "summary: active=" "$GATE_OUTPUT"
echo "  ok: exit 1 with file:line finding"
echo "  ok: summary reports active > 0"

printf '[2b/3] negative profile fixture must FAIL (exit 1, file:line finding)\n'
run_gate_capture "$NEGATIVE_PROFILE"
assert_exit "negative profile fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "negative profile file:line" \
  "fixtures/removed-feature-residue/negative_profile.txt:" "$GATE_OUTPUT"
echo "  ok: exit 1 with file:line finding"

printf '[3/3] real repository scan must PASS (exit 0, no active residue)\n'
run_gate_capture
assert_exit "real repository scan" "0" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "real repository summary" "summary: active=0" "$GATE_OUTPUT"
assert_output_omits "real repository active line" " REMOVED-FEATURE:" "$GATE_OUTPUT"
echo "  ok: exit 0"
echo "  ok: summary reports active=0"
echo "  ok: no REMOVED-FEATURE line in output"

echo "self-test PASSED"
exit 0
