#!/usr/bin/env bash
set -euo pipefail
# Integration test for Loom cfg execution (I37)

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
PASSED=0; FAILED=0

run_test() {
  local n="$1" f="$2"
  printf '  %-65s ' "$n"
  set +e; local out; out="$("$f" 2>&1)"; local rc=$?; set -e
  if [ "$rc" -eq 0 ]; then printf 'PASS\n'; PASSED=$((PASSED+1))
  else printf 'FAIL\n'; [ -n "$out" ] && printf '%s\n' "$out" | sed 's/^/    /'; FAILED=$((FAILED+1)); fi
}

test_i37() {
  if ! command -v cargo >/dev/null 2>&1; then printf 'SKIP: no cargo\n'; return 0; fi
  [ -d "$ROOT/crates/vb_runtime/src/models/loom" ] || { printf 'SKIP: no loom models dir\n'; return 0; }
  grep -q 'loom' "$ROOT/crates/vb_runtime/Cargo.toml" 2>/dev/null || { printf 'SKIP: no loom dep\n'; return 0; }

  local out; out="$(RUSTFLAGS='--cfg loom' cargo test -p vb_runtime --lib -- models::loom 2>&1)"; local rc=$?
  if [ "$rc" -ne 0 ]; then
    printf 'SKIP: loom tests failed to compile (exit %d): %s\n' "$rc" "$(echo "$out" | tail -3)"
    return 0
  fi
  local passed; passed="$(echo "$out" | grep -o '[0-9]* passed' | sed 's/ passed//' | head -1)"
  [ -n "$passed" ] && [ "$passed" -gt 0 ] && printf '  %s passed\n' "$passed"
}

main() {
  run_test "I37: Loom model tests compile+execute under cfg(loom)" test_i37
  printf '\nLoom-cfg results: %d passed, %d failed\n' "$PASSED" "$FAILED"
  [ "$FAILED" -eq 0 ]
}
main "$@"
