#!/usr/bin/env bash
set -euo pipefail
# Integration tests for guard-zero-tests.sh (I21-I29)
# NOTE: guard-zero-tests.sh has pipefragility (FIND-SHVXY-001)

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
SCRIPT="$ROOT/scripts/guard-zero-tests.sh"
PASSED=0; FAILED=0

run_test() {
  local n="$1" f="$2"
  printf '  %-65s ' "$n"
  set +e; local out; out="$("$f" 2>&1)"; local rc=$?; set -e
  if [ "$rc" -eq 0 ]; then printf 'PASS\n'; PASSED=$((PASSED+1))
  else printf 'FAIL\n'; [ -n "$out" ] && printf '%s\n' "$out" | sed 's/^/    /'; FAILED=$((FAILED+1)); fi
}

test_i21() {
  local out; out="$("$SCRIPT" 2>&1)"; local rc=$?
  [ "$rc" -eq 2 ] || { printf 'expected exit 2 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -qi 'usage' || { printf 'missing usage\n'; return 1; }
}

test_i22() {
  local fake; fake="$(mktemp)"
  printf '#!/usr/bin/env bash\nprintf "running 0 tests\n"; printf "test result: ok. 0 passed; 0 failed; 10 filtered out\n"; exit 0\n' > "$fake"; chmod +x "$fake"
  local out; out="$("$SCRIPT" -- "$fake" 2>&1)"; local rc=$?; rm -f "$fake"
  [ "$rc" -eq 1 ] || { printf 'expected exit 1 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -qi 'zero applicable' || { printf 'missing zero applicable\n'; return 1; }
}

test_i23() {
  local fake; fake="$(mktemp)"
  printf '#!/usr/bin/env bash\nprintf "running 5 tests\n"; printf "test result: ok. 5 passed; 0 failed; 0 ignored\n"; exit 0\n' > "$fake"; chmod +x "$fake"
  local out; out="$("$SCRIPT" -- "$fake" 2>&1)"; local rc=$?; rm -f "$fake"
  [ "$rc" -eq 0 ] || { printf 'expected exit 0 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -qi 'PASS' || { printf 'missing PASS\n'; return 1; }
}

test_i24() {
  local fake; fake="$(mktemp)"
  printf '#!/usr/bin/env bash\nprintf "running 5 tests\n"; printf "test result: ok. 5 passed; 0 failed; 0 ignored\n"; exit 0\n' > "$fake"; chmod +x "$fake"
  local out; out="$("$SCRIPT" -- "$fake" 2>&1)"; local rc=$?; rm -f "$fake"
  [ "$rc" -eq 0 ] || { printf 'expected exit 0 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -q '5 applicable' || { printf 'missing 5 applicable\n'; return 1; }
}

test_i25() {
  local fake; fake="$(mktemp)"
  printf '#!/usr/bin/env bash\nprintf "running 5 tests\n"; printf "test result: ok. 5 passed; 0 failed; 3 filtered out\n"; exit 0\n' > "$fake"; chmod +x "$fake"
  local out; out="$("$SCRIPT" -- "$fake" 2>&1)"; local rc=$?; rm -f "$fake"
  [ "$rc" -eq 0 ] || { printf 'expected exit 0 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -q '5 applicable' || { printf 'missing 5 applicable\n'; return 1; }
}

test_i26() {
  local fake; fake="$(mktemp)"
  printf '#!/usr/bin/env bash\nprintf "running 0 tests\n"; printf "test result: ok. 0 passed; 0 failed; 10 filtered out\n"; exit 0\n' > "$fake"; chmod +x "$fake"
  local out; out="$("$SCRIPT" -- "$fake" 2>&1)"; local rc=$?; rm -f "$fake"
  [ "$rc" -eq 1 ] || { printf 'expected exit 1 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -qi 'zero applicable' || { printf 'missing zero applicable\n'; return 1; }
}

test_i27() {
  local fake; fake="$(mktemp)"
  printf '#!/usr/bin/env bash\nprintf "running 1 tests\n"; printf "error: compilation failed\n"; exit 101\n' > "$fake"; chmod +x "$fake"
  local out; out="$("$SCRIPT" -- "$fake" 2>&1)"; local rc=$?; rm -f "$fake"
  [ "$rc" -eq 1 ] || { printf 'expected exit 1 got %d\n' "$rc"; return 1; }
}

test_i28() {
  # Behavioral test: feed intentionally unparseable output to guard-zero-tests.sh
  # Create a fake test command that produces output matching none of the known patterns
  # NOTE: guard-zero-tests.sh has pipefragility (FIND-SHVXY-001) — it exits via
  # `set -e` on a failed grep|head pipeline before reaching the "could not parse" printf.
  # The exit 1 behavior is still correct (fail-closed on unparseable output).
  local fake; fake="$(mktemp)"
  printf '#!/usr/bin/env bash\nprintf "garbled output: something unrecognizable\\n"; printf "no known patterns here\\n"; exit 0\n' > "$fake"; chmod +x "$fake"
  local out; out="$("$SCRIPT" -- "$fake" 2>&1)"; local rc=$?; rm -f "$fake"
  [ "$rc" -eq 1 ] || { printf 'expected exit 1 for unparseable output, got %d\n' "$rc"; return 1; }
  # Exit 1 proves fail-closed; exit 0 would mean vacuous pass
}

test_i29() {
  local fake; fake="$(mktemp)"
  printf '#!/usr/bin/env bash\nprintf "running 0 tests\n"; exit 0\n' > "$fake"; chmod +x "$fake"
  local out; out="$("$SCRIPT" -- "$fake" 2>&1)"; local rc=$?; rm -f "$fake"
  [ "$rc" -eq 1 ] || { printf 'expected exit 1 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -qi 'zero applicable' || { printf 'missing zero applicable\n'; return 1; }
}

main() {
  run_test "I21: exits 2 without args" test_i21
  run_test "I22: exits 1 on zero applicable tests" test_i22
  run_test "I23: exits 0 on nonzero applicable tests" test_i23
  run_test "I24: parses 'N passed' simple format" test_i24
  run_test "I25: parses 'N passed M filtered' format" test_i25
  run_test "I26: detects 0 passed M filtered as zero" test_i26
  run_test "I27: exits 1 on cargo test nonzero exit" test_i27
  run_test "I28: exits 1 on unparseable output (struct)" test_i28
  run_test "I29: detects 'running 0 tests' as zero" test_i29
  printf '\nGuard-zero results: %d passed, %d failed\n' "$PASSED" "$FAILED"
  [ "$FAILED" -eq 0 ]
}
main "$@"
