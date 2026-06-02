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
  local repo="$1"

  git -C "$repo" add .
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
  mkdir -p "$repo/crates/vb_cli/src/args/tests"
  for line in 1 2 3 4 5 6; do
    printf '// line %s\n' "$line"
  done > "$repo/crates/vb_cli/src/args/tests/oversize.rs"
  track_repo "$repo"

  if output="$(run_gate "$repo" 5 99 2>&1)"; then
    printf 'Expected source-length gate to reject over-limit test-like source\n' >&2
    return 1
  fi
  assert_contains "$output" 'crates/vb_cli/src/args/tests/oversize.rs has 6 physical lines' 'over-limit test-like source failure'
}

test_gate_fails_on_hot_function_over_limit() {
  local repo
  local output

  repo="$(make_repo)"
  mkdir -p "$repo/crates/vb_runtime/src"
  {
    printf 'pub fn too_long() {\n'
    printf '    let first = 1;\n'
    printf '    let second = 2;\n'
    printf '    let third = first + second;\n'
    printf '}\n'
  } > "$repo/crates/vb_runtime/src/long.rs"
  track_repo "$repo"

  if output="$(run_gate "$repo" 99 3 2>&1)"; then
    printf 'Expected source-length gate to reject over-limit hot function\n' >&2
    return 1
  fi
  assert_contains "$output" 'crates/vb_runtime/src/long.rs:1 hot function has 4 logical lines' 'hot function failure'
}

test_gate_passes_on_compliant_files
test_gate_fails_on_over_limit_test_like_source
test_gate_fails_on_hot_function_over_limit

printf 'check-source-length self-tests passed\n'
