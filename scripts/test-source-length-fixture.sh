#!/usr/bin/env bash
# Integration tests for scripts/check-source-length.sh.
#
# These tests build a fixture repo directory containing synthetic Rust
# source files at known line counts, then run the gate against the
# fixture with GATE_REPO_ROOT pointing at it. They prove the
# categorization, limit lookup, ledger validation, and exit-code paths
# behave correctly together.
#
# Run from the script directory:
#   bash scripts/test-source-length-fixture.sh
#
# Exit codes:
#   0   all fixtures passed
#   1   at least one fixture failed

set -euo pipefail

if [[ "${BASH_VERSINFO[0]}" -ge 3 ]]; then
  shopt -s extglob 2>/dev/null || true
fi

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
GATE="$SCRIPT_DIR/check-source-length.sh"

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

assert_contains() {
  local desc="$1"
  local expected_substr="$2"
  local actual="$3"
  if [[ "$actual" == *"$expected_substr"* ]]; then
    PASS=$((PASS + 1))
    printf 'ok %3d - %s\n' "$PASS" "$desc"
  else
    FAIL_CASES+=("$desc")
    printf 'not ok    - %s\n      expected substr: %q\n      actual:           %q\n' \
      "$desc" "$expected_substr" "$actual"
  fi
}

# Build a synthetic fixture. Args:
#   $1  fixture root
#   $2+ key=value pairs:
#          ledger_row_count=<N>   count of empty rows in the ledger
#          bad_ledger=<bool>      if true, write a malformed ledger
#          files=<spec>           semicolon-separated list of <path>:<lines>
build_fixture() {
  local root="$1"
  shift
  local kv files
  local ledger_rows=0
  local bad_ledger=0
  files=""
  for kv in "$@"; do
    case "$kv" in
      ledger_row_count=*) ledger_rows="${kv#ledger_row_count=}" ;;
      bad_ledger=*)       bad_ledger="${kv#bad_ledger=}" ;;
      files=*)            files="${kv#files=}" ;;
    esac
  done

  rm -rf "$root"
  mkdir -p "$root/.config"
  mkdir -p "$root/crates/vb_x/src/foo"

  # Ledger
  if [[ "$bad_ledger" == "1" ]]; then
    printf 'malformed-row-without-pipes\n' > "$root/.config/source-length-exceptions.txt"
  elif [[ "$ledger_rows" -gt 0 ]]; then
    {
      printf '# Synthetic ledger for fixture test\n'
      printf '# Format: <file>|<owner>|<split_bead>|<removal_plan>|<reason>\n'
    } > "$root/.config/source-length-exceptions.txt"
    local i
    for ((i = 1; i <= ledger_rows; i++)); do
      printf 'crates/vb_x/src/foo/row_%d.rs|lewis|vb-95nyw|split-after-landing|over 300 (fixture row %d)\n' \
        "$i" "$i" >> "$root/.config/source-length-exceptions.txt"
    done
  else
    printf '# empty\n' > "$root/.config/source-length-exceptions.txt"
  fi

  # Synthetic Rust files. Each entry is "<path>:<lines>".
  local entry path lines
  IFS=';' read -ra entries <<< "$files"
  for entry in "${entries[@]}"; do
    [[ -z "$entry" ]] && continue
    path="${entry%%:*}"
    lines="${entry#*:}"
    mkdir -p "$root/$(dirname "$path")"
    : > "$root/$path"
    local j
    for ((j = 1; j <= lines; j++)); do
      printf '// line %d\n' "$j" >> "$root/$path"
    done
  done
}

run_gate() {
  local root="$1"
  shift
  GATE_REPO_ROOT="$root" bash "$GATE" "$@" 2>&1
}

# ---- fixture 1: clean repo with all files under the production limit ----

fx1=$(mktemp -d)
build_fixture "$fx1" \
  files="crates/vb_x/src/lib.rs:50;crates/vb_y/src/lib.rs:299"
out=$(run_gate "$fx1" -q; echo "_EXIT_$?")
exit_code=$(printf '%s' "$out" | sed -n 's/.*_EXIT_\([0-9]*\).*/\1/p')
out_only=$(printf '%s' "$out" | sed 's/_EXIT_[0-9]*$//')
assert_eq   'fixture 1: clean repo gate exit is 0' 0 "$exit_code"
assert_eq   'fixture 1: clean repo gate has no FAIL' 0 \
  "$(printf '%s\n' "$out_only" | { grep -c '^FAIL ' || true; })"
rm -rf "$fx1"

# ---- fixture 2: production file over 300 lines, no exception ----------

fx2=$(mktemp -d)
build_fixture "$fx2" \
  files="crates/vb_x/src/oversize.rs:350"
out=$(run_gate "$fx2" -q; echo "_EXIT_$?")
exit_code=$(printf '%s' "$out" | sed -n 's/.*_EXIT_\([0-9]*\).*/\1/p')
out_only=$(printf '%s' "$out" | sed 's/_EXIT_[0-9]*$//')
assert_eq   'fixture 2: oversize production gate exit is 1' 1 "$exit_code"
assert_contains 'fixture 2: error message names file' \
  'crates/vb_x/src/oversize.rs' \
  "$out_only"
assert_contains 'fixture 2: error mentions hard limit' \
  'hard limit 300' \
  "$out_only"
assert_contains 'fixture 2: error notes category' \
  'category=production' \
  "$out_only"
rm -rf "$fx2"

# ---- fixture 3: test_in_src file over 1500 lines, no exception ----------

fx3=$(mktemp -d)
build_fixture "$fx3" \
  files="crates/vb_x/src/tests.rs:1700"
out=$(run_gate "$fx3" -q; echo "_EXIT_$?")
exit_code=$(printf '%s' "$out" | sed -n 's/.*_EXIT_\([0-9]*\).*/\1/p')
out_only=$(printf '%s' "$out" | sed 's/_EXIT_[0-9]*$//')
assert_eq   'fixture 3: oversize test_in_src exit is 1' 1 "$exit_code"
assert_contains 'fixture 3: error mentions hard limit 1500' \
  'hard limit 1500' \
  "$out_only"
assert_contains 'fixture 3: error notes category test_in_src' \
  'category=test_in_src' \
  "$out_only"
rm -rf "$fx3"

# ---- fixture 4: ledger exception rescues over-limit file --------------

fx4=$(mktemp -d)
mkdir -p "$fx4/.config" "$fx4/crates/vb_x/src"
printf '# fixture ledger\n' > "$fx4/.config/source-length-exceptions.txt"
printf 'crates/vb_x/src/oversize.rs|lewis|vb-fixture|split-after-landing|fixture test exception\n' >> "$fx4/.config/source-length-exceptions.txt"
{
  printf '// line %d\n' {1..350}
} > "$fx4/crates/vb_x/src/oversize.rs"
out=$(run_gate "$fx4" -q; echo "_EXIT_$?")
exit_code=$(printf '%s' "$out" | sed -n 's/.*_EXIT_\([0-9]*\).*/\1/p')
out_only=$(printf '%s' "$out" | sed 's/_EXIT_[0-9]*$//')
assert_eq   'fixture 4: ledger exception rescues gate exit 0' 0 "$exit_code"
assert_eq   'fixture 4: ledger-rescued file is not in FAIL' 0 \
  "$(printf '%s\n' "$out_only" | { grep -c '^FAIL ' || true; })"
rm -rf "$fx4"

# ---- fixture 5: malformed ledger ---------------------------------------

fx5=$(mktemp -d)
build_fixture "$fx5" bad_ledger=1 files="crates/vb_x/src/lib.rs:50"
out=$(run_gate "$fx5" -q; echo "_EXIT_$?")
exit_code=$(printf '%s' "$out" | sed -n 's/.*_EXIT_\([0-9]*\).*/\1/p')
out_only=$(printf '%s' "$out" | sed 's/_EXIT_[0-9]*$//')
assert_eq   'fixture 5: malformed ledger exit is 1' 1 "$exit_code"
assert_contains 'fixture 5: malformed-row error reported' \
  'malformed row' \
  "$out_only"
rm -rf "$fx5"

# ---- fixture 6: verus file over 800 lines, no exception ----------------

fx6=$(mktemp -d)
build_fixture "$fx6" \
  files="verification/verus/spec.rs:900"
out=$(run_gate "$fx6" -q; echo "_EXIT_$?")
exit_code=$(printf '%s' "$out" | sed -n 's/.*_EXIT_\([0-9]*\).*/\1/p')
out_only=$(printf '%s' "$out" | sed 's/_EXIT_[0-9]*$//')
assert_eq   'fixture 6: oversize verus exit is 1' 1 "$exit_code"
assert_contains 'fixture 6: error mentions hard limit 800' \
  'hard limit 800' \
  "$out_only"
rm -rf "$fx6"

# ---- fixture 7: generated file excluded --------------------------------

fx7=$(mktemp -d)
build_fixture "$fx7" \
  files="crates/vb_x/src/generated/big.rs:5000"
out=$(run_gate "$fx7" -q; echo "_EXIT_$?")
exit_code=$(printf '%s' "$out" | sed -n 's/.*_EXIT_\([0-9]*\).*/\1/p')
out_only=$(printf '%s' "$out" | sed 's/_EXIT_[0-9]*$//')
assert_eq   'fixture 7: generated file excluded exit 0' 0 "$exit_code"
rm -rf "$fx7"

# ---- fixture 8: summary mode prints WARNs only in verbose ------------

fx8=$(mktemp -d)
build_fixture "$fx8" \
  files="crates/vb_x/src/warner.rs:260"
out=$(run_gate "$fx8" -q; echo "_EXIT_$?")
quiet_warns=$(printf '%s' "$out" | { grep -c "^WARN " || true; })
out=$(run_gate "$fx8" --verbose; echo "_EXIT_$?")
verbose_warns=$(printf '%s' "$out" | { grep -c "^WARN " || true; })
assert_eq   'fixture 8: -q mode hides WARN' 0 "$quiet_warns"
if [[ "$verbose_warns" -ge 1 ]]; then
  PASS=$((PASS + 1))
  printf 'ok %3d - fixture 8: --verbose shows WARN\n' "$PASS"
else
  FAIL_CASES+=('fixture 8: --verbose shows WARN')
  printf 'not ok    - --verbose mode hides WARNs\n'
fi
rm -rf "$fx8"

# ---- summary ----------------------------------------------------------

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
