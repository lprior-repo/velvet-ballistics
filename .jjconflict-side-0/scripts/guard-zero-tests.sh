#!/usr/bin/env bash
set -euo pipefail
# guard-zero-tests.sh — fail-closed detector that rejects vacuous cargo test output
# (zero applicable tests). Wraps cargo test with passthrough (-- separator).
#
# usage: bash scripts/guard-zero-tests.sh -- <cargo test args>
#   exit 0: non-zero applicable tests executed
#   exit 1: tooling failure or zero applicable tests detected
#   exit 2: usage error
#
# Obligations: PO-006 (zero-test detector), PO-007 (non-vacuous execution proof)

usage() {
  printf 'usage: bash scripts/guard-zero-tests.sh -- <cargo-test-args>\n' >&2
  printf '  wrapper for cargo test that rejects zero-test (vacuous) output\n' >&2
  printf '  exit 0: >0 applicable tests executed\n' >&2
  printf '  exit 1: 0 applicable tests or tooling failure\n' >&2
  printf '  exit 2: usage error\n' >&2
}

# Parse the passthrough separator
passthrough_marker_found=false
passthrough_args=()
for arg in "$@"; do
  if [ "$arg" = "--" ]; then
    passthrough_marker_found=true
  elif $passthrough_marker_found; then
    passthrough_args+=("$arg")
  fi
done

if ! $passthrough_marker_found; then
  usage
  exit 2
fi

if [ "${#passthrough_args[@]}" -eq 0 ]; then
  printf 'guard-zero-tests: no cargo test arguments after --\n' >&2
  usage
  exit 2
fi

printf '[guard-zero-tests] running: %s\n' "${passthrough_args[*]}" >&2

# Run the command passed after --, capture stdout+stderr
output="$(mktemp)"
trap 'rm -f -- "$output"' EXIT
set +e
"${passthrough_args[@]}" >"$output" 2>&1
cargo_exit=$?
set -e

# Parse for test count from various cargo output formats.
# Format 1 (cargo nextest / standard libtest): "running N tests"
# Format 2 (libtest summary): "test result: ok. N passed; M failed; ..."
# Format 3 (cargo test summary): "cargo test: N passed (M suites, ...)"
# Format 4 (cargo filtered): "cargo test: N passed, M filtered out (S suites, ...)"
applicable_count=-1

# Pattern 1: "running 5 tests" or "running 0 tests"
count_line="$(grep -E '^running [0-9]+ tests?$' "$output" | head -1)"
if [ -n "$count_line" ]; then
  applicable_count="$(echo "$count_line" | sed -n 's/^running \([0-9]\+\) tests\?$/\1/p')"
fi

# Pattern 2: "test result: ok. 5 passed; 0 failed; ..." (from test binary output)
if [ "$applicable_count" = "-1" ]; then
  result_line="$(grep -E '^test result:' "$output" | head -1)"
  if [ -n "$result_line" ]; then
    passed="$(echo "$result_line" | sed -n 's/.* \([0-9]\+\) passed.*/\1/p')"
    if [ -n "$passed" ]; then
      applicable_count="$passed"
    fi
  fi
fi

# Pattern 3: "cargo test: N passed (M suites, X.XXs)"
if [ "$applicable_count" = "-1" ]; then
  cargo_line="$(grep -E '^cargo test: [0-9]+ passed' "$output" | head -1)"
  if [ -n "$cargo_line" ]; then
    # "cargo test: 5 passed (1 suite, 0.08s)" -> extract "5"
    applicable_count="$(echo "$cargo_line" | sed -n 's/^cargo test: \([0-9]\+\) passed.*/\1/p')"
  fi
fi

# Pattern 4: "cargo test: N passed, M filtered out" — treat N as applicable count
# If all tests are filtered (N=0), this is vacuous
if [ "$applicable_count" = "-1" ]; then
  cargo_filtered="$(grep -E '^cargo test: [0-9]+ passed, [0-9]+ filtered out' "$output" | head -1)"
  if [ -n "$cargo_filtered" ]; then
    applicable_count="$(echo "$cargo_filtered" | sed -n 's/^cargo test: \([0-9]\+\) passed.*/\1/p')"
  fi
fi

if [ "$cargo_exit" -ne 0 ]; then
  printf '[guard-zero-tests] cargo test exited %d — treating as tooling failure\n' "$cargo_exit" >&2
  # Still parse for zero tests
  if [ "$applicable_count" != "-1" ]; then
    printf '[guard-zero-tests] applicable test count: %d (cargo failed with exit %d)\n' "$applicable_count" "$cargo_exit" >&2
  fi
  cat "$output" >&2
  exit 1
fi

if [ "$applicable_count" = "-1" ]; then
  printf '[guard-zero-tests] FAIL: could not parse test count from cargo output.\n' >&2
  printf '[guard-zero-tests] Raw output:\n' >&2
  cat "$output" >&2
  exit 1
fi

if [ "$applicable_count" -le 0 ]; then
  printf '[guard-zero-tests] FAIL: zero applicable tests detected (count=%d). Refusing vacuous evidence.\n' "$applicable_count" >&2
  exit 1
fi

printf '[guard-zero-tests] PASS: %d applicable tests executed\n' "$applicable_count" >&2
exit 0
