#!/usr/bin/env bash
# test-check-spelling-gate.sh
# Bash integration tests for the spelling allowlist CI gate.
# Test cases: 3 named, matching the tier-a-0-003 contract.
#
# Exit 0: all selected tests PASS
# Exit 1: one or more selected tests FAIL
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

GATE_SOURCE="$ROOT/scripts/check-spelling-gate.sh"
FIXTURES_DIR="$ROOT/fixtures/check-spelling-gate"
MOON_ROOT="$ROOT/.moon.yml"
MOON_TASKS="$ROOT/.moon/tasks/all.yml"

TOKEN_HEAD="velvet"
TOKEN_TAIL="ballistics"
BAD_TOKEN="${TOKEN_HEAD}-${TOKEN_TAIL}"
CANONICAL_TOKEN="velvet_ballistics"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/spellgate-tests.XXXXXX")"
MOON_PROBE_PATH="$ROOT/docs/__spellgate_moon_probe.md"
cleanup() {
  rm -f "$MOON_PROBE_PATH"
  rm -rf "$TMP_ROOT"
}
trap cleanup EXIT INT TERM

GATE_EXIT=0
GATE_STDOUT=""
GATE_STDERR=""
MOON_EXIT=0
MOON_STDOUT=""
MOON_STDERR=""

fail() {
  local label="$1"
  local detail="$2"
  printf 'AssertionFailed: %s: %s\n' "$label" "$detail" >&2
  exit 1
}

assert_file_exists() {
  local label="$1"
  local path="$2"
  if [[ ! -f "$path" ]]; then
    fail "$label" "missing file: $path"
  fi
}

assert_exit_code() {
  local label="$1"
  local expected="$2"
  local actual="$3"
  if [[ "$expected" != "$actual" ]]; then
    printf 'Captured stderr for %s:\n%s\n' "$label" "$GATE_STDERR" >&2
    fail "$label" "expected exit $expected, got $actual"
  fi
}

assert_stdout_empty() {
  local label="$1"
  if [[ -n "$GATE_STDOUT" ]]; then
    fail "$label" "expected empty stdout, got: $GATE_STDOUT"
  fi
}

assert_contains() {
  local label="$1"
  local needle="$2"
  local haystack="$3"
  case "$haystack" in
    *"$needle"*) ;;
    *)
      printf 'Captured output for %s:\n%s\n' "$label" "$haystack" >&2
      fail "$label" "missing substring: $needle"
      ;;
  esac
}

assert_omits() {
  local label="$1"
  local needle="$2"
  local haystack="$3"
  case "$haystack" in
    *"$needle"*)
      printf 'Captured output for %s:\n%s\n' "$label" "$haystack" >&2
      fail "$label" "unexpected substring: $needle"
      ;;
  esac
}

assert_equal() {
  local label="$1"
  local expected="$2"
  local actual="$3"
  if [[ "$expected" != "$actual" ]]; then
    fail "$label" "expected [$expected], got [$actual]"
  fi
}

first_line_of() {
  local output="$1"
  local line=""
  IFS= read -r line <<< "$output" || true
  printf '%s' "$line"
}

expected_banner() {
  printf '=== Spelling Gate: %s vs %s ===' "$BAD_TOKEN" "$CANONICAL_TOKEN"
}

expected_summary() {
  local count="$1"
  printf '=== Spelling Gate complete: %s violations ===' "$count"
}

expected_violation_line() {
  local path="$1"
  local line_number="$2"
  printf "VIOLATION: %s:%s: wrong spelling '%s' (use '%s')" \
    "$path" "$line_number" "$BAD_TOKEN" "$CANONICAL_TOKEN"
}

expected_failure_hint_block() {
  printf '\n'
  printf "Hint: Replace active code identifiers with '%s' or document an exact allowlisted artifact.\n" "$CANONICAL_TOKEN"
  printf 'HZ-DRIFT-001: product/package prose still needs a canonical naming repair before claiming global closure.\n'
  printf 'Allowlisted path patterns (excluded entirely):\n'
  printf '  - .beads/ (bead artifacts and CI output)\n'
  printf '  - .jj/ (JJ internal state)\n'
  printf '  - .evidence/ and evidence/ at workspace root only (evidence artifacts)\n'
  printf '  - target/ (build artifacts)\n'
  printf '  - tests/ and benches/ (test/bench clippy is not strict)\n'
  printf '  - %s-MASTER.md (master contract file)\n' "$BAD_TOKEN"
  printf 'Allowlisted content patterns:\n'
  printf '  - %s-MASTER.md (reference to master file)\n' "$BAD_TOKEN"
  printf '  - /home/.*/%s/ (source checkout path, migration artifact)\n' "$BAD_TOKEN"
  printf '  - FORBIDDEN_FEATURE_NAMES blocks %s (spelling used as forbid-tag)\n' "$BAD_TOKEN"
  printf "  - '%s' is invalid (rule statement)" "$BAD_TOKEN"
}

assert_stderr_banner_exact() {
  local label="$1"
  assert_equal "$label" "$(expected_banner)" "$(first_line_of "$GATE_STDERR")"
}

assert_summary_exact() {
  local label="$1"
  local count="$2"
  assert_contains "$label" "$(expected_summary "$count")" "$GATE_STDERR"
}

assert_exact_violation_line() {
  local label="$1"
  local path="$2"
  local line_number="$3"
  assert_contains "$label" "$(expected_violation_line "$path" "$line_number")" "$GATE_STDERR"
}

assert_failure_hint_block_exact() {
  local label="$1"
  assert_contains "$label" "$(expected_failure_hint_block)" "$GATE_STDERR"
}

assert_no_failure_hint() {
  local label="$1"
  assert_omits "$label" "Hint:" "$GATE_STDERR"
  assert_omits "$label" "Allowlisted path patterns" "$GATE_STDERR"
  assert_omits "$label" "Allowlisted content patterns" "$GATE_STDERR"
}

count_violation_lines() {
  local output="$1"
  local count=0
  local line
  while IFS= read -r line || [[ -n "$line" ]]; do
    case "$line" in
      VIOLATION:*) count=$((count + 1)) ;;
    esac
  done <<< "$output"
  printf '%s' "$count"
}

assert_violation_count() {
  local label="$1"
  local expected="$2"
  local actual
  actual="$(count_violation_lines "$GATE_STDERR")"
  assert_equal "$label" "$expected" "$actual"
}

write_candidate_file() {
  local path="$1"
  local label="$2"
  mkdir -p "$(dirname "$path")"
  {
    printf 'candidate %s header\n' "$label"
    printf 'candidate %s neutral line\n' "$label"
    printf 'candidate %s active spelling: %s\n' "$label" "$BAD_TOKEN"
  } > "$path"
}

write_single_violation_file() {
  local path="$1"
  mkdir -p "$(dirname "$path")"
  printf 'active spelling probe: %s\n' "$BAD_TOKEN" > "$path"
}

assert_all_candidate_extensions_are_scanned() {
  local scratch
  scratch="$(new_scratch_repo "candidate-extensions")"

  local -a candidate_paths=(
    "$scratch/src/candidate.rs"
    "$scratch/config/candidate.toml"
    "$scratch/config/candidate.yaml"
    "$scratch/config/candidate.yml"
    "$scratch/docs/candidate.md"
    "$scratch/scripts/candidate.sh"
    "$scratch/tools/candidate.py"
  )
  local -a candidate_labels=(
    "rs"
    "toml"
    "yaml"
    "yml"
    "md"
    "sh"
    "py"
  )

  local index
  for index in "${!candidate_paths[@]}"; do
    write_candidate_file "${candidate_paths[$index]}" "${candidate_labels[$index]}"
  done
  mkdir -p "$scratch/data"
  printf 'non-candidate extension must be invisible: %s\n' "$BAD_TOKEN" > \
    "$scratch/data/noncandidate.txt"

  run_gate_in_dir "$scratch"
  assert_exit_code "candidate extension exit" "1" "$GATE_EXIT"
  assert_stdout_empty "candidate extension stdout"
  assert_stderr_banner_exact "candidate extension banner"
  assert_violation_count "candidate extension violation count" "7"
  assert_summary_exact "candidate extension summary" "7"
  assert_failure_hint_block_exact "candidate extension hint"
  for index in "${!candidate_paths[@]}"; do
    assert_exact_violation_line \
      "candidate extension ${candidate_labels[$index]} exact violation" \
      "${candidate_paths[$index]}" \
      "3"
  done
  assert_omits "non-candidate extension omitted" "/data/noncandidate.txt:" "$GATE_STDERR"
}

assert_path_exclusion_globs_are_total() {
  local scratch
  scratch="$(new_scratch_repo "path-exclusion-globs")"

  local active_path="$scratch/docs/active.md"
  write_single_violation_file "$active_path"

  local -a excluded_paths=(
    "$scratch/.beads/probe.md"
    "$scratch/.jj/probe.md"
    "$scratch/.evidence/probe.md"
    "$scratch/evidence/probe.md"
    "$scratch/target/probe.md"
    "$scratch/target_nosccache/probe.md"
    "$scratch/target_debug_clean/probe.md"
    "$scratch/target_clean/probe.md"
    "$scratch/tests/probe.md"
    "$scratch/benches/probe.md"
    "$scratch/${BAD_TOKEN}-MASTER.md"
    "$scratch/BIG-ASS-TESTING-TO-FIX.md"
    "$scratch/src/naming_scan/probe.rs"
    "$scratch/src/name_tests.rs"
  )

  local -a active_docs_src_paths=(
    "$scratch/docs/.evidence/probe.md"
    "$scratch/src/.evidence/probe.rs"
    "$scratch/docs/evidence/probe.md"
    "$scratch/src/evidence/probe.rs"
    "$scratch/docs/vb-spelling/probe.md"
    "$scratch/src/vb-spelling/probe.rs"
    "$scratch/docs/femdation-vb-spelling/probe.md"
    "$scratch/src/femdation-vb-spelling/probe.rs"
    "$scratch/docs/go-skill-spelling/probe.md"
    "$scratch/src/go-skill-spelling/probe.rs"
    "$scratch/docs/holzman-workspace-spelling/probe.md"
    "$scratch/src/holzman-workspace-spelling/probe.rs"
    "$scratch/docs/pick5-spelling/probe.md"
    "$scratch/src/pick5-spelling/probe.rs"
  )

  local path
  for path in "${excluded_paths[@]}"; do
    write_single_violation_file "$path"
  done
  for path in "${active_docs_src_paths[@]}"; do
    write_single_violation_file "$path"
  done

  run_gate_in_dir "$scratch"
  assert_exit_code "path exclusion glob exit" "1" "$GATE_EXIT"
  assert_stdout_empty "path exclusion glob stdout"
  assert_stderr_banner_exact "path exclusion glob banner"
  assert_violation_count "path exclusion glob violation count" "15"
  assert_exact_violation_line "path exclusion active control" "$active_path" "1"
  for path in "${active_docs_src_paths[@]}"; do
    assert_exact_violation_line "path exclusion active docs/src" "$path" "1"
  done
  assert_summary_exact "path exclusion glob summary" "15"
  assert_failure_hint_block_exact "path exclusion glob hint"
  for path in "${excluded_paths[@]}"; do
    assert_omits "excluded path omitted" "$path:" "$GATE_STDERR"
  done
  assert_omits "gate script self-exclusion omitted" "/scripts/check-spelling-gate.sh:" "$GATE_STDERR"
}

run_gate_in_dir() {
  local workdir="$1"
  local stdout_file
  local stderr_file
  stdout_file="$(mktemp "$TMP_ROOT/stdout.XXXXXX")"
  stderr_file="$(mktemp "$TMP_ROOT/stderr.XXXXXX")"

  set +e
  (cd "$workdir" && bash scripts/check-spelling-gate.sh >"$stdout_file" 2>"$stderr_file")
  GATE_EXIT=$?
  set -e

  GATE_STDOUT="$(<"$stdout_file")"
  GATE_STDERR="$(<"$stderr_file")"
  rm -f "$stdout_file" "$stderr_file"
}

run_moon_spelling_gate_in_root() {
  local stdout_file
  local stderr_file
  stdout_file="$(mktemp "$TMP_ROOT/moon-stdout.XXXXXX")"
  stderr_file="$(mktemp "$TMP_ROOT/moon-stderr.XXXXXX")"

  set +e
  (
    cd "$ROOT" && \
      timeout 60s moon run :check-spelling-gate --force --cache off \
        >"$stdout_file" 2>"$stderr_file"
  )
  MOON_EXIT=$?
  set -e

  MOON_STDOUT="$(<"$stdout_file")"
  MOON_STDERR="$(<"$stderr_file")"
  rm -f "$stdout_file" "$stderr_file"
}

new_scratch_repo() {
  local label="$1"
  local scratch="$TMP_ROOT/$label"
  rm -rf "$scratch"
  mkdir -p "$scratch/scripts" "$scratch/docs"
  cp -f "$GATE_SOURCE" "$scratch/scripts/check-spelling-gate.sh"
  printf '%s' "$scratch"
}

assert_common_prerequisites() {
  assert_file_exists "gate script" "$GATE_SOURCE"
  assert_file_exists "positive fixture" "$FIXTURES_DIR/positive.md"
  assert_file_exists "negative fixture" "$FIXTURES_DIR/negative.md"
  assert_file_exists "allowlist fixture" "$FIXTURES_DIR/allowlist.md"
  assert_file_exists "inline-bypass fixture" "$FIXTURES_DIR/_inline-bypass.md"
}

assert_file_contains_literal() {
  local label="$1"
  local path="$2"
  local needle="$3"
  local line
  while IFS= read -r line || [[ -n "$line" ]]; do
    case "$line" in
      *"$needle"*) return 0 ;;
    esac
  done < "$path"
  fail "$label" "missing literal [$needle] in $path"
}

pipeline_item_line() {
  local item="$1"
  local line
  local number=0
  local single="  - '$item'"
  local double="  - \"$item\""
  local bare="  - $item"
  while IFS= read -r line || [[ -n "$line" ]]; do
    number=$((number + 1))
    case "$line" in
      "$single"|"$double"|"$bare")
        printf '%s' "$number"
        return 0
        ;;
    esac
  done < "$MOON_ROOT"
  return 1
}

pipeline_item_count() {
  local item="$1"
  local line
  local count=0
  local single="  - '$item'"
  local double="  - \"$item\""
  local bare="  - $item"
  while IFS= read -r line || [[ -n "$line" ]]; do
    case "$line" in
      "$single"|"$double"|"$bare")
        count=$((count + 1))
        ;;
    esac
  done < "$MOON_ROOT"
  printf '%s' "$count"
}

assert_moon_task_block_exact() {
  local in_task=0
  local in_inputs=0
  local line
  local command_line=""
  local command_count=0
  local run_ci_count=0
  local -a input_lines=()

  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" == "  check-spelling-gate:" ]]; then
      in_task=1
      continue
    fi
    if [[ "$in_task" -eq 1 ]]; then
      if [[ "$line" == "  "*":" && "$line" != "    "* ]]; then
        break
      fi
      case "$line" in
        "    command: "*)
          command_line="$line"
          command_count=$((command_count + 1))
          ;;
        "    inputs:")
          in_inputs=1
          ;;
        "    options:")
          in_inputs=0
          ;;
        "      - "*)
          if [[ "$in_inputs" -eq 1 ]]; then
            input_lines+=("$line")
          fi
          ;;
        "      runInCI: true")
          run_ci_count=$((run_ci_count + 1))
          ;;
      esac
    fi
  done < "$MOON_TASKS"

  if [[ "$in_task" -ne 1 ]]; then
    fail "moon task exact wiring" "missing check-spelling-gate task block"
  fi
  assert_equal "moon task command count" "1" "$command_count"
  assert_equal "moon task command exact" \
    "    command: 'bash scripts/check-spelling-gate.sh'" \
    "$command_line"
  assert_equal "moon task input count" "3" "${#input_lines[@]}"

  local -a expected_inputs=(
    "      - 'scripts/check-spelling-gate.sh'"
    "      - '@globs(spellingGateUniverse)'"
    "      - '.moon/tasks/all.yml'"
  )
  local index
  for index in "${!expected_inputs[@]}"; do
    assert_equal "moon task input $index exact" \
      "${expected_inputs[$index]}" \
      "${input_lines[$index]}"
  done
  assert_equal "moon task runInCI exact" "1" "$run_ci_count"
}

assert_moon_run_propagates_gate_exit_for_active_probe() {
  if ! command -v moon >/dev/null 2>&1; then
    fail "moon executable" "moon command is unavailable; State 11 cannot prove task wiring"
  fi

  rm -f "$MOON_PROBE_PATH"
  mkdir -p "$(dirname "$MOON_PROBE_PATH")"
  printf 'moon active spelling probe: %s\n' "$BAD_TOKEN" > "$MOON_PROBE_PATH"

  run_moon_spelling_gate_in_root
  rm -f "$MOON_PROBE_PATH"

  if [[ "$MOON_EXIT" != "1" ]]; then
    printf 'Captured moon stdout:\n%s\n' "$MOON_STDOUT" >&2
    printf 'Captured moon stderr:\n%s\n' "$MOON_STDERR" >&2
    fail "moon run exit propagation" \
      "expected moon run :check-spelling-gate --force --cache off to exit 1 for active probe, got $MOON_EXIT"
  fi

  assert_contains "moon run task target" \
    "${BAD_TOKEN}:check-spelling-gate" \
    "$MOON_STDOUT"
  assert_contains "moon run banner" "$(expected_banner)" "$MOON_STDERR"
  assert_contains "moon run active probe exact violation" \
    "$(expected_violation_line "$MOON_PROBE_PATH" "1")" \
    "$MOON_STDERR"
  assert_contains "moon run failure hint" \
    "$(expected_failure_hint_block)" \
    "$MOON_STDERR"
}

assert_moon_ci_orders_spelling_gate() {
  assert_file_exists "moon root" "$MOON_ROOT"
  assert_file_exists "moon task file" "$MOON_TASKS"
  assert_moon_task_block_exact
  assert_moon_run_propagates_gate_exit_for_active_probe

  local source_line
  local spelling_line
  local test_line
  local spelling_count
  source_line="$(pipeline_item_line "source-length" || true)"
  spelling_line="$(pipeline_item_line "check-spelling-gate" || true)"
  test_line="$(pipeline_item_line "test" || true)"
  spelling_count="$(pipeline_item_count "check-spelling-gate")"

  if [[ "$spelling_count" != "1" ]]; then
    fail "moon ci pipeline" "expected exactly one check-spelling-gate entry, got $spelling_count"
  fi
  if [[ -z "$source_line" || -z "$test_line" ]]; then
    fail "moon ci pipeline" "missing source-length or test anchor"
  fi
  if (( spelling_line <= source_line )); then
    fail "moon ci pipeline" "spelling gate must run after source-length"
  fi
  if (( spelling_line >= test_line )); then
    fail "moon ci pipeline" "spelling gate must run before test"
  fi
}

test_spelling_gate_rejects_velvet_ballistics() {
  assert_common_prerequisites

  local scratch
  scratch="$(new_scratch_repo "negative-one")"
  cp -f "$FIXTURES_DIR/negative.md" "$scratch/docs/negative.md"

  run_gate_in_dir "$scratch"
  assert_exit_code "negative fixture exit" "1" "$GATE_EXIT"
  assert_stdout_empty "negative fixture stdout"
  assert_stderr_banner_exact "negative fixture banner"
  assert_violation_count "negative fixture violation count" "1"
  assert_exact_violation_line "negative fixture exact violation" \
    "$scratch/docs/negative.md" \
    "5"
  assert_summary_exact "negative fixture summary" "1"
  assert_failure_hint_block_exact "negative fixture hint"

  local many
  many="$(new_scratch_repo "negative-ten")"
  local n
  for n in 1 2 3 4 5 6 7 8 9 10; do
    printf 'line %02d has %s\n' "$n" "$BAD_TOKEN"
  done > "$many/docs/negative-ten.md"

  run_gate_in_dir "$many"
  assert_exit_code "negative ten exit" "1" "$GATE_EXIT"
  assert_stdout_empty "negative ten stdout"
  assert_stderr_banner_exact "negative ten banner"
  assert_violation_count "negative ten violation count" "10"
  for n in 1 2 3 4 5 6 7 8 9 10; do
    assert_exact_violation_line "negative ten exact violation $n" \
      "$many/docs/negative-ten.md" \
      "$n"
  done
  assert_summary_exact "negative ten summary" "10"
  assert_failure_hint_block_exact "negative ten hint"
}

test_spelling_gate_passes_on_allowlisted() {
  assert_common_prerequisites

  local scratch
  scratch="$(new_scratch_repo "positive")"
  cp -f "$FIXTURES_DIR/positive.md" "$scratch/docs/positive.md"

  run_gate_in_dir "$scratch"
  local first_stderr="$GATE_STDERR"
  assert_exit_code "positive fixture exit" "0" "$GATE_EXIT"
  assert_stdout_empty "positive fixture stdout"
  assert_stderr_banner_exact "positive fixture banner"
  assert_violation_count "positive fixture violation count" "0"
  assert_summary_exact "positive summary" "0"
  assert_omits "positive violations" "VIOLATION:" "$GATE_STDERR"
  assert_no_failure_hint "positive fixture no hint"

  run_gate_in_dir "$scratch"
  assert_exit_code "positive fixture rerun exit" "0" "$GATE_EXIT"
  assert_stdout_empty "positive fixture rerun stdout"
  assert_equal "positive idempotent stderr" "$first_stderr" "$GATE_STDERR"
  assert_all_candidate_extensions_are_scanned
}

test_moon_ci_spelling_dependency_correctly_ordered() {
  assert_common_prerequisites

  local scratch
  scratch="$(new_scratch_repo "allowlist")"
  cp -f "$FIXTURES_DIR/allowlist.md" "$scratch/docs/allowlist.md"
  mkdir -p "$scratch/.beads/_stage" "$scratch/target/_stage"
  printf 'path-excluded beads probe: %s\n' "$BAD_TOKEN" > "$scratch/.beads/_stage/under-beads.md"
  printf 'path-excluded target probe: %s\n' "$BAD_TOKEN" > "$scratch/target/_stage/under-target.md"

  run_gate_in_dir "$scratch"
  assert_exit_code "allowlist fixture exit" "0" "$GATE_EXIT"
  assert_stdout_empty "allowlist fixture stdout"
  assert_stderr_banner_exact "allowlist fixture banner"
  assert_violation_count "allowlist fixture violation count" "0"
  assert_summary_exact "allowlist summary" "0"
  assert_omits "allowlist violations" "VIOLATION:" "$GATE_STDERR"
  assert_no_failure_hint "allowlist fixture no hint"
  assert_path_exclusion_globs_are_total

  cp -f "$FIXTURES_DIR/_inline-bypass.md" "$scratch/docs/inline-bypass.md"
  run_gate_in_dir "$scratch"
  assert_exit_code "inline-bypass exit" "1" "$GATE_EXIT"
  assert_stdout_empty "inline-bypass stdout"
  assert_stderr_banner_exact "inline-bypass banner"
  assert_violation_count "inline-bypass violation count" "1"
  assert_exact_violation_line "inline-bypass exact violation" \
    "$scratch/docs/inline-bypass.md" \
    "5"
  assert_summary_exact "inline-bypass summary" "1"
  assert_failure_hint_block_exact "inline-bypass hint"

  assert_moon_ci_orders_spelling_gate
}

declare -a default_test_names=(
  "test_spelling_gate_rejects_velvet_ballistics"
  "test_spelling_gate_passes_on_allowlisted"
  "test_moon_ci_spelling_dependency_correctly_ordered"
)

declare -a selected_test_names=()
if [[ "$#" -gt 0 ]]; then
  selected_test_names=("$@")
else
  selected_test_names=("${default_test_names[@]}")
fi

failed=0
for tname in "${selected_test_names[@]}"; do
  if ! declare -F "$tname" >/dev/null; then
    printf 'FAIL: unknown test %s\n' "$tname" >&2
    failed=$((failed + 1))
    continue
  fi

  printf '%s\n' "--- running $tname ---"
  set +e
  ( "$tname" )
  status=$?
  set -e
  if [[ "$status" -eq 0 ]]; then
    printf 'PASS: %s\n' "$tname"
  else
    printf 'FAIL: %s\n' "$tname" >&2
    failed=$((failed + 1))
  fi
done

if [[ "$failed" -gt 0 ]]; then
  printf '=== %s test(s) failed ===\n' "$failed" >&2
  exit 1
fi

printf '=== all %s tests passed ===\n' "${#selected_test_names[@]}"
exit 0
