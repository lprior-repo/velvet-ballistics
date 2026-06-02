#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd -- "$script_dir/.." && pwd -P)"
gate="$repo_root/scripts/check-source-length.sh"
tmp_repos=()

cleanup() {
  local repo
  for repo in "${tmp_repos[@]}"; do
    rm -rf "$repo"
  done
}
trap cleanup EXIT

make_repo() {
  local repo
  repo="$(mktemp -d "${TMPDIR:-/tmp}/source-length-test.XXXXXX")"
  tmp_repos+=("$repo")
  git -C "$repo" init -q
  mkdir -p "$repo/.config"
  printf '# test exception ledger\n' > "$repo/.config/source-length-exceptions.txt"
  printf '# test hot-function exception ledger\n' > "$repo/.config/hot-function-length-exceptions.txt"
  mkdir -p "$repo/crates/vb_compile/src"
  printf 'mod part_001;\n' > "$repo/crates/vb_compile/src/mod_compile_core.rs"
  printf 'mod part_001;\n' > "$repo/crates/vb_compile/src/mod_compile_errors.rs"
  printf 'mod part_001;\n' > "$repo/crates/vb_compile/src/mod_compile_validation.rs"
  printf 'mod part_001;\n' > "$repo/crates/vb_compile/src/mod_compile_lowering.rs"
  printf '%s\n' "$repo"
}

track_repo() {
  git -C "$1" add .
}

run_gate() {
  local repo="$1"
  local file_limit="$2"
  local function_limit="$3"
  (
    cd "$repo"
    SOURCE_LENGTH_FILE_LIMIT="$file_limit" \
      SOURCE_LENGTH_HOT_FUNCTION_LIMIT="$function_limit" \
      bash "$gate"
  )
}

assert_contains() {
  local haystack="$1"
  local needle="$2"
  local context="$3"
  case "$haystack" in
    *"$needle"*) ;;
    *)
      printf 'AssertionFailed: %s missing %s\nOutput:\n%s\n' "$context" "$needle" "$haystack" >&2
      return 1
      ;;
  esac
}

write_numbered_lines() {
  local file="$1"
  local count="$2"
  local line
  mkdir -p "$(dirname -- "$file")"
  for ((line = 1; line <= count; line += 1)); do
    printf '// line %s\n' "$line"
  done > "$file"
}

expect_gate_failure() {
  local repo="$1"
  local file_limit="$2"
  local function_limit="$3"
  local context="$4"
  local output
  if output="$(run_gate "$repo" "$file_limit" "$function_limit" 2>&1)"; then
    printf 'Expected source-length gate failure: %s\n' "$context" >&2
    return 1
  fi
  printf '%s\n' "$output"
}

write_hot_function() {
  local file="$1"
  local name="$2"
  mkdir -p "$(dirname -- "$file")"
  {
    printf 'pub fn %s() {\n' "$name"
    printf '    let first = 1;\n'
    printf '    let second = 2;\n'
    printf '    let third = first + second;\n'
    printf '}\n'
  } > "$file"
}

test_gate_passes_on_compliant_files() {
  local repo
  repo="$(make_repo)"
  mkdir -p "$repo/crates/vb_runtime/src"
  printf 'pub fn ok() {}\n' > "$repo/crates/vb_runtime/src/lib.rs"
  track_repo "$repo"
  run_gate "$repo" 5 3 >/dev/null
}

test_gate_fails_on_over_limit_test_like_source() {
  local repo
  local output
  repo="$(make_repo)"
  write_numbered_lines "$repo/crates/vb_cli/src/args/tests/oversize.rs" 6
  track_repo "$repo"
  output="$(expect_gate_failure "$repo" 5 99 'over-limit test-like source')"
  assert_contains "$output" 'crates/vb_cli/src/args/tests/oversize.rs has 6 physical lines' 'over-limit test-like source failure'
}

test_gate_fails_on_over_limit_arbitrary_first_party_source() {
  local repo
  local output
  repo="$(make_repo)"
  write_numbered_lines "$repo/crates/vb_expr/src/oversize.rs" 6
  track_repo "$repo"
  output="$(expect_gate_failure "$repo" 5 99 'over-limit arbitrary first-party source')"
  assert_contains "$output" 'crates/vb_expr/src/oversize.rs has 6 physical lines' 'over-limit arbitrary source failure'
}

test_gate_fails_on_over_limit_proof_source() {
  local repo
  local output
  repo="$(make_repo)"
  write_numbered_lines "$repo/verification/proof_case.rs" 6
  track_repo "$repo"
  output="$(expect_gate_failure "$repo" 5 99 'over-limit proof source')"
  assert_contains "$output" 'verification/proof_case.rs has 6 physical lines' 'over-limit proof source failure'
}

test_gate_fails_on_hot_function_over_limit() {
  local repo
  local output
  repo="$(make_repo)"
  write_hot_function "$repo/crates/vb_runtime/src/long.rs" 'too_long'
  track_repo "$repo"
  output="$(expect_gate_failure "$repo" 99 3 'over-limit hot function')"
  assert_contains "$output" 'crates/vb_runtime/src/long.rs:1 hot function has 4 logical lines' 'hot function failure'
}

test_valid_source_ledger_allows_over_limit_file() {
  local repo
  repo="$(make_repo)"
  write_numbered_lines "$repo/crates/vb_expr/src/ledgered.rs" 6
  printf 'crates/vb_expr/src/ledgered.rs|tester|vb-test|split-later|fixture exception\n' > "$repo/.config/source-length-exceptions.txt"
  track_repo "$repo"
  run_gate "$repo" 5 99 >/dev/null
}

test_malformed_source_ledger_fails() {
  local repo
  local output
  repo="$(make_repo)"
  printf 'crates/vb_expr/src/file.rs|too-few-fields\n' > "$repo/.config/source-length-exceptions.txt"
  track_repo "$repo"
  output="$(expect_gate_failure "$repo" 5 99 'malformed source ledger')"
  assert_contains "$output" 'malformed row' 'malformed source ledger failure'
}

test_duplicate_source_ledger_fails() {
  local repo
  local output
  repo="$(make_repo)"
  write_numbered_lines "$repo/crates/vb_expr/src/duplicate.rs" 6
  {
    printf 'crates/vb_expr/src/duplicate.rs|tester|vb-test|split-later|fixture exception\n'
    printf 'crates/vb_expr/src/duplicate.rs|tester|vb-test|split-later|fixture exception\n'
  } > "$repo/.config/source-length-exceptions.txt"
  track_repo "$repo"
  output="$(expect_gate_failure "$repo" 5 99 'duplicate source ledger')"
  assert_contains "$output" 'duplicate exception for crates/vb_expr/src/duplicate.rs' 'duplicate source ledger failure'
}

test_stale_source_ledger_fails() {
  local repo
  local output
  repo="$(make_repo)"
  mkdir -p "$repo/crates/vb_expr/src"
  printf 'pub fn small() {}\n' > "$repo/crates/vb_expr/src/small.rs"
  printf 'crates/vb_expr/src/small.rs|tester|vb-test|split-later|fixture exception\n' > "$repo/.config/source-length-exceptions.txt"
  track_repo "$repo"
  output="$(expect_gate_failure "$repo" 5 99 'stale source ledger')"
  assert_contains "$output" 'stale exception for crates/vb_expr/src/small.rs' 'stale source ledger failure'
}

test_invalid_source_ledger_path_fails() {
  local repo
  local output
  repo="$(make_repo)"
  printf '../outside.rs|tester|vb-test|split-later|fixture exception\n' > "$repo/.config/source-length-exceptions.txt"
  track_repo "$repo"
  output="$(expect_gate_failure "$repo" 5 99 'invalid source ledger path')"
  assert_contains "$output" 'invalid path' 'invalid source ledger path failure'
}

test_valid_hot_ledger_allows_current_violation() {
  local repo
  repo="$(make_repo)"
  write_hot_function "$repo/crates/vb_runtime/src/ledgered_hot.rs" 'ledgered_hot'
  printf 'crates/vb_runtime/src/ledgered_hot.rs|1|tester|vb-test|split-later|fixture exception\n' > "$repo/.config/hot-function-length-exceptions.txt"
  track_repo "$repo"
  run_gate "$repo" 99 3 >/dev/null
}

test_duplicate_hot_ledger_fails() {
  local repo
  local output
  repo="$(make_repo)"
  write_hot_function "$repo/crates/vb_runtime/src/duplicate_hot.rs" 'duplicate_hot'
  {
    printf 'crates/vb_runtime/src/duplicate_hot.rs|1|tester|vb-test|split-later|fixture exception\n'
    printf 'crates/vb_runtime/src/duplicate_hot.rs|1|tester|vb-test|split-later|fixture exception\n'
  } > "$repo/.config/hot-function-length-exceptions.txt"
  track_repo "$repo"
  output="$(expect_gate_failure "$repo" 99 3 'duplicate hot ledger')"
  assert_contains "$output" 'duplicate exception for crates/vb_runtime/src/duplicate_hot.rs:1' 'duplicate hot ledger failure'
}

test_stale_hot_ledger_fails() {
  local repo
  local output
  repo="$(make_repo)"
  mkdir -p "$repo/crates/vb_runtime/src"
  printf 'pub fn short_hot() {}\n' > "$repo/crates/vb_runtime/src/short_hot.rs"
  printf 'crates/vb_runtime/src/short_hot.rs|1|tester|vb-test|split-later|fixture exception\n' > "$repo/.config/hot-function-length-exceptions.txt"
  track_repo "$repo"
  output="$(expect_gate_failure "$repo" 99 3 'stale hot ledger')"
  assert_contains "$output" 'stale or non-matching hot-function exception' 'stale hot ledger failure'
}

test_invalid_hot_ledger_start_fails() {
  local repo
  local output
  repo="$(make_repo)"
  mkdir -p "$repo/crates/vb_runtime/src"
  printf 'pub fn short_hot() {}\n' > "$repo/crates/vb_runtime/src/short_hot.rs"
  printf 'crates/vb_runtime/src/short_hot.rs|zero|tester|vb-test|split-later|fixture exception\n' > "$repo/.config/hot-function-length-exceptions.txt"
  track_repo "$repo"
  output="$(expect_gate_failure "$repo" 99 3 'invalid hot ledger start')"
  assert_contains "$output" 'start line is not a positive integer' 'invalid hot ledger start failure'
}

test_malformed_hot_ledger_fails() {
  local repo
  local output
  repo="$(make_repo)"
  printf 'crates/vb_runtime/src/file.rs|too-few-fields\n' > "$repo/.config/hot-function-length-exceptions.txt"
  track_repo "$repo"
  output="$(expect_gate_failure "$repo" 99 3 'malformed hot ledger')"
  assert_contains "$output" 'malformed row' 'malformed hot ledger failure'
}

test_invalid_hot_ledger_path_fails() {
  local repo
  local output
  repo="$(make_repo)"
  printf '../outside.rs|1|tester|vb-test|split-later|fixture exception\n' > "$repo/.config/hot-function-length-exceptions.txt"
  track_repo "$repo"
  output="$(expect_gate_failure "$repo" 99 3 'invalid hot ledger path')"
  assert_contains "$output" 'invalid path' 'invalid hot ledger path failure'
}

test_hot_ledger_non_hot_scope_fails() {
  local repo
  local output
  repo="$(make_repo)"
  mkdir -p "$repo/crates/vb_expr/src"
  printf 'pub fn non_hot() {}\n' > "$repo/crates/vb_expr/src/non_hot.rs"
  printf 'crates/vb_expr/src/non_hot.rs|1|tester|vb-test|split-later|fixture exception\n' > "$repo/.config/hot-function-length-exceptions.txt"
  track_repo "$repo"
  output="$(expect_gate_failure "$repo" 99 3 'hot ledger non-hot scope')"
  assert_contains "$output" 'path is not in the hot-function scan scope' 'hot ledger non-hot scope failure'
}

test_braces_in_strings_and_comments_do_not_hide_next_hot_function() {
  local repo
  local output
  repo="$(make_repo)"
  mkdir -p "$repo/crates/vb_runtime/src"
  {
    printf 'pub fn string_brace_ok() {\n'
    printf '    let text = "{";\n'
    printf '    let value = text; // }\n'
    printf '}\n'
    printf 'pub fn too_long_after() {\n'
    printf '    let first = 1;\n'
    printf '    let second = 2;\n'
    printf '    let third = first + second;\n'
    printf '}\n'
  } > "$repo/crates/vb_runtime/src/brace_noise.rs"
  track_repo "$repo"
  output="$(expect_gate_failure "$repo" 99 3 'brace noise hot scan')"
  assert_contains "$output" 'crates/vb_runtime/src/brace_noise.rs:5 hot function has 4 logical lines' 'brace noise hot function failure'
}

test_gate_passes_on_compliant_files
test_gate_fails_on_over_limit_test_like_source
test_gate_fails_on_over_limit_arbitrary_first_party_source
test_gate_fails_on_over_limit_proof_source
test_gate_fails_on_hot_function_over_limit
test_valid_source_ledger_allows_over_limit_file
test_malformed_source_ledger_fails
test_duplicate_source_ledger_fails
test_stale_source_ledger_fails
test_invalid_source_ledger_path_fails
test_valid_hot_ledger_allows_current_violation
test_duplicate_hot_ledger_fails
test_stale_hot_ledger_fails
test_invalid_hot_ledger_start_fails
test_malformed_hot_ledger_fails
test_invalid_hot_ledger_path_fails
test_hot_ledger_non_hot_scope_fails
test_braces_in_strings_and_comments_do_not_hide_next_hot_function

printf 'check-source-length self-tests passed\n'
