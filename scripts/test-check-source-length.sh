#!/usr/bin/env bash
# Unit tests for scripts/lib-source-length.sh.
#
# These tests exercise the categorization and limit-lookup functions
# directly. They are written in plain bash so they run on every CI box
# without extra dependencies. The goal is to lock in path-to-category
# matching behavior so a refactor cannot silently mis-classify a file.
#
# Run from the script directory:
#   bash scripts/test-check-source-length.sh
#
# Exit codes:
#   0   all tests passed
#   1   at least one test failed

set -euo pipefail

if [[ "${BASH_VERSINFO[0]}" -ge 3 ]]; then
  shopt -s extglob 2>/dev/null || true
fi

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=lib-source-length.sh
. "$SCRIPT_DIR/lib-source-length.sh"

# TAP-ish output. Each successful assertion increments PASS; each failure
# increments FAIL and emits a diagnostic.
PASS=0
FAIL_CASES=()

assert_eq() {
  local desc="$1"
  local expected="$2"
  local actual="$3"
  if [[ "$expected" == "$actual" ]]; then
    PASS=$((PASS + 1))
    printf 'ok %3d - %s\n' "$PASS" "$desc"
  else
    FAIL_CASES+=("$desc")
    printf 'not ok    - %s\n      expected: %q\n      actual:   %q\n' \
      "$desc" "$expected" "$actual"
  fi
}

assert_match() {
  local desc="$1"
  local expected_pattern="$2"
  local actual="$3"
  if [[ "$actual" =~ $expected_pattern ]]; then
    PASS=$((PASS + 1))
    printf 'ok %3d - %s\n' "$PASS" "$desc"
  else
    FAIL_CASES+=("$desc")
    printf 'not ok    - %s\n      expected pattern: %s\n      actual: %s\n' \
      "$desc" "$expected_pattern" "$actual"
  fi
}

# ---- production categorization ----------------------------------------

assert_eq 'production: crates/vb_core/src/lib.rs' \
  production "$(sl_categorize crates/vb_core/src/lib.rs)"

assert_eq 'production: crates/vb_core/src/foo/bar.rs' \
  production "$(sl_categorize crates/vb_core/src/foo/bar.rs)"

assert_eq 'production: crates/vb_core/src/foo/bar/baz.rs' \
  production "$(sl_categorize crates/vb_core/src/foo/bar/baz.rs)"

assert_eq 'production: xtask/src/main.rs' \
  production "$(sl_categorize xtask/src/main.rs)"

assert_eq 'production: scripts/check-workspace-assertions.rs' \
  production "$(sl_categorize scripts/check-workspace-assertions.rs)"

# ---- test_in_src categorization -----------------------------------------

assert_eq 'test_in_src: tests.rs directly in src' \
  test_in_src "$(sl_categorize crates/vb_compile/src/expr_eval_tests.rs)"

assert_eq 'test_in_src: _tests.rs directly in src' \
  test_in_src "$(sl_categorize crates/vb_runtime/src/handlers_tests.rs)"

assert_eq 'test_in_src: tests.rs under submodule' \
  test_in_src "$(sl_categorize crates/vb_cli/src/commands_journal/tests.rs)"

assert_eq 'test_in_src: _tests.rs under submodule' \
  test_in_src "$(sl_categorize crates/vb_ipc/src/server/impl_tests.rs)"

assert_eq 'test_in_src: tests/*.rs under submodule' \
  test_in_src \
  "$(sl_categorize crates/vb_runtime/src/engine/tests/integration_taint_propagation.rs)"

assert_eq 'test_in_src: nested deep tests.rs' \
  test_in_src \
  "$(sl_categorize crates/vb_storage/src/codec/tests/kill_kind_admission.rs)"

assert_eq 'test_in_src: tests with prefix tests_' \
  test_in_src "$(sl_categorize crates/vb_compile/src/tests_proptest.rs)"

# ---- test_top_level categorization -------------------------------------

assert_eq 'test_top_level: crates/foo/tests/bar.rs' \
  test_top_level "$(sl_categorize crates/foo/tests/bar.rs)"

assert_eq 'test_top_level: crates/foo/tests.rs' \
  test_top_level "$(sl_categorize crates/vb_core/tests/section36.rs)"

assert_eq 'test_top_level: crates/foo/benches/baz.rs' \
  test_top_level "$(sl_categorize crates/foo/benches/baz.rs)"

assert_eq 'test_top_level: top-level benches/foo.rs' \
  test_top_level "$(sl_categorize benches/foo.rs)"

# ---- kani / verus / flux / verification categorization ------------------

assert_eq 'kani: verification/kani/foo.rs' \
  kani "$(sl_categorize verification/kani/foo.rs)"

assert_eq 'kani: contracts/kani/foo.rs' \
  kani "$(sl_categorize contracts/kani/foo.rs)"

assert_eq 'kani: src/**/*kani*.rs' \
  kani "$(sl_categorize crates/vb_runtime/src/verification/kani/kani_foo.rs)"

assert_eq 'verus: verification/verus/foo.rs' \
  verus "$(sl_categorize verification/verus/foo.rs)"

assert_eq 'verus: contracts/verus/foo.rs' \
  verus "$(sl_categorize contracts/verus/foo.rs)"

assert_eq 'verus: src/**/*verus*.rs' \
  verus "$(sl_categorize crates/vb_runtime/src/verification/verus/vb_y9d3v_action_fence.rs)"

assert_eq 'flux: verification/flux/foo.rs' \
  flux "$(sl_categorize verification/flux/foo.rs)"

assert_eq 'flux: contracts/flux/foo.rs' \
  flux "$(sl_categorize contracts/flux/foo.rs)"

assert_eq 'verification: src/verification/foo.rs' \
  verification "$(sl_categorize crates/vb_core/src/verification/foo.rs)"

# ---- generated / perf exclusions ---------------------------------------

assert_eq 'generated: src/generated/foo.rs' \
  generated "$(sl_categorize crates/vb_core/src/generated/foo.rs)"

assert_eq 'perf: src/perf/foo.rs' \
  perf "$(sl_categorize crates/vb_runtime/src/perf/something.rs)"

assert_eq 'excluded is_excluded(generated) returns 0' \
  'yes' "$(sl_is_excluded generated && echo yes || echo no)"

assert_eq 'not-excluded is_excluded(verus) returns empty' \
  '' "$(sl_is_excluded verus && echo yes || echo '')"

# ---- limit and warn_at values -----------------------------------------

assert_eq 'limit production = 300' \
  300 "$(sl_limit production)"
assert_eq 'limit test_in_src = 1500' \
  1500 "$(sl_limit test_in_src)"
assert_eq 'limit test_top_level = 3000' \
  3000 "$(sl_limit test_top_level)"
assert_eq 'limit kani = 800' \
  800 "$(sl_limit kani)"
assert_eq 'limit verus = 800' \
  800 "$(sl_limit verus)"
assert_eq 'limit flux = 800' \
  800 "$(sl_limit flux)"
assert_eq 'limit verification = 600' \
  600 "$(sl_limit verification)"
assert_eq 'limit generated = -1' \
  '-1' "$(sl_limit generated)"
assert_eq 'limit perf = -1' \
  '-1' "$(sl_limit perf)"

assert_eq 'warn_at production = 240' \
  240 "$(sl_warn_at production)"
assert_eq 'warn_at test_in_src = 1200' \
  1200 "$(sl_warn_at test_in_src)"
assert_eq 'warn_at test_top_level = 2400' \
  2400 "$(sl_warn_at test_top_level)"
assert_eq 'warn_at kani = 640' \
  640 "$(sl_warn_at kani)"
assert_eq 'warn_at verus = 640' \
  640 "$(sl_warn_at verus)"
assert_eq 'warn_at flux = 640' \
  640 "$(sl_warn_at flux)"
assert_eq 'warn_at verification = 480' \
  480 "$(sl_warn_at verification)"
assert_eq 'warn_at generated = -1' \
  '-1' "$(sl_warn_at generated)"
assert_eq 'warn_at perf = -1' \
  '-1' "$(sl_warn_at perf)"

# ---- bead id validation ------------------------------------------------

assert_match 'bead id valid: vb-95nyw' \
  '^vb-[a-z0-9]+(\.[a-z0-9]+)*$' 'vb-95nyw'
assert_match 'bead id valid: vb-jpq7.47' \
  '^vb-[a-z0-9]+(\.[a-z0-9]+)*$' 'vb-jpq7.47'

# Negative tests via reverse logic
assert_no_match() {
  local desc="$1"
  local pattern="$2"
  local value="$3"
  if ! [[ "$value" =~ $pattern ]]; then
    PASS=$((PASS + 1))
    printf 'ok %3d - %s\n' "$PASS" "$desc"
  else
    FAIL_CASES+=("$desc")
    printf 'not ok    - %s\n      pattern: %s\n      value:   %s (should NOT match)\n' \
      "$desc" "$pattern" "$value"
  fi
}
assert_no_match 'bead id invalid: vb-' \
  '^vb-[a-z0-9]+(\.[a-z0-9]+)*$' 'vb-'
assert_no_match 'bead id invalid: bead-1' \
  '^vb-[a-z0-9]+(\.[a-z0-9]+)*$' 'bead-1'
assert_no_match 'bead id invalid: TODO' \
  '^vb-[a-z0-9]+(\.[a-z0-9]+)*$' 'TODO'
assert_no_match 'bead id invalid: vb-UPPER' \
  '^vb-[a-z0-9]+(\.[a-z0-9]+)*$' 'vb-UPPER'

# ---- ledger row parsing -----------------------------------------------

assert_match 'ledger row parses with 5 columns' \
  '^[^|]+\|[^|]+\|vb-[^|]+\|[^|]+\|[^|]+$' \
  'crates/vb_core/src/foo.rs|lewis|vb-95nyw|split-after-landing|over 300 lines'

assert_match 'ledger row with category 6th column' \
  '^[^|]+\|[^|]+\|vb-[^|]+\|[^|]+\|[^|]+\|test_in_src$' \
  'crates/vb_core/src/foo/tests.rs|lewis|vb-95nyw|split-after-landing|over 1500|test_in_src'

# ---- summary -----------------------------------------------------------

TOTAL=$((PASS + ${#FAIL_CASES[@]}))

if [[ ${#FAIL_CASES[@]} -eq 0 ]]; then
  printf '\n# pass %d / %d\n' "$PASS" "$TOTAL"
  exit 0
else
  printf '\n# fail %d / %d\n  failing cases:\n' "${#FAIL_CASES[@]}" "$TOTAL"
  for f in "${FAIL_CASES[@]}"; do
    printf '  - %s\n' "$f"
  done
  exit 1
fi
