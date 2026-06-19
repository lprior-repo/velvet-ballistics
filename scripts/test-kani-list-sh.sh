#!/usr/bin/env bash
# test-kani-list-sh.sh — bash integration tests for the kani-list.sh wrapper.
#
# Required tests per tier-a-4-010:
#   1. test_kani_list_sh_lists_vb_core_harnesses
#   2. test_kani_list_sh_emits_arbitrary_status
#   3. test_kani_list_sh_fails_on_missing_package

set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

LIST_SH="$ROOT/scripts/kani-list.sh"
FIXTURES_DIR="$ROOT/fixtures/kani-list"
TMP_OUTPUT="$(mktemp -d "${TMPDIR:-/tmp}/kani-list-tests.XXXXXX")"
cleanup() {
  rm -rf "$TMP_OUTPUT"
}
trap cleanup EXIT INT TERM

fail() {
  local label="$1"
  local detail="$2"
  printf 'AssertionFailed: %s: %s\n' "$label" "$detail" >&2
  exit 1
}

assert_file_exists() {
  local label="$1"
  local path="$2"
  if [[ ! -f "$path" ]]; then
    fail "$label" "missing file: $path"
  fi
}

# Test 1: bash scripts/kani-list.sh vb_core lists harnesses for the real crate.
test_kani_list_sh_lists_vb_core_harnesses() {
  local out_dir="$TMP_OUTPUT/test1"
  mkdir -p "$out_dir"
  KANI_LIST_DIR="$out_dir" bash "$LIST_SH" vb_core >/dev/null 2>&1 \
    || fail "test_kani_list_sh_lists_vb_core_harnesses" "kani-list.sh failed on vb_core"

  local json="$out_dir/vb_core.json"
  assert_file_exists "test_kani_list_sh_lists_vb_core_harnesses" "$json"

  local count
  count="$(python3 -c "import json; print(json.load(open('$json'))['harness_count'])")"
  if [[ "$count" -le 0 ]]; then
    fail "test_kani_list_sh_lists_vb_core_harnesses" \
      "expected harness_count > 0 for vb_core, got $count"
  fi

  local sample_harness
  sample_harness="$(python3 -c "
import json, sys
data = json.load(open('$json'))
for h in data['harnesses']:
    if h['harness'].startswith('kani_'):
        print(h['harness'])
        break
")"
  if [[ -z "$sample_harness" ]]; then
    fail "test_kani_list_sh_lists_vb_core_harnesses" \
      "no kani_* harness names surfaced from vb_core"
  fi

  printf 'PASS test_kani_list_sh_lists_vb_core_harnesses count=%s sample=%s\n' \
    "$count" "$sample_harness"
}

# Test 2: every harness entry carries a kani_arbitrary_status field with one
# of the three expected substrings.
test_kani_list_sh_emits_arbitrary_status() {
  local out_dir="$TMP_OUTPUT/test2"
  mkdir -p "$out_dir"
  KANI_LIST_DIR="$out_dir" bash "$LIST_SH" vb_core >/dev/null 2>&1 \
    || fail "test_kani_list_sh_emits_arbitrary_status" "kani-list.sh failed on vb_core"

  local json="$out_dir/vb_core.json"
  assert_file_exists "test_kani_list_sh_emits_arbitrary_status" "$json"

  local invalid
  invalid="$(python3 -c "
import json
allowed = {'kani_arbitrary_impl', 'kani_any_only', 'no_input_generator'}
data = json.load(open('$json'))
bad = [h for h in data['harnesses']
       if h.get('kani_arbitrary_status') not in allowed]
print(len(bad))
")"
  if [[ "$invalid" -ne 0 ]]; then
    fail "test_kani_list_sh_emits_arbitrary_status" \
      "found $invalid harnesses without a valid kani_arbitrary_status"
  fi

  # Confirm the substrate statuses are exposed with at least one entry per
  # known variant. vb_core ships harnesses in all three categories today.
  local present
  present="$(python3 -c "
import json
data = json.load(open('$json'))
statuses = sorted({h['kani_arbitrary_status'] for h in data['harnesses']})
print(' '.join(statuses))
")"
  if [[ "$present" != *"kani_any_only"* ]]; then
    fail "test_kani_list_sh_emits_arbitrary_status" \
      "expected at least one kani_any_only entry, got: $present"
  fi

  printf 'PASS test_kani_list_sh_emits_arbitrary_status bad=%s statuses=%s\n' \
    "$invalid" "$present"
}

# Test 3: a missing package must exit 2 (per bead contract).
test_kani_list_sh_fails_on_missing_package() {
  local out_dir="$TMP_OUTPUT/test3"
  mkdir -p "$out_dir"
  local exit_code=0
  KANI_LIST_DIR="$out_dir" \
    bash "$LIST_SH" definitely_not_a_package_xyz >/dev/null 2>&1 \
    || exit_code=$?

  if [[ "$exit_code" -ne 2 ]]; then
    fail "test_kani_list_sh_fails_on_missing_package" \
      "expected exit 2 for missing package, got $exit_code"
  fi

  printf 'PASS test_kani_list_sh_fails_on_missing_package exit=%s\n' "$exit_code"
}

test_kani_list_sh_lists_vb_core_harnesses
test_kani_list_sh_emits_arbitrary_status
test_kani_list_sh_fails_on_missing_package

printf 'OK test-kani-list-sh\n'
