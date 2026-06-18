#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
source "$SCRIPT_DIR/spelling_gate_blackhat_lib.sh"

test_content_allowlist_same_line_extra_token_rejected() {
  local scratch target n
  scratch="$(new_scratch_repo "content-same-line")"
  target="$scratch/docs/content-same-line-bypass.md"
  cat > "$target" <<EOF_CONTENT
master reference plus active token: ${BAD_TOKEN}-MASTER.md extra ${BAD_TOKEN}
checkout path plus active token: /home/lewis/src/${BAD_TOKEN}/ extra ${BAD_TOKEN}
forbid tag plus active token: FORBIDDEN_FEATURE_NAMES extra ${BAD_TOKEN}
rule statement plus active token: \`${BAD_TOKEN}\` is invalid extra ${BAD_TOKEN}
dolt URL plus active token: https://doltremoteapi.dolthub.com/priorlewis43/${BAD_TOKEN} extra ${BAD_TOKEN}
legacy version plus active token: ${BAD_TOKEN}/v2 extra ${BAD_TOKEN}
EOF_CONTENT
  run_gate_in_dir "$scratch"
  assert_equal "content same-line exit" "1" "$GATE_EXIT"
  assert_stdout_empty "content same-line stdout"
  assert_equal "content same-line count" "6" "$(count_violation_lines "$GATE_STDERR")"
  assert_contains "content same-line summary" \
    "=== Spelling Gate complete: 6 violations ===" "$GATE_STDERR"
  for n in 1 2 3 4 5 6; do
    assert_violation_location "content same-line violation $n" "$target" "$n"
  done
}

test_broad_path_allowlist_active_code_rejected() {
  local scratch path
  scratch="$(new_scratch_repo "path-bypass")"
  local -a paths=(
    "$scratch/docs/final-active.md"
    "$scratch/docs/proof-repair-active.md"
    "$scratch/docs/black-hat-review-active.md"
    "$scratch/src/final-active.rs"
    "$scratch/src/proof-repair-active.rs"
    "$scratch/src/black-hat-review-active.rs"
  )
  for path in "${paths[@]}"; do
    mkdir -p "$(dirname "$path")"
    printf 'active spelling must be scanned: %s\n' "$BAD_TOKEN" > "$path"
  done
  run_gate_in_dir "$scratch"
  assert_equal "path bypass exit" "1" "$GATE_EXIT"
  assert_stdout_empty "path bypass stdout"
  assert_equal "path bypass count" "6" "$(count_violation_lines "$GATE_STDERR")"
  assert_contains "path bypass summary" \
    "=== Spelling Gate complete: 6 violations ===" "$GATE_STDERR"
  for path in "${paths[@]}"; do
    assert_violation_location "path bypass violation" "$path" "1"
  done
}

test_remaining_overbroad_path_allowlists_active_docs_and_src_rejected() {
  local scratch path
  scratch="$(new_scratch_repo "remaining-path-bypass")"
  local -a paths=(
    "$scratch/docs/.evidence/active.md"
    "$scratch/src/.evidence/active.rs"
    "$scratch/docs/evidence/active.md"
    "$scratch/src/evidence/active.rs"
    "$scratch/docs/vb-active/active.md"
    "$scratch/src/vb-active/active.rs"
    "$scratch/docs/femdation-vb-active/active.md"
    "$scratch/src/femdation-vb-active/active.rs"
    "$scratch/docs/go-skill-active/active.md"
    "$scratch/src/go-skill-active/active.rs"
    "$scratch/docs/holzman-workspace-active/active.md"
    "$scratch/src/holzman-workspace-active/active.rs"
    "$scratch/docs/pick5-active/active.md"
    "$scratch/src/pick5-active/active.rs"
  )
  for path in "${paths[@]}"; do
    mkdir -p "$(dirname "$path")"
    printf 'active docs/src spelling must be scanned: %s\n' "$BAD_TOKEN" > "$path"
  done
  run_gate_in_dir "$scratch"
  assert_equal "remaining path bypass exit" "1" "$GATE_EXIT"
  assert_stdout_empty "remaining path bypass stdout"
  assert_equal "remaining path bypass count" "14" "$(count_violation_lines "$GATE_STDERR")"
  assert_contains "remaining path bypass summary" \
    "=== Spelling Gate complete: 14 violations ===" "$GATE_STDERR"
  for path in "${paths[@]}"; do
    assert_violation_location "remaining path bypass violation" "$path" "1"
  done
}

test_forbidden_and_canonical_diagnostic_bytes_are_distinct() {
  local scratch target violation wrong replacement banner_left banner_right
  scratch="$(new_scratch_repo "diagnostic-identity")"
  target="$scratch/docs/negative.md"
  printf 'active spelling: %s\n' "$BAD_TOKEN" > "$target"
  run_gate_in_dir "$scratch"
  assert_equal "diagnostic identity exit" "1" "$GATE_EXIT"
  violation="$(printf '%s\n' "$GATE_STDERR" | while IFS= read -r line; do case "$line" in VIOLATION:*) printf '%s' "$line"; break ;; esac; done)"
  wrong="${violation#*wrong spelling \'}"; wrong="${wrong%%\' \(use \'*}"
  replacement="${violation##*\(use \'}"; replacement="${replacement%\')}"
  assert_equal "diagnostic wrong token" "$BAD_TOKEN" "$wrong"
  assert_not_equal "diagnostic canonical replacement" "$wrong" "$replacement"
  banner_left="${GATE_STDERR#=== Spelling Gate: }"; banner_left="${banner_left%% vs *}"
  banner_right="${GATE_STDERR#* vs }"; banner_right="${banner_right%% ===*}"
  assert_not_equal "banner canonical replacement" "$banner_left" "$banner_right"
}

default_tests=(
  test_content_allowlist_same_line_extra_token_rejected
  test_broad_path_allowlist_active_code_rejected
  test_remaining_overbroad_path_allowlists_active_docs_and_src_rejected
  test_forbidden_and_canonical_diagnostic_bytes_are_distinct
)
if [[ "$#" -gt 0 ]]; then default_tests=("$@"); fi
run_test_names "${default_tests[@]}"
