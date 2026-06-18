#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$script_dir/.." && pwd)
test_root="$repo_root/target/nightly-feature-gate-real-scope-globs"
failures=0

fail() {
  local test_name="$1"
  local message="$2"

  printf 'not ok - %s - %s\n' "$test_name" "$message" >&2
  failures=$((failures + 1))
}

assert_exit() {
  local test_name="$1"
  local expected="$2"
  local actual="$3"
  local output="$4"

  if [[ "$actual" != "$expected" ]]; then
    fail "$test_name" "expected exit $expected, got $actual; output: $output"
  fi
}

assert_output_empty() {
  local test_name="$1"
  local output="$2"

  if [[ -n "$output" ]]; then
    fail "$test_name" "expected empty output, got: $output"
  fi
}

assert_output_contains() {
  local test_name="$1"
  local needle="$2"
  local output="$3"

  if [[ "$output" != *"$needle"* ]]; then
    fail "$test_name" "missing output substring: $needle; output: $output"
  fi
}

new_sandbox() {
  local name="$1"
  local sandbox="$test_root/$name"

  rm -rf "$sandbox"
  mkdir -p "$sandbox/scripts"
  cp -f "$repo_root/scripts/check-nightly-features.sh" "$sandbox/scripts/check-nightly-features.sh"
  printf '%s\n' "$sandbox"
}

write_rs() {
  local sandbox="$1"
  local rel="$2"
  local feature="$3"
  local path="$sandbox/$rel"

  mkdir -p "$(dirname -- "$path")"
  printf '#![feature(%s)]\nfn main() {}\n' "$feature" > "$path"
}

run_gate() {
  local sandbox="$1"
  local -n output_ref="$2"
  local -n status_ref="$3"
  local captured_output
  local captured_status

  set +e
  captured_output=$(cd "$sandbox" && timeout 60s bash scripts/check-nightly-features.sh 2>&1)
  captured_status=$?
  set -e

  output_ref="$captured_output"
  status_ref="$captured_status"
}

test_nightly_feature_gate_real_scope_globs_allow_perf_only_features() {
  local test_name="test_nightly_feature_gate_real_scope_globs_allow_perf_only_features"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  write_rs "$sandbox" 'crates/nightly_feature_gate/src/perf/perf_allocator_api.rs' 'allocator_api'
  write_rs "$sandbox" 'crates/nightly_feature_gate/src/generated/generated_generic_const_exprs.rs' 'generic_const_exprs'
  write_rs "$sandbox" 'benches/bench_allocator_api.rs' 'allocator_api'

  run_gate "$sandbox" output status
  assert_exit "$test_name" 0 "$status" "$output"
  assert_output_empty "$test_name" "$output"

  if (( failures == 0 )); then
    printf 'ok - %s\n' "$test_name"
  fi
}

test_nightly_feature_gate_real_scope_globs_reject_perf_only_outside_scope() {
  local test_name="test_nightly_feature_gate_real_scope_globs_reject_perf_only_outside_scope"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  write_rs "$sandbox" 'crates/nightly_feature_gate/src/core/negative_allocator_api.rs' 'allocator_api'

  run_gate "$sandbox" output status
  assert_exit "$test_name" 1 "$status" "$output"
  assert_output_contains "$test_name" 'perf-only unstable feature allocator_api outside approved scope in crates/nightly_feature_gate/src/core/negative_allocator_api.rs:' "$output"

  if (( failures == 0 )); then
    printf 'ok - %s\n' "$test_name"
  fi
}

test_nightly_feature_gate_real_scope_globs_allow_perf_only_features
test_nightly_feature_gate_real_scope_globs_reject_perf_only_outside_scope

if (( failures > 0 )); then
  printf '%s test assertion(s) failed\n' "$failures" >&2
  exit 1
fi

exit 0
