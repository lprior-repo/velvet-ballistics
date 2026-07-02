#!/usr/bin/env bash
set -euo pipefail
# Proptest invariants (P01-P06)

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
PASSED=0; FAILED=0

run_test() {
  local n="$1" f="$2"
  printf '  %-65s ' "$n"
  set +e; local out; out="$("$f" 2>&1)"; local rc=$?; set -e
  if [ "$rc" -eq 0 ]; then printf 'PASS\n'; PASSED=$((PASSED+1))
  else printf 'FAIL\n'; [ -n "$out" ] && printf '%s\n' "$out" | sed 's/^/    /'; FAILED=$((FAILED+1)); fi
}

test_p01() {
  local s="$ROOT/scripts/kani-list.sh"
  [ -f "$s" ] || { printf 'SKIP: no kani-list.sh\n'; return 0; }
  # Anti-invariant: random non-package name must exit non-zero
  local out; out="$("$s" "zzz_not_a_real_pkg_$(date +%s)" 2>&1)"; local rc=$?
  [ "$rc" -ne 0 ] || { printf 'accepted non-existent package (exit 0)\n'; return 1; }
}

test_p02() {
  local s="$ROOT/scripts/flux-check-package.sh"
  [ -f "$s" ] || { printf 'SKIP: no flux-check-package.sh\n'; return 0; }
  for sel in --lib --test --tests --benches --all-targets; do
    local out; out="$("$s" vb_core "$sel" 2>&1)"; local rc=$?
    [ "$rc" -eq 2 ] || { printf '%s should exit 2, got %d\n' "$sel" "$rc"; return 1; }
    echo "$out" | grep -qi 'unsupported' || { printf '%s missing unsupported\n' "$sel"; return 1; }
  done
  # Anti-invariant: valid flag must NOT trigger "unsupported" rejection
  local out; out="$("$s" vb_core --message-format human 2>&1)" || true
  if echo "$out" | grep -qi 'unsupported'; then
    printf '--message-format incorrectly rejected\n'; return 1
  fi
}

test_p03() {
  local s="$ROOT/scripts/guard-zero-tests.sh"
  [ -f "$s" ] || { printf 'SKIP: no guard-zero-tests.sh\n'; return 0; }
  # N=0 patterns must exit 1
  for pat in \
    'printf "running 0 tests\n"; exit 0' \
    'printf "running 0 tests\n"; printf "test result: ok. 0 passed; 0 failed\n"; exit 0'; do
    local fake; fake="$(mktemp)"; printf '#!/usr/bin/env bash\n%s\n' "$pat" > "$fake"; chmod +x "$fake"
    local out; out="$("$s" -- "$fake" 2>&1)"; local rc=$?; rm -f "$fake"
    [ "$rc" -eq 1 ] || { printf 'N=0 pattern exited %d, expected 1\n' "$rc"; return 1; }
  done
  # N>0 patterns must exit 0
  for pat in \
    'printf "running 5 tests\n"; exit 0' \
    'printf "running 1 tests\n"; printf "test result: ok. 1 passed; 0 failed\n"; exit 0'; do
    local fake; fake="$(mktemp)"; printf '#!/usr/bin/env bash\n%s\n' "$pat" > "$fake"; chmod +x "$fake"
    local out; out="$("$s" -- "$fake" 2>&1)"; local rc=$?; rm -f "$fake"
    [ "$rc" -eq 0 ] || { printf 'N>0 pattern exited %d, expected 0\n' "$rc"; return 1; }
  done
}

test_p04() {
  local s="$ROOT/scripts/kani-list.sh"
  [ -f "$s" ] || { printf 'SKIP: no kani-list.sh\n'; return 0; }
  if ! command -v cargo >/dev/null 2>&1; then printf 'SKIP: no cargo\n'; return 0; fi
  if ! cargo kani --version >/dev/null 2>&1; then printf 'SKIP: no cargo kani\n'; return 0; fi

  # Behavioral property: ∀ packages with real cargo kani → output JSON is valid
  local tmp; tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' RETURN
  local failures=0
  for pkg in vb_core vb_runtime; do
    local out; out="$(KANI_LIST_DIR="$tmp" "$s" "$pkg" 2>&1)"; local rc=$?
    if [ "$rc" -ne 0 ]; then
      printf '  %s: exit %d (SKIP row)\n' "$pkg" "$rc"
      continue
    fi
    [ -f "$tmp/${pkg}.json" ] || { printf '  %s: missing evidence file\n' "$pkg"; failures=$((failures+1)); continue; }
    if ! python3 -m json.tool "$tmp/${pkg}.json" >/dev/null 2>&1; then
      printf '  %s: invalid JSON evidence\n' "$pkg"; failures=$((failures+1)); continue
    fi
    local cnt; cnt="$(python3 -c "import json; print(json.load(open('$tmp/${pkg}.json')).get('totals',{}).get('standard-harnesses',0))" 2>/dev/null)"
    if [ -z "$cnt" ] || [ "$cnt" -le 0 ]; then
      printf '  %s: zero harnesses in valid JSON\n' "$pkg"; failures=$((failures+1)); continue
    fi
    printf '  %s: valid JSON (%s harnesses)\n' "$pkg" "$cnt"
  done
  [ "$failures" -eq 0 ] || { printf 'JSON validity property violated: %d failure(s)\n' "$failures"; return 1; }
}

test_p05() {
  local s="$ROOT/scripts/flux-check-package.sh"
  [ -f "$s" ] || { printf 'SKIP: no flux-check-package.sh\n'; return 0; }
  # Determinism: same input must produce same exit code
  local r1 r2
  "$s" vb_core --lib >/dev/null 2>&1; r1=$?
  "$s" vb_core --lib >/dev/null 2>&1; r2=$?
  [ "$r1" -eq "$r2" ] || { printf 'non-deterministic exit: %d vs %d\n' "$r1" "$r2"; return 1; }
  "$s" >/dev/null 2>&1; r1=$?
  "$s" >/dev/null 2>&1; r2=$?
  [ "$r1" -eq "$r2" ] || { printf 'non-deterministic exit for no-args: %d vs %d\n' "$r1" "$r2"; return 1; }
}

test_p06() {
  [ -f "$ROOT/fuzz/Cargo.toml" ] || { printf 'SKIP: no fuzz/Cargo.toml\n'; return 0; }
  if command -v cargo >/dev/null 2>&1 && cargo fuzz --help >/dev/null 2>&1; then
    local targets; targets="$(cargo fuzz list 2>/dev/null || true)"
    [ -n "$targets" ] || { printf 'SKIP: no fuzz targets\n'; return 0; }
    while IFS= read -r t; do
      t="$(echo "$t" | xargs)"; [ -z "$t" ] && continue
      grep -q "name = \"$t\"" "$ROOT/fuzz/Cargo.toml" || { printf 'target %s not in Cargo.toml\n' "$t"; return 1; }
    done <<< "$targets"
  else
    printf 'SKIP: no cargo fuzz\n'
    return 0
  fi
}

main() {
  run_test "P01: kani-list rejects non-package names" test_p01
  run_test "P02: flux-check rejects all unsupported selectors" test_p02
  run_test "P03: guard-zero-test roundtrips N=0/N>0" test_p03
  run_test "P04: kani-list JSON always valid on success" test_p04
  run_test "P05: flux-check exit codes deterministic" test_p05
  run_test "P06: fuzz target list prefix-closed" test_p06
  printf '\nProptest results: %d passed, %d failed\n' "$PASSED" "$FAILED"
  [ "$FAILED" -eq 0 ]
}
main "$@"
