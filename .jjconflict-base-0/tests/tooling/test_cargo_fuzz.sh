#!/usr/bin/env bash
set -euo pipefail
# Integration tests for cargo fuzz (I33-I36)

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
PASSED=0; FAILED=0

run_test() {
  local n="$1" f="$2"
  printf '  %-65s ' "$n"
  set +e; local out; out="$("$f" 2>&1)"; local rc=$?; set -e
  if [ "$rc" -eq 0 ]; then printf 'PASS\n'; PASSED=$((PASSED+1))
  else printf 'FAIL\n'; [ -n "$out" ] && printf '%s\n' "$out" | sed 's/^/    /'; FAILED=$((FAILED+1)); fi
}

test_i33() {
  [ -f "$ROOT/fuzz/Cargo.toml" ] || { printf 'SKIP: no fuzz/Cargo.toml\n'; return 0; }
  if ! command -v cargo >/dev/null 2>&1 || ! cargo fuzz --help >/dev/null 2>&1; then
    printf 'SKIP: cargo fuzz not available\n'; return 0
  fi
  local out; out="$(cargo fuzz list 2>&1)"; local rc=$?
  [ "$rc" -eq 0 ] || { printf 'exit %d\n' "$rc"; return 1; }
  [ -n "$out" ] || { printf 'empty output\n'; return 1; }
}

test_i34() {
  [ -f "$ROOT/fuzz/Cargo.toml" ] || { printf 'SKIP: no fuzz/Cargo.toml\n'; return 0; }
  if ! command -v cargo >/dev/null 2>&1 || ! cargo fuzz --help >/dev/null 2>&1; then
    printf 'SKIP: cargo fuzz not available\n'; return 0
  fi
  local out; out="$(cargo fuzz list 2>&1)"; local rc=$?
  # Aligned with I33: fail on non-zero exit (tool is confirmed available via --help above)
  [ "$rc" -eq 0 ] || { printf 'exit %d\n' "$rc"; return 1; }
  local cnt; cnt="$(echo "$out" | grep -c . || echo 0)"
  [ "$cnt" -ge 1 ] || { printf '0 targets\n'; return 1; }
  printf '  %d targets\n' "$cnt"
}

test_i35() {
  [ -f "$ROOT/fuzz/Cargo.toml" ] || { printf 'SKIP: no fuzz/Cargo.toml\n'; return 0; }
  if ! command -v cargo >/dev/null 2>&1 || ! cargo fuzz --help >/dev/null 2>&1; then
    printf 'SKIP: cargo fuzz not available\n'; return 0
  fi
  if ! rustup target list --installed 2>/dev/null | grep -q 'x86_64-unknown-linux-gnu'; then
    printf 'SKIP: gnu target not installed\n'; return 0
  fi
  local out; out="$(cargo fuzz build --target x86_64-unknown-linux-gnu 2>&1)"; local rc=$?
  [ "$rc" -eq 0 ] || { printf 'build failed (exit %d)\n' "$rc"; return 1; }
}

test_i36() {
  [ -f "$ROOT/fuzz/Cargo.toml" ] || { printf 'SKIP: no fuzz/Cargo.toml\n'; return 0; }
  if ! command -v cargo >/dev/null 2>&1 || ! cargo fuzz --help >/dev/null 2>&1; then
    printf 'SKIP: cargo fuzz not available\n'; return 0
  fi
  local out; out="$(cargo fuzz build --target x86_64-unknown-nonexistent 2>&1)"; local rc=$?
  [ "$rc" -ne 0 ] || { printf 'should have failed with bad target\n'; return 1; }
}

main() {
  run_test "I33: cargo fuzz list exits 0 with targets" test_i33
  run_test "I34: cargo fuzz list non-empty target count" test_i34
  run_test "I35: cargo fuzz build with GNU target" test_i35
  run_test "I36: cargo fuzz build fails with bad target" test_i36
  printf '\nCargo-fuzz results: %d passed, %d failed\n' "$PASSED" "$FAILED"
  [ "$FAILED" -eq 0 ]
}
main "$@"
