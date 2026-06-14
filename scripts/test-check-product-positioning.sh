#!/usr/bin/env bash
# test-check-product-positioning: self-test for the product-positioning
# scanner. Verifies that:
#   [1/7] the positive fixture passes (exit 0, no active findings),
#   [2/7] the negative fixture exercises every banned phrase category,
#   [3/7] disclaimer-spam bypasses fail with active findings,
#   [4/7] inline hyphen/underscore bypasses fail,
#   [5/7] Unicode lookalikes / zero-width bypasses fail,
#   [6/7] an unclosed disclaimer block fails hard, and
#   [7/7] the real repository scan passes (exit 0, no active residue).
#
# Exits 0 on success, exits 1 on any failed assertion.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

GATE="$ROOT/scripts/check-product-positioning.sh"
POSITIVE="$ROOT/fixtures/product-positioning/positive.md"
NEGATIVE="$ROOT/fixtures/product-positioning/negative.md"

ALL_BANNED_PHRASES=(
  "generic dag runner"
  "low-code graph editor"
  "yaml-as-programming"
  "yaml as programming"
  "airflow replacement"
  "airflow alternative"
  "temporal clone"
  "temporal alternative"
)

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

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/vb-product-positioning-tests.XXXXXX")"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

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

assert_all_banned_phrases() {
  local label="$1"
  local output="$2"
  local phrase
  for phrase in "${ALL_BANNED_PHRASES[@]}"; do
    assert_output_contains "$label phrase" "$phrase" "$output"
  done
}

run_expected_failure() {
  local label="$1"
  local path="$2"
  run_gate_capture "$path"
  assert_exit "$label" "1" "$GATE_EXIT" "$GATE_OUTPUT"
  assert_output_contains "$label summary" "summary: active=" "$GATE_OUTPUT"
  assert_output_contains "$label finding" " POSITIONING:" "$GATE_OUTPUT"
}

printf '[1/7] positive fixture must PASS (exit 0, no active findings)\n'
run_gate_capture "$POSITIVE"
assert_exit "positive fixture" "0" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "positive summary" "summary: active=0" "$GATE_OUTPUT"
echo "  ok: exit 0"
echo "  ok: summary reports active=0"

printf '[2/7] negative fixture must FAIL and exercise every banned phrase\n'
run_gate_capture "$NEGATIVE"
assert_exit "negative fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "negative file:line" \
  "fixtures/product-positioning/negative.md:" "$GATE_OUTPUT"
assert_output_contains "negative summary" "summary: active=" "$GATE_OUTPUT"
assert_all_banned_phrases "negative fixture" "$GATE_OUTPUT"
echo "  ok: exit 1 with file:line findings"
echo "  ok: every banned phrase category appeared"

DISALLOWED_DISCLAIMER="$TMP_DIR/disclaimer-spam.md"
cat >"$DISALLOWED_DISCLAIMER" <<'EOF'
<!-- position-disclaimer -->
velvet-ballistics is a generic DAG runner.
velvet-ballistics is a low-code graph editor.
velvet-ballistics is a yaml-as-programming framework.
velvet-ballistics is a yaml as programming framework.
velvet-ballistics is a airflow replacement.
velvet-ballistics is a airflow alternative.
velvet-ballistics is a temporal clone.
velvet-ballistics is a temporal alternative.
<!-- /position-disclaimer -->
EOF

printf '[3/7] disclaimer-spam bypass must FAIL with active findings\n'
run_expected_failure "disclaimer-spam" "$DISALLOWED_DISCLAIMER"
assert_output_omits "disclaimer-spam disclaimered" "disclaimered:" "$GATE_OUTPUT"
assert_all_banned_phrases "disclaimer-spam" "$GATE_OUTPUT"
echo "  ok: exit 1 with active findings"

INLINE_VARIANTS="$TMP_DIR/inline-variants.md"
cat >"$INLINE_VARIANTS" <<'EOF'
generic_dag_runner
low-code-graph-editor
yaml-as-programming
yaml_as_programming
airflow-replacement
airflow_replacement
airflow-alternative
temporal-clone
temporal_clone
temporal-alternative
EOF

printf '[4/7] inline hyphen/underscore bypass must FAIL\n'
run_expected_failure "inline-variants" "$INLINE_VARIANTS"
assert_all_banned_phrases "inline-variants" "$GATE_OUTPUT"
echo "  ok: exit 1 with active findings"

UNICODE_VARIANTS="$TMP_DIR/unicode-variants.md"
cat >"$UNICODE_VARIANTS" <<'EOF'
ｇｅｎｅｒｉｃ＿ｄａｇ＿ｒｕｎｎｅｒ
ｌｏｗ－ｃｏｄｅ－ｇｒａｐｈ－ｅｄｉｔｏｒ
ｙａｍｌ－ａｓ－ｐｒｏｇｒａｍｍｉｎｇ
ｙａｍｌ＿ａｓ＿ｐｒｏｇｒａｍｍｉｎｇ
ａｉｒｆｌｏｗ＿ｒｅｐｌａｃｅｍｅｎｔ
ａｉｒｆｌｏｗ＿ａｌｔｅｒｎａｔｉｖｅ
ｔｅｍｐｏｒａｌ＿ｃｌｏｎｅ
ｔｅｍｐｏｒａｌ＿ａｌｔｅｒｎａｔｉｖｅ
EOF
printf '%b' 't\u200bemporal clone\n' >>"$UNICODE_VARIANTS"

printf '[5/7] Unicode lookalike bypass must FAIL\n'
run_expected_failure "unicode-variants" "$UNICODE_VARIANTS"
assert_all_banned_phrases "unicode-variants" "$GATE_OUTPUT"
echo "  ok: exit 1 with active findings"

UNCLOSED_DISCLAIMER="$TMP_DIR/unclosed-disclaimer.md"
cat >"$UNCLOSED_DISCLAIMER" <<'EOF'
<!-- position-disclaimer -->
velvet-ballistics is a generic dag runner.
EOF

printf '[6/7] unclosed disclaimer block must FAIL hard\n'
run_gate_capture "$UNCLOSED_DISCLAIMER"
assert_exit "unclosed disclaimer block" "2" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "unclosed disclaimer error" "scan error:" "$GATE_OUTPUT"
assert_output_contains "unclosed disclaimer message" \
  "unclosed position-disclaimer block opened at line" "$GATE_OUTPUT"
echo "  ok: exit 2 scan error"

printf '[7/7] real repository scan must PASS (exit 0, no active residue)\n'
run_gate_capture
assert_exit "real repository scan" "0" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "real repository summary" "summary: active=0" "$GATE_OUTPUT"
assert_output_omits "real repository active line" " POSITIONING:" "$GATE_OUTPUT"
echo "  ok: exit 0"
echo "  ok: summary reports active=0"
echo "  ok: no POSITIONING line in output"

echo "self-test PASSED"
exit 0
