#!/usr/bin/env bash
set -euo pipefail
# runner.sh — Test runner for vb-shvxy tooling tests (State 9)
#
# Each test file is a self-contained bash script with a `main()` function.
# The runner executes each file in its own bash process and reports results.
#
# Test file contract:
#   - Must define `main()` that runs all tests.
#   - `main()` must return 0 if all pass, 1 if any fail.
#
# usage: bash tests/tooling/runner.sh [test_file...]

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

GLOBAL_PASS=0
GLOBAL_FAIL=0

run_test_file() {
  local test_file="$1"
  local test_file_name
  test_file_name="$(basename "$test_file")"
  printf '\n=== %s ===\n' "$test_file_name"

  set +e
  local output
  output="$(bash "$test_file" 2>&1)"
  local exit_code=$?
  set -e

  if [ "$exit_code" -eq 0 ]; then
    printf "${GREEN}PASS${NC}\n"
    GLOBAL_PASS=$((GLOBAL_PASS + 1))
  else
    printf "${RED}FAIL${NC} (exit=%d)\n" "$exit_code"
    GLOBAL_FAIL=$((GLOBAL_FAIL + 1))
  fi

  # Print the test file's output (test results)
  if [ -n "$output" ]; then
    printf '%s\n' "$output" | while IFS= read -r line; do printf '  %s\n' "$line"; done
  fi
}

print_summary() {
  local total=$((GLOBAL_PASS + GLOBAL_FAIL))
  printf '\n'
  printf '========================================\n'
  printf 'Test Suite Summary\n'
  printf '========================================\n'
  printf 'Files passed: %d\n' "$GLOBAL_PASS"
  printf 'Files failed: %d\n' "$GLOBAL_FAIL"
  printf 'Total files:  %d\n' "$total"
  printf '========================================\n'

  if [ "$GLOBAL_FAIL" -gt 0 ]; then
    printf '%sSOME TESTS FAILED%s\n' "$RED" "$NC"
    return 1
  else
    printf '%sALL TESTS PASSED%s\n' "$GREEN" "$NC"
    return 0
  fi
}

main() {
  if [ "$#" -eq 0 ]; then
    local test_files
    mapfile -t test_files < <(find "$SCRIPT_DIR" -maxdepth 1 -name 'test_*.sh' | sort)
    for test_file in "${test_files[@]}"; do
      run_test_file "$test_file"
    done
  else
    for test_file in "$@"; do
      local abs_file
      if [ -f "$test_file" ]; then
        abs_file="$test_file"
      elif [ -f "$SCRIPT_DIR/$test_file" ]; then
        abs_file="$SCRIPT_DIR/$test_file"
      else
        printf 'Test file not found: %s\n' "$test_file" >&2
        return 1
      fi
      run_test_file "$abs_file"
    done
  fi

  print_summary
}

main "$@"
