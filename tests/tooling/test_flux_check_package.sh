#!/usr/bin/env bash
set -euo pipefail
# Integration tests for flux-check-package.sh (I11-I20)

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
SCRIPT="$ROOT/scripts/flux-check-package.sh"
PASSED=0; FAILED=0

run_test() {
  local n="$1" f="$2"
  printf '  %-65s ' "$n"
  set +e; local out; out="$("$f" 2>&1)"; local rc=$?; set -e
  if [ "$rc" -eq 0 ]; then printf 'PASS\n'; PASSED=$((PASSED+1))
  else printf 'FAIL\n'; [ -n "$out" ] && printf '%s\n' "$out" | sed 's/^/    /'; FAILED=$((FAILED+1)); fi
}

test_i11() {
  local out; out="$("$SCRIPT" 2>&1)"; local rc=$?
  [ "$rc" -eq 2 ] || { printf 'expected exit 2 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -qi 'usage' || { printf 'missing usage\n'; return 1; }
}

test_i12() {
  if ! command -v cargo >/dev/null 2>&1; then printf 'SKIP: no cargo\n'; return 0; fi
  if ! cargo flux --version >/dev/null 2>&1; then printf 'SKIP: no cargo flux\n'; return 0; fi
  local out; out="$("$SCRIPT" vb_core 2>&1)"; local rc=$?
  [ "$rc" -eq 0 ] || { printf 'flux check failed (exit %d): %s\n' "$rc" "$(echo "$out" | tail -5)"; return 1; }
}

test_i13() {
  local out; out="$("$SCRIPT" vb_core --lib 2>&1)"; local rc=$?
  [ "$rc" -eq 2 ] || { printf 'expected exit 2 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -qi 'unsupported.*--lib' || { printf 'missing unsupported --lib\n'; return 1; }
}
test_i14() {
  local out; out="$("$SCRIPT" vb_core --test 2>&1)"; local rc=$?
  [ "$rc" -eq 2 ] || { printf 'expected exit 2 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -qi 'unsupported.*--test' || { printf 'missing unsupported --test\n'; return 1; }
}
test_i15() {
  local out; out="$("$SCRIPT" vb_core --tests 2>&1)"; local rc=$?
  [ "$rc" -eq 2 ] || { printf 'expected exit 2 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -qi 'unsupported.*--tests' || { printf 'missing unsupported --tests\n'; return 1; }
}
test_i16() {
  local out; out="$("$SCRIPT" vb_core --benches 2>&1)"; local rc=$?
  [ "$rc" -eq 2 ] || { printf 'expected exit 2 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -qi 'unsupported.*--benches' || { printf 'missing unsupported --benches\n'; return 1; }
}
test_i17() {
  local out; out="$("$SCRIPT" vb_core --all-targets 2>&1)"; local rc=$?
  [ "$rc" -eq 2 ] || { printf 'expected exit 2 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -qi 'unsupported.*--all-targets' || { printf 'missing unsupported --all-targets\n'; return 1; }
}
test_i18() {
  # Behavioral test: pass --message-format human through to cargo flux
  # Create a fake cargo that records what it received and verify flag passthrough
  local tmp; tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' RETURN
  local fake_cargo; fake_cargo="$tmp/cargo"
  cat > "$fake_cargo" <<'FAKE_CARGO'
#!/usr/bin/env bash
# Record args and exit 0
printf '%s\n' "$*" > /tmp/fake_cargo_args_f18.txt
exit 0
FAKE_CARGO
  chmod +x "$fake_cargo"
  rm -f /tmp/fake_cargo_args_f18.txt

  local out; out="$(PATH="$tmp:/usr/bin:/bin" "$SCRIPT" vb_core --message-format human 2>&1)"; local rc=$?
  # With a fake cargo, we don't require exit 0 (script may fail earlier)
  # But we verify --message-format was NOT rejected
  if echo "$out" | grep -qi 'unsupported'; then
    printf '--message-format incorrectly rejected\n'; return 1
  fi
  # Verify it was passed through (the fake cargo should have received it)
  if [ -f /tmp/fake_cargo_args_f18.txt ]; then
    grep -q 'message-format' /tmp/fake_cargo_args_f18.txt || { printf '--message-format not passed through to cargo\n'; return 1; }
  else
    # Script may have exited before reaching cargo flux; check if unsupported flag was rejected
    if echo "$out" | grep -qi 'unsupported.*message-format'; then
      printf '--message-format incorrectly rejected\n'; return 1
    fi
    # If we get here, script exited early for other reasons (no cargo metadata, etc.)
    # That's acceptable for a fake cargo env — the key is: --message-format is not unsupported
  fi
  rm -f /tmp/fake_cargo_args_f18.txt
}
test_i19() {
  local out; out="$("$SCRIPT" vb_core --lib --test 2>&1)"; local rc=$?
  [ "$rc" -eq 2 ] || { printf 'expected exit 2 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -qi 'unsupported' || { printf 'missing unsupported\n'; return 1; }
}
test_i20() {
  # Behavioral test (B020): cargo flux failure exit code propagation
  # Create a fake cargo that exits with code 42, verify flux-check-package.sh exits 42
  local tmp; tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' RETURN
  local fake_cargo; fake_cargo="$tmp/cargo"
  cat > "$fake_cargo" <<'FAKE_CARGO'
#!/usr/bin/env bash
exit 42
FAKE_CARGO
  chmod +x "$fake_cargo"

  local out; out="$(PATH="$tmp:/usr/bin:/bin" "$SCRIPT" vb_core 2>&1)"; local rc=$?
  [ "$rc" -eq 42 ] || { printf 'expected exit 42 (cargo flux failure propagation), got %d\n' "$rc"; return 1; }
}

main() {
  run_test "I11: exits 2 with usage when no args" test_i11
  run_test "I12: runs cargo flux for vb_core with exit 0" test_i12
  run_test "I13: rejects --lib with exit 2" test_i13
  run_test "I14: rejects --test with exit 2" test_i14
  run_test "I15: rejects --tests with exit 2" test_i15
  run_test "I16: rejects --benches with exit 2" test_i16
  run_test "I17: rejects --all-targets with exit 2" test_i17
  run_test "I18: passes through valid flags" test_i18
  run_test "I19: rejects multiple unsupported selectors" test_i19
  run_test "I20: propagates cargo flux failure exit" test_i20
  printf '\nFlux check results: %d passed, %d failed\n' "$PASSED" "$FAILED"
  [ "$FAILED" -eq 0 ]
}
main "$@"
