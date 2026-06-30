#!/usr/bin/env bash
set -euo pipefail
# Static analysis tests (S01-S05)

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
PASSED=0; FAILED=0

run_test() {
  local name="$1" func="$2"
  printf '  %-65s ' "$name"
  set +e
  local out; out="$("$func" 2>&1)"; local rc=$?
  set -e
  if [ "$rc" -eq 0 ]; then printf 'PASS\n'; PASSED=$((PASSED+1))
  else printf 'FAIL\n'; [ -n "$out" ] && printf '%s\n' "$out" | sed 's/^/    /'; FAILED=$((FAILED+1)); fi
}

test_s01() {
  local e=0
  for s in scripts/kani-list.sh scripts/flux-check-package.sh scripts/guard-zero-tests.sh scripts/loom-list.sh; do
    local p="$ROOT/$s"
    if [ -f "$p" ] && command -v shellcheck >/dev/null 2>&1; then
      shellcheck -x "$p" >/dev/null 2>&1 || { printf '  shellcheck: %s\n' "$s"; e=1; }
    fi
  done
  [ "$e" -eq 0 ] || { printf '  shellcheck found errors in tooling scripts\n'; return 1; }
}

test_s02() {
  local e=0
  for s in scripts/kani-list.sh scripts/flux-check-package.sh scripts/guard-zero-tests.sh scripts/loom-list.sh; do
    local p="$ROOT/$s"
    [ -f "$p" ] || { printf '  MISSING: %s\n' "$s"; e=1; continue; }
    [ "$(head -1 "$p")" = "#!/usr/bin/env bash" ] || { printf '  WRONG SHEBANG: %s\n' "$s"; e=1; }
    [ -x "$p" ] || { printf '  NOT EXECUTABLE: %s\n' "$s"; e=1; }
  done
  [ "$e" -eq 0 ] || { printf '  script metadata audit failed\n'; return 1; }
}

test_s03() {
  local sp="$ROOT/schemas/kani-list.schema.json"
  [ -f "$sp" ] || { printf '  SKIP: schema not found\n'; return 0; }
  python3 -m json.tool "$sp" >/dev/null 2>&1 || { printf '  schema is not valid JSON\n'; return 1; }
}

test_s04() {
  local d="$ROOT/xtask"
  [ -f "$d/Cargo.toml" ] || { printf '  SKIP: xtask not found\n'; return 0; }
  [ -f "$d/src/loom.rs" ] || { printf '  SKIP: loom.rs not found\n'; return 0; }
  grep -q '"loom"' "$d/src/cli.rs" 2>/dev/null || { printf '  loom cmd not in cli.rs\n'; return 1; }
  local mc; mc="$(grep -cE '(journal_writer_queue|action_completion_cancel|timer_fired_cancel|shutdown_drain|bounded_queue)' "$d/src/loom.rs" 2>/dev/null || echo 0)"
  [ "$mc" -ge 5 ] || { printf '  only %d known models in loom.rs\n' "$mc"; return 1; }
}

test_s05() {
  local k="$ROOT/.moon/tasks/kani.yml"
  [ -f "$k" ] || { printf '  kani.yml not found\n'; return 1; }
  grep -q 'verify-kani:' "$k" || { printf '  verify-kani not in kani.yml\n'; return 1; }
  grep -q 'verify-kani-vb-validate:' "$k" || { printf '  verify-kani-vb-validate not in kani.yml\n'; return 1; }
}

main() {
  run_test "S01: shellcheck passes on tooling scripts" test_s01
  run_test "S02: scripts have shebang and execute bit" test_s02
  run_test "S03: kani-list JSON schema is valid" test_s03
  run_test "S04: xtask loom.rs has 5 expected models" test_s04
  run_test "S05: .moon/tasks/kani.yml has required tasks" test_s05
  printf '\nStatic results: %d passed, %d failed\n' "$PASSED" "$FAILED"
  [ "$FAILED" -eq 0 ]
}
main "$@"
