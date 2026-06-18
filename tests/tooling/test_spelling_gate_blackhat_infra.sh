#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
source "$SCRIPT_DIR/spelling_gate_blackhat_lib.sh"

test_recursive_search_errors_fail_closed() {
  local scratch match_file
  scratch="$(new_scratch_repo "collect-error")"
  match_file="$scratch/docs/active.md"
  printf 'active spelling: %s\n' "$BAD_TOKEN" > "$match_file"
  run_gate_with_fake_grep "$scratch" "collect_error" "$match_file"
  assert_equal "recursive search error exit" "2" "$GATE_EXIT"
  assert_stdout_empty "recursive search error stdout"
  assert_contains "recursive search error stderr" \
    "injected recursive search failure" "$GATE_STDERR"
}

test_per_file_unreadable_search_errors_fail_closed() {
  local scratch match_file
  scratch="$(new_scratch_repo "line-error")"
  match_file="$scratch/docs/active.md"
  printf 'active spelling: %s\n' "$BAD_TOKEN" > "$match_file"
  run_gate_with_fake_grep "$scratch" "line_error" "$match_file"
  assert_equal "per-file search error exit" "2" "$GATE_EXIT"
  assert_stdout_empty "per-file search error stdout"
  assert_contains "per-file search error stderr" "Permission denied" "$GATE_STDERR"
}

test_moon_task_inputs_cover_scanner_universe() {
  load_moon_patterns
  local -a sentinels=(
    "AGENTS.md" "moon-rust-verification.yml"
    "scripts/spellgate-cache-probe.sh" ".moon/tasks/spellgate-cache-probe.yml"
    "fixtures/check-spelling-gate/cache-probe.md" "design/spellgate-cache-probe.md"
    "verification/spellgate-cache-probe.md" "reference/spellgate-cache-probe.md"
    "to-fix/spellgate-cache-probe.md" "xtask/spellgate-cache-probe.rs"
  )
  local -a missing=()
  local sentinel pattern covered
  for sentinel in "${sentinels[@]}"; do
    covered=0
    for pattern in "${EXPANDED_INPUTS[@]}"; do
      if pattern_matches_path "$pattern" "$sentinel"; then covered=1; fi
    done
    if [[ "$covered" -ne 1 ]]; then missing+=("$sentinel"); fi
  done
  if [[ "${#missing[@]}" -ne 0 ]]; then
    fail "moon scanner-universe inputs" "missing coverage for: ${missing[*]}"
  fi
}

default_tests=(
  test_recursive_search_errors_fail_closed
  test_per_file_unreadable_search_errors_fail_closed
  test_moon_task_inputs_cover_scanner_universe
)
if [[ "$#" -gt 0 ]]; then default_tests=("$@"); fi
run_test_names "${default_tests[@]}"
