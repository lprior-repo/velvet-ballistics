#!/usr/bin/env bash
# test-check-removed-crate-residue: self-test for the removed-crate scanner.
# Verifies that:
#   [1/6] the positive fixture passes (exit 0, no active findings),
#   [2/6] the negative fixture fails (exit 1, all removed tokens fire),
#   [3/6] the bare-makepad fixture fails (exit 1, file:line finding),
#   [4/6] a shell negation probe fails active (exit 1, not allowlisted),
#   [5/6] the real repository scan passes (exit 0, no active residue), and
#   [6/6] a typoed explicit path fails closed (exit 2, no false green).
#
# Exits 0 on success, exits 1 on any failed assertion.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

GATE="$ROOT/scripts/check-removed-crate-residue.sh"
POSITIVE="$ROOT/fixtures/removed-crate-residue/positive.md"
NEGATIVE="$ROOT/fixtures/removed-crate-residue/negative.md"
NEGATIVE_MAKEPAD="$ROOT/fixtures/removed-crate-residue/negative_makepad.rs"
MISSING_EXPLICIT="$ROOT/fixtures/removed-crate-residue/typo-does-not-exist.md"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

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

printf '[1/5] positive fixture must PASS (exit 0, no active findings)\n'
run_gate_capture "$POSITIVE"
assert_exit "positive fixture" "0" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "positive summary" "summary: active=0" "$GATE_OUTPUT"
echo "  ok: exit 0"
echo "  ok: summary reports active=0"

printf '[2/5] negative fixture must FAIL (exit 1, all removed tokens fire)\n'
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

printf '[3/5] negative makepad fixture must FAIL (exit 1, bare token)\n'
run_gate_capture "$NEGATIVE_MAKEPAD"
assert_exit "negative makepad fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "negative makepad file:line" \
  "fixtures/removed-crate-residue/negative_makepad.rs:" "$GATE_OUTPUT"
assert_output_contains "negative makepad token" "makepad" "$GATE_OUTPUT"
echo "  ok: exit 1 with makepad finding"

printf '[4/6] shell negation probe must FAIL (exit 1, no allowlist bypass)\n'
SHELL_BYPASS="$TMPDIR/shell-bypass.sh"
cat > "$SHELL_BYPASS" <<'EOF'
! vb_codegen
EOF
run_gate_capture "$SHELL_BYPASS"
assert_exit "shell negation probe" "1" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "shell negation file:line" "shell-bypass.sh:" "$GATE_OUTPUT"
assert_output_contains "shell negation token" "vb_codegen" "$GATE_OUTPUT"
assert_output_omits "shell negation allowlist banner" " allowlisted: " "$GATE_OUTPUT"
echo "  ok: exit 1 with shell negation finding"
echo "  ok: no allowlisted banner in output"

printf '[5/6] real repository scan must PASS (exit 0, no active residue)\n'
run_gate_capture
assert_exit "real repository scan" "0" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "real repository summary" "summary: active=0" "$GATE_OUTPUT"
assert_output_omits "real repository active line" " REMOVED-CRATE:" "$GATE_OUTPUT"
echo "  ok: exit 0"
echo "  ok: summary reports active=0"
echo "  ok: no REMOVED-CRATE line in output"

printf '[6/6] typoed explicit path must FAIL CLOSED (exit 2, no false green)\n'
run_gate_capture "$MISSING_EXPLICIT"
assert_exit "missing explicit path" "2" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "missing explicit path diagnostic" \
  "explicit target missing" "$GATE_OUTPUT"
echo "  ok: exit 2 for missing explicit path"
echo "  ok: diagnostic names explicit target"

echo "self-test PASSED"
exit 0
