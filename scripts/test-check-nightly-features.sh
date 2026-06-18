#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$script_dir/.." && pwd)
fixture_root="$repo_root/fixtures/nightly-feature-gate"
test_root="$repo_root/target/nightly-feature-gate-tests"
failures=0

bootstrap_token="RUSTC_""BOOTSTRAP"
bootstrap_fixture_dir="RUSTC_""BOOTSTRAP"

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

assert_output_contains() {
  local test_name="$1"
  local needle="$2"
  local output="$3"

  if [[ "$output" != *"$needle"* ]]; then
    fail "$test_name" "missing output substring: $needle; output: $output"
  fi
}

assert_output_omits() {
  local test_name="$1"
  local needle="$2"
  local output="$3"

  if [[ "$output" == *"$needle"* ]]; then
    fail "$test_name" "unexpected output substring: $needle; output: $output"
  fi
}

assert_output_empty() {
  local test_name="$1"
  local output="$2"

  if [[ -n "$output" ]]; then
    fail "$test_name" "expected empty output, got: $output"
  fi
}

require_fixture() {
  local test_name="$1"
  local source_rel="$2"
  local source_path="$fixture_root/$source_rel"

  if [[ ! -f "$source_path" ]]; then
    fail "$test_name" "missing fixture: fixtures/nightly-feature-gate/$source_rel"
    return 1
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

stage_fixture() {
  local test_name="$1"
  local sandbox="$2"
  local source_rel="$3"
  local dest_rel="$4"
  local source_path="$fixture_root/$source_rel"
  local dest_path="$sandbox/$dest_rel"

  require_fixture "$test_name" "$source_rel" || return 1
  mkdir -p "$(dirname -- "$dest_path")"
  cp -f "$source_path" "$dest_path"
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

run_source_syntax_check() {
  local -n output_ref="$1"
  local -n status_ref="$2"
  local captured_output
  local captured_status

  set +e
  captured_output=$(cd "$repo_root" && bash -n scripts/check-nightly-features.sh 2>&1)
  captured_status=$?
  set -e

  output_ref="$captured_output"
  status_ref="$captured_status"
}

make_clean_rs_files() {
  local sandbox="$1"
  local n="$2"
  local i
  local rel
  local path

  for ((i = 1; i <= n; i += 1)); do
    rel=$(printf 'crates/terminates/src/bin/file_%03d.rs' "$i")
    path="$sandbox/$rel"
    mkdir -p "$(dirname -- "$path")"
    printf 'fn main() {}\n' > "$path"
  done
}

write_bootstrap_doc() {
  local sandbox="$1"
  local rel="$2"
  local path="$sandbox/$rel"

  mkdir -p "$(dirname -- "$path")"
  printf '%s\n' "$bootstrap_token=1" > "$path"
}

run_positive_fixture() {
  local test_name="$1"
  local source_rel="$2"
  local dest_rel="$3"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  stage_fixture "$test_name" "$sandbox" "$source_rel" "$dest_rel"
  run_gate "$sandbox" output status
  assert_exit "$test_name" 0 "$status" "$output"
  assert_output_empty "$test_name" "$output"
  assert_output_omits "$test_name" 'disallowed unstable feature' "$output"
  assert_output_omits "$test_name" 'perf-only unstable feature' "$output"
}

test_nightly_feature_gate_blocks_try_blocks_outside_perf() {
  local test_name="test_nightly_feature_gate_blocks_try_blocks_outside_perf"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  stage_fixture "$test_name" "$sandbox" 'normal/negative_specialization.rs' 'fixtures/nightly-feature-gate/negative_specialization.rs'
  run_gate "$sandbox" output status
  assert_exit "$test_name" 1 "$status" "$output"
  assert_output_contains "$test_name" 'disallowed unstable feature specialization' "$output"
  assert_output_contains "$test_name" 'fixtures/nightly-feature-gate/negative_specialization.rs:' "$output"
  assert_output_omits "$test_name" 'perf-only unstable feature' "$output"
}

test_nightly_feature_gate_allows_portable_simd_in_perf() {
  run_positive_fixture \
    'test_nightly_feature_gate_allows_portable_simd_in_perf' \
    'perf/perf_portable_simd.rs' \
    'crates/nightly_feature_gate/src/perf/perf_portable_simd.rs'
}

test_nightly_feature_gate_rejects_allocator_api_outside_perf() {
  local test_name="test_nightly_feature_gate_rejects_allocator_api_outside_perf"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  stage_fixture "$test_name" "$sandbox" 'normal/negative_allocator_api.rs' 'fixtures/nightly-feature-gate/negative_allocator_api.rs'
  run_gate "$sandbox" output status
  assert_exit "$test_name" 1 "$status" "$output"
  assert_output_contains "$test_name" 'perf-only unstable feature allocator_api' "$output"
  assert_output_contains "$test_name" 'outside approved scope' "$output"
  assert_output_contains "$test_name" 'fixtures/nightly-feature-gate/negative_allocator_api.rs:' "$output"
  assert_output_omits "$test_name" 'disallowed unstable feature' "$output"
}

test_nightly_feature_gate_resolves_scope_perf_path() {
  run_positive_fixture \
    'test_nightly_feature_gate_resolves_scope_perf_path' \
    'perf/perf_allocator_api.rs' \
    'crates/nightly_feature_gate/src/perf/perf_allocator_api.rs'
}

test_nightly_feature_gate_resolves_scope_generated_path() {
  run_positive_fixture \
    'test_nightly_feature_gate_resolves_scope_generated_path' \
    'generated/generated_allocator_api.rs' \
    'crates/nightly_feature_gate/src/generated/generated_allocator_api.rs'
}

test_nightly_feature_gate_resolves_scope_bench_path() {
  run_positive_fixture \
    'test_nightly_feature_gate_resolves_scope_bench_path' \
    'bench/bench_generic_const_exprs.rs' \
    'benches/bench_generic_const_exprs.rs'
}

test_nightly_feature_gate_resolves_scope_normal_path() {
  run_positive_fixture \
    'test_nightly_feature_gate_resolves_scope_normal_path' \
    'normal/positive_portable_simd.rs' \
    'fixtures/nightly-feature-gate/normal/positive_portable_simd.rs'
}

test_nightly_feature_gate_allows_try_blocks_anywhere() {
  run_positive_fixture \
    'test_nightly_feature_gate_allows_try_blocks_anywhere' \
    'normal/positive_try_blocks.rs' \
    'fixtures/nightly-feature-gate/normal/positive_try_blocks.rs'
}

test_nightly_feature_gate_allows_generic_const_exprs_in_benches() {
  run_positive_fixture \
    'test_nightly_feature_gate_allows_generic_const_exprs_in_benches' \
    'bench/bench_generic_const_exprs.rs' \
    'benches/bench_generic_const_exprs.rs'
}

test_nightly_feature_gate_rejects_generic_const_exprs_outside_perf() {
  local test_name="test_nightly_feature_gate_rejects_generic_const_exprs_outside_perf"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  stage_fixture "$test_name" "$sandbox" 'normal/negative_generic_const_exprs.rs' 'fixtures/nightly-feature-gate/normal/negative_generic_const_exprs.rs'
  run_gate "$sandbox" output status
  assert_exit "$test_name" 1 "$status" "$output"
  assert_output_contains "$test_name" 'perf-only unstable feature generic_const_exprs' "$output"
  assert_output_contains "$test_name" 'outside approved scope' "$output"
  assert_output_contains "$test_name" 'fixtures/nightly-feature-gate/normal/negative_generic_const_exprs.rs:' "$output"
  assert_output_omits "$test_name" 'disallowed unstable feature' "$output"
}

test_nightly_feature_gate_allows_allocator_api_with_marker() {
  run_positive_fixture \
    'test_nightly_feature_gate_allows_allocator_api_with_marker' \
    'normal/positive_allocator_api_with_marker.rs' \
    'fixtures/nightly-feature-gate/normal/positive_allocator_api_with_marker.rs'
}

test_nightly_feature_gate_rejects_rustc_bootstrap_in_tracked_file() {
  local test_name="test_nightly_feature_gate_rejects_rustc_bootstrap_in_tracked_file"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  stage_fixture "$test_name" "$sandbox" "$bootstrap_fixture_dir/rustc_bootstrap_violation.sh" 'fixtures/nightly-feature-gate/rustc_bootstrap_violation.sh'
  run_gate "$sandbox" output status
  assert_exit "$test_name" 1 "$status" "$output"
  assert_output_contains "$test_name" "$bootstrap_token is rejected by master §4 in fixtures/nightly-feature-gate/rustc_bootstrap_violation.sh" "$output"
}

test_nightly_feature_gate_allows_rustc_bootstrap_in_skip_set() {
  local test_name="test_nightly_feature_gate_allows_rustc_bootstrap_in_skip_set"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  run_gate "$sandbox" output status
  assert_exit "$test_name" 0 "$status" "$output"
  assert_output_empty "$test_name" "$output"
}

bash_n_syntax_check() {
  local test_name="bash_n_syntax_check"
  local output status

  run_source_syntax_check output status
  assert_exit "$test_name" 0 "$status" "$output"
  assert_output_empty "$test_name" "$output"
}

test_nightly_feature_gate_terminates_on_n_files() {
  local test_name="test_nightly_feature_gate_terminates_on_n_files"
  local n="${N_FILES:-1}"
  local sandbox output status elapsed

  if [[ ! "$n" =~ ^(1|10|100)$ ]]; then
    fail "$test_name" "--n must be one of 1, 10, 100; got $n"
    return 0
  fi

  sandbox=$(new_sandbox "${test_name}_n${n}")
  make_clean_rs_files "$sandbox" "$n"
  SECONDS=0
  run_gate "$sandbox" output status
  elapsed=$SECONDS
  assert_exit "$test_name" 0 "$status" "$output"
  assert_output_empty "$test_name" "$output"
  if (( elapsed >= 60 )); then
    fail "$test_name" "expected completion under 60s for n=$n, got ${elapsed}s"
  fi
}

test_nightly_feature_gate_multiline_feature_attribute() {
  local test_name="test_nightly_feature_gate_multiline_feature_attribute"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  stage_fixture "$test_name" "$sandbox" 'normal/edge_multiline.rs' 'fixtures/nightly-feature-gate/normal/edge_multiline.rs'
  run_gate "$sandbox" output status
  assert_exit "$test_name" 1 "$status" "$output"
  assert_output_contains "$test_name" 'perf-only unstable feature allocator_api' "$output"
  assert_output_contains "$test_name" 'perf-only unstable feature generic_const_exprs' "$output"
}

test_nightly_feature_gate_unterminated_feature_attribute() {
  local test_name="test_nightly_feature_gate_unterminated_feature_attribute"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  stage_fixture "$test_name" "$sandbox" 'normal/edge_unterminated.rs' 'fixtures/nightly-feature-gate/normal/edge_unterminated.rs'
  run_gate "$sandbox" output status
  assert_exit "$test_name" 1 "$status" "$output"
  assert_output_contains "$test_name" 'unterminated unstable feature attribute in fixtures/nightly-feature-gate/normal/edge_unterminated.rs:' "$output"
}

test_nightly_feature_gate_whitespace_in_feature_attribute() {
  local test_name="test_nightly_feature_gate_whitespace_in_feature_attribute"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  stage_fixture "$test_name" "$sandbox" 'normal/edge_whitespace.rs' 'fixtures/nightly-feature-gate/normal/edge_whitespace.rs'
  run_gate "$sandbox" output status
  assert_exit "$test_name" 1 "$status" "$output"
  assert_output_contains "$test_name" 'perf-only unstable feature allocator_api' "$output"
  assert_output_contains "$test_name" 'outside approved scope' "$output"
}

test_nightly_feature_gate_marker_with_whitespace_works() {
  run_positive_fixture \
    'test_nightly_feature_gate_marker_with_whitespace_works' \
    'normal/marker_with_whitespace.rs' \
    'fixtures/nightly-feature-gate/normal/marker_with_whitespace.rs'
}

test_nightly_feature_gate_marker_case_sensitive() {
  local test_name="test_nightly_feature_gate_marker_case_sensitive"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  stage_fixture "$test_name" "$sandbox" 'normal/marker_case_sensitive.rs' 'fixtures/nightly-feature-gate/normal/marker_case_sensitive.rs'
  run_gate "$sandbox" output status
  assert_exit "$test_name" 1 "$status" "$output"
  assert_output_contains "$test_name" 'perf-only unstable feature allocator_api' "$output"
}

test_nightly_feature_gate_bootstrap_in_shell_comment() {
  local test_name="test_nightly_feature_gate_bootstrap_in_shell_comment"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  stage_fixture "$test_name" "$sandbox" "$bootstrap_fixture_dir/rustc_bootstrap_comment.sh" 'fixtures/nightly-feature-gate/rustc_bootstrap_comment.sh'
  run_gate "$sandbox" output status
  assert_exit "$test_name" 1 "$status" "$output"
  assert_output_contains "$test_name" "$bootstrap_token is rejected by master §4 in fixtures/nightly-feature-gate/rustc_bootstrap_comment.sh" "$output"
}

test_nightly_feature_gate_bootstrap_in_toml_dependency_block() {
  local test_name="test_nightly_feature_gate_bootstrap_in_toml_dependency_block"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  stage_fixture "$test_name" "$sandbox" "$bootstrap_fixture_dir/rustc_bootstrap_toml.toml" 'fixtures/nightly-feature-gate/rustc_bootstrap_toml.toml'
  run_gate "$sandbox" output status
  assert_exit "$test_name" 1 "$status" "$output"
  assert_output_contains "$test_name" "$bootstrap_token is rejected by master §4 in fixtures/nightly-feature-gate/rustc_bootstrap_toml.toml" "$output"
}

test_nightly_feature_gate_bootstrap_in_yaml_ci_step() {
  local test_name="test_nightly_feature_gate_bootstrap_in_yaml_ci_step"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  stage_fixture "$test_name" "$sandbox" "$bootstrap_fixture_dir/rustc_bootstrap_yaml.yml" 'fixtures/nightly-feature-gate/rustc_bootstrap_yaml.yml'
  run_gate "$sandbox" output status
  assert_exit "$test_name" 1 "$status" "$output"
  assert_output_contains "$test_name" "$bootstrap_token is rejected by master §4 in fixtures/nightly-feature-gate/rustc_bootstrap_yaml.yml" "$output"
}

test_nightly_feature_gate_bootstrap_in_markdown_doc() {
  local test_name="test_nightly_feature_gate_bootstrap_in_markdown_doc"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  write_bootstrap_doc "$sandbox" 'docs/poison.md'
  run_gate "$sandbox" output status
  assert_exit "$test_name" 1 "$status" "$output"
  assert_output_contains "$test_name" "$bootstrap_token is rejected by master §4 in docs/poison.md" "$output"
}

test_nightly_feature_gate_allows_bootstrap_in_documentation_skip_set() {
  local test_name="test_nightly_feature_gate_allows_bootstrap_in_documentation_skip_set"
  local sandbox output status

  sandbox=$(new_sandbox "$test_name")
  write_bootstrap_doc "$sandbox" 'velvet-ballistics-MASTER.md'
  write_bootstrap_doc "$sandbox" 'docs/rust-governance.md'
  write_bootstrap_doc "$sandbox" 'docs/xtask-prd.md'
  run_gate "$sandbox" output status
  assert_exit "$test_name" 0 "$status" "$output"
  assert_output_empty "$test_name" "$output"
}

test_nightly_feature_gate_marker_on_non_perf_file_allows() {
  run_positive_fixture \
    'test_nightly_feature_gate_marker_on_non_perf_file_allows' \
    'normal/normal_allocator_api_with_marker.rs' \
    'fixtures/nightly-feature-gate/normal/normal_allocator_api_with_marker.rs'
}

all_tests=(
  test_nightly_feature_gate_blocks_try_blocks_outside_perf
  test_nightly_feature_gate_allows_portable_simd_in_perf
  test_nightly_feature_gate_rejects_allocator_api_outside_perf
  test_nightly_feature_gate_resolves_scope_perf_path
  test_nightly_feature_gate_resolves_scope_generated_path
  test_nightly_feature_gate_resolves_scope_bench_path
  test_nightly_feature_gate_resolves_scope_normal_path
  test_nightly_feature_gate_allows_try_blocks_anywhere
  test_nightly_feature_gate_allows_generic_const_exprs_in_benches
  test_nightly_feature_gate_rejects_generic_const_exprs_outside_perf
  test_nightly_feature_gate_allows_allocator_api_with_marker
  test_nightly_feature_gate_rejects_rustc_bootstrap_in_tracked_file
  test_nightly_feature_gate_allows_rustc_bootstrap_in_skip_set
  bash_n_syntax_check
  test_nightly_feature_gate_multiline_feature_attribute
  test_nightly_feature_gate_unterminated_feature_attribute
  test_nightly_feature_gate_whitespace_in_feature_attribute
  test_nightly_feature_gate_marker_with_whitespace_works
  test_nightly_feature_gate_marker_case_sensitive
  test_nightly_feature_gate_bootstrap_in_shell_comment
  test_nightly_feature_gate_bootstrap_in_toml_dependency_block
  test_nightly_feature_gate_bootstrap_in_yaml_ci_step
  test_nightly_feature_gate_bootstrap_in_markdown_doc
  test_nightly_feature_gate_allows_bootstrap_in_documentation_skip_set
  test_nightly_feature_gate_marker_on_non_perf_file_allows
)

run_one() {
  local test_name="$1"

  if ! declare -F "$test_name" >/dev/null; then
    fail "$test_name" "unknown test"
    return 0
  fi

  "$test_name"
  if (( failures == 0 )); then
    printf 'ok - %s\n' "$test_name"
  fi
}

parse_n_arg() {
  local arg="$1"

  case "$arg" in
    --n=*) N_FILES="${arg#--n=}" ;;
    --n) shift ;;
    *) fail "argument-parser" "unknown argument: $arg" ;;
  esac
}

selected="${1:-}"
if [[ -n "$selected" ]]; then
  shift
  while (($# > 0)); do
    case "$1" in
      --n=*) N_FILES="${1#--n=}" ;;
      --n)
        shift
        if (($# == 0)); then
          fail "$selected" 'missing value after --n'
          break
        fi
        N_FILES="$1"
        ;;
      *) fail "$selected" "unknown argument: $1" ;;
    esac
    shift || true
  done
  run_one "$selected"
else
  for test_name in "${all_tests[@]}"; do
    if [[ "$test_name" == 'test_nightly_feature_gate_terminates_on_n_files' ]]; then
      continue
    fi
    run_one "$test_name"
  done
  for n in 1 10 100; do
    N_FILES="$n" run_one test_nightly_feature_gate_terminates_on_n_files
  done
fi

if (( failures > 0 )); then
  printf '%s test assertion(s) failed\n' "$failures" >&2
  exit 1
fi

exit 0
