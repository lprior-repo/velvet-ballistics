#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd -P)"
GATE_SOURCE="$ROOT/scripts/check-spelling-gate.sh"
MOON_TASKS="$ROOT/.moon/tasks/all.yml"
TOKEN_HEAD="velvet"
TOKEN_TAIL="ballistics"
BAD_TOKEN="${TOKEN_HEAD}-${TOKEN_TAIL}"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/spellgate-blackhat.XXXXXX")"
cleanup_spellgate_blackhat() { rm -rf "$TMP_ROOT"; }
trap cleanup_spellgate_blackhat EXIT INT TERM

GATE_EXIT=0
GATE_STDOUT=""
GATE_STDERR=""

fail() {
  local label="$1" detail="$2"
  printf 'AssertionFailed: %s: %s\n' "$label" "$detail" >&2
  return 1
}

assert_equal() {
  local label="$1" expected="$2" actual="$3"
  if [[ "$expected" != "$actual" ]]; then
    fail "$label" "expected [$expected], got [$actual]"
  fi
}

assert_not_equal() {
  local label="$1" left="$2" right="$3"
  if [[ "$left" == "$right" ]]; then
    fail "$label" "values must differ but both were [$left]"
  fi
}

assert_contains() {
  local label="$1" needle="$2" haystack="$3"
  case "$haystack" in
    *"$needle"*) ;;
    *)
      printf 'Captured output for %s:\n%s\n' "$label" "$haystack" >&2
      fail "$label" "missing substring: $needle"
      ;;
  esac
}

assert_stdout_empty() {
  local label="$1"
  if [[ -n "$GATE_STDOUT" ]]; then
    fail "$label" "expected empty stdout, got: $GATE_STDOUT"
  fi
}

new_scratch_repo() {
  local label="$1" scratch
  scratch="$TMP_ROOT/$label"
  rm -rf "$scratch"
  mkdir -p "$scratch/scripts" "$scratch/docs" "$scratch/src"
  cp -f "$GATE_SOURCE" "$scratch/scripts/check-spelling-gate.sh"
  printf '%s' "$scratch"
}

capture_gate_result() {
  local stdout_file="$1" stderr_file="$2"
  GATE_STDOUT="$(<"$stdout_file")"
  GATE_STDERR="$(<"$stderr_file")"
  rm -f "$stdout_file" "$stderr_file"
}

run_gate_in_dir() {
  local workdir="$1" stdout_file stderr_file
  stdout_file="$(mktemp "$TMP_ROOT/stdout.XXXXXX")"
  stderr_file="$(mktemp "$TMP_ROOT/stderr.XXXXXX")"
  set +e
  (cd "$workdir" && bash scripts/check-spelling-gate.sh >"$stdout_file" 2>"$stderr_file")
  GATE_EXIT=$?
  set -e
  capture_gate_result "$stdout_file" "$stderr_file"
}

write_fake_grep() {
  local fakebin="$1"
  mkdir -p "$fakebin"
  cat > "$fakebin/grep" <<'FAKE_GREP'
#!/usr/bin/env bash
set -euo pipefail
contains_arg() {
  local wanted="$1" arg
  shift
  for arg in "$@"; do
    if [[ "$arg" == "$wanted" ]]; then return 0; fi
  done
  return 1
}
if [[ "${FAKE_GREP_MODE:-}" == "collect_error" ]] && contains_arg "-rl" "$@"; then
  printf 'grep: injected recursive search failure\n' >&2
  exit 2
fi
if [[ "${FAKE_GREP_MODE:-}" == "line_error" ]] && contains_arg "-rl" "$@"; then
  printf '%s\n' "${FAKE_GREP_MATCH_FILE:?missing match file}"
  exit 0
fi
if [[ "${FAKE_GREP_MODE:-}" == "line_error" ]] && contains_arg "-n" "$@"; then
  printf 'grep: %s: Permission denied\n' "${FAKE_GREP_MATCH_FILE:?missing match file}" >&2
  exit 2
fi
exec /usr/bin/grep "$@"
FAKE_GREP
  chmod +x "$fakebin/grep"
}

run_gate_with_fake_grep() {
  local workdir="$1" mode="$2" match_file="$3" fakebin
  fakebin="$workdir/fakebin"
  local stdout_file stderr_file
  write_fake_grep "$fakebin"
  stdout_file="$(mktemp "$TMP_ROOT/stdout.XXXXXX")"
  stderr_file="$(mktemp "$TMP_ROOT/stderr.XXXXXX")"
  set +e
  (
    cd "$workdir" && env MOON_TASK_ID=blackhat PATH="$fakebin:$PATH" \
      FAKE_GREP_MODE="$mode" FAKE_GREP_MATCH_FILE="$match_file" \
      bash scripts/check-spelling-gate.sh >"$stdout_file" 2>"$stderr_file"
  )
  GATE_EXIT=$?
  set -e
  capture_gate_result "$stdout_file" "$stderr_file"
}

count_violation_lines() {
  local output="$1" count=0 line
  while IFS= read -r line || [[ -n "$line" ]]; do
    case "$line" in VIOLATION:*) count=$((count + 1)) ;; esac
  done <<< "$output"
  printf '%s' "$count"
}

assert_violation_location() {
  local label="$1" path="$2" line_number="$3"
  assert_contains "$label" \
    "VIOLATION: $path:$line_number: wrong spelling '$BAD_TOKEN'" \
    "$GATE_STDERR"
}

strip_quotes() {
  local value="$1"
  value="${value#\'}"; value="${value%\'}"
  value="${value#\"}"; value="${value%\"}"
  printf '%s' "$value"
}

declare -a FILE_GROUP_ROWS=()
declare -a TASK_INPUTS=()
declare -a EXPANDED_INPUTS=()

load_moon_patterns() {
  FILE_GROUP_ROWS=(); TASK_INPUTS=(); EXPANDED_INPUTS=()
  local in_groups=0 current_group="" in_task=0 in_inputs=0 line item group row
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" == "fileGroups:" ]]; then in_groups=1; continue; fi
    if [[ "$line" == "tasks:" ]]; then in_groups=0; fi
    if [[ "$in_groups" -eq 1 ]]; then
      case "$line" in
        "  "*":") current_group="${line#  }"; current_group="${current_group%:}" ;;
        "    - "*) FILE_GROUP_ROWS+=("$current_group|$(strip_quotes "${line#    - }")") ;;
      esac
    fi
    if [[ "$line" == "  check-spelling-gate:" ]]; then in_task=1; continue; fi
    if [[ "$in_task" -eq 1 ]]; then
      if [[ "$line" == "  "*":" && "$line" != "    "* ]]; then in_task=0; fi
      case "$line" in
        "    inputs:") in_inputs=1 ;;
        "    options:"*) in_inputs=0 ;;
        "      - "*)
          if [[ "$in_inputs" -eq 1 ]]; then
            TASK_INPUTS+=("$(strip_quotes "${line#      - }")")
          fi
          ;;
      esac
    fi
  done < "$MOON_TASKS"
  for item in "${TASK_INPUTS[@]}"; do
    case "$item" in
      "@globs("*")")
        group="${item#@globs(}"; group="${group%)}"
        for row in "${FILE_GROUP_ROWS[@]}"; do
          if [[ "${row%%|*}" == "$group" ]]; then EXPANDED_INPUTS+=("${row#*|}"); fi
        done
        ;;
      *) EXPANDED_INPUTS+=("$item") ;;
    esac
  done
}

pattern_matches_path() {
  local pattern="$1" path="$2" prefix ext
  if [[ "$pattern" == "**/*."* ]]; then
    ext="${pattern##*.}"; [[ "$path" == *."$ext" ]]
  elif [[ "$pattern" == *"/**/*" ]]; then
    prefix="${pattern%%/**/*}"; [[ "$path" == "$prefix/"* ]]
  elif [[ "$pattern" == *"/**" ]]; then
    prefix="${pattern%%/**}"; [[ "$path" == "$prefix/"* ]]
  elif [[ "$pattern" == *"*"* ]]; then
    [[ "$path" == $pattern ]]
  else
    [[ "$path" == "$pattern" ]]
  fi
}

run_test_names() {
  local failed=0 test_name status
  for test_name in "$@"; do
    printf '%s\n' "--- running $test_name ---"
    set +e
    ( set +e; "$test_name" )
    status=$?
    set -e
    if [[ "$status" -eq 0 ]]; then
      printf 'PASS: %s\n' "$test_name"
    else
      printf 'FAIL: %s\n' "$test_name" >&2
      failed=$((failed + 1))
    fi
  done
  if [[ "$failed" -ne 0 ]]; then
    printf '=== %s test(s) failed ===\n' "$failed" >&2
    return 1
  fi
  printf '=== all %s tests passed ===\n' "$#"
}
