#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$script_dir/.." && pwd)
moon_file="$repo_root/.moon/tasks/all.yml"
test_name="test_nightly_feature_gate_moon_task_invokes_self_test_runner"
failures=0

fail() {
  local message="$1"

  printf 'not ok - %s - %s\n' "$test_name" "$message" >&2
  failures=$((failures + 1))
}

assert_contains() {
  local needle="$1"
  local haystack="$2"

  if [[ "$haystack" != *"$needle"* ]]; then
    fail "missing Moon wiring substring: $needle"
  fi
}

if [[ ! -f "$moon_file" ]]; then
  fail 'missing .moon/tasks/all.yml'
else
  moon_body=$(<"$moon_file")
  assert_contains 'nightly-feature-gate-test:' "$moon_body"
  assert_contains "command: 'bash scripts/test-check-nightly-features.sh'" "$moon_body"
  assert_contains "- 'nightly-feature-gate-test'" "$moon_body"
fi

if (( failures > 0 )); then
  printf '%s test assertion(s) failed\n' "$failures" >&2
  exit 1
fi

printf 'ok - %s\n' "$test_name"
exit 0
