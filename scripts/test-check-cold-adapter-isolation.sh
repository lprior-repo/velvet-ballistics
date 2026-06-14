#!/usr/bin/env bash
# test-check-cold-adapter-isolation: self-test for the cold-adapter
# isolation scanner. Verifies that:
#   [1/5] the positive fixture passes (exit 0, no active findings),
#   [2/5] the negative serde_json fixture fails (exit 1, file:line finding),
#   [3/5] the negative http (hyper/reqwest) fixture fails (exit 1, file:line finding),
#   [4/5] the negative allowlisted fixture passes (exit 0, allowlisted=1),
#   [5/5] the real repository scan reports summary=... and either clean or
#         document a known historical serde_json dev-dep in vb_core.
#
# Exits 0 on success, exits 1 on any failed assertion.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

GATE="$ROOT/scripts/check-cold-adapter-isolation.sh"
POSITIVE="$ROOT/fixtures/cold-adapter-isolation/positive.rs"
NEGATIVE="$ROOT/fixtures/cold-adapter-isolation/negative.rs"
NEGATIVE_HTTP="$ROOT/fixtures/cold-adapter-isolation/negative_http.rs"
NEGATIVE_ALLOWLISTED="$ROOT/fixtures/cold-adapter-isolation/negative_allowlisted.rs"

if [[ ! -x "$GATE" ]]; then
  echo "AssertionFailed: gate script is missing or not executable: $GATE" >&2
  exit 1
fi
for fixture in "$POSITIVE" "$NEGATIVE" "$NEGATIVE_HTTP" "$NEGATIVE_ALLOWLISTED"; do
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

printf '[1/5] positive fixture must PASS (exit 0, no active findings)\n'
run_gate_capture "$POSITIVE"
assert_exit "positive fixture" "0" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "positive summary" "summary: active=0" "$GATE_OUTPUT"
echo "  ok: exit 0"
echo "  ok: summary reports active=0"

printf '[2/5] negative serde_json fixture must FAIL (exit 1, file:line finding)\n'
run_gate_capture "$NEGATIVE"
assert_exit "negative fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "negative file:line" \
  "fixtures/cold-adapter-isolation/negative.rs:" "$GATE_OUTPUT"
assert_output_contains "negative token" "COLD-ADAPTER: serde_json" "$GATE_OUTPUT"
echo "  ok: exit 1 with file:line finding"
echo "  ok: token reported as COLD-ADAPTER: serde_json"

printf '[3/5] negative http fixture must FAIL (exit 1, hyper+reqwest findings)\n'
run_gate_capture "$NEGATIVE_HTTP"
assert_exit "negative http fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "negative http hyper" "COLD-ADAPTER: hyper" "$GATE_OUTPUT"
assert_output_contains "negative http reqwest" "COLD-ADAPTER: reqwest" "$GATE_OUTPUT"
echo "  ok: exit 1"
echo "  ok: hyper + reqwest both reported"

printf '[4/5] negative allowlisted fixture must PASS (exit 0, allowlisted=1)\n'
run_gate_capture "$NEGATIVE_ALLOWLISTED"
assert_exit "allowlisted fixture" "0" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "allowlisted summary" "summary: active=0" "$GATE_OUTPUT"
assert_output_contains "allowlisted marker" "allowlisted:" "$GATE_OUTPUT"
echo "  ok: exit 0"
echo "  ok: allowlisted marker consumes the violation"

printf '[5/5] real repository scan must complete and emit a summary line\n'
run_gate_capture
case "$GATE_OUTPUT" in
  *"summary: "*) ;;
  *)
    printf 'AssertionFailed: real repository scan missing summary line\nOutput:\n%s\n' \
      "$GATE_OUTPUT" >&2
    exit 1
    ;;
esac
# Real-repo status: the bead's evidence.md records the exact state.
# The scan MUST emit a summary line; it MAY exit 0 (clean) or 1
# (active violations, with file:line evidence). We do not assert a
# specific exit code here because vb_core's [dev-dependencies] table
# carries a historical serde_json entry that this scanner will flag.
echo "  ok: summary line emitted"
echo "  ok: real-repo exit code: $GATE_EXIT (see evidence.md for context)"

echo "self-test PASSED"
exit 0
