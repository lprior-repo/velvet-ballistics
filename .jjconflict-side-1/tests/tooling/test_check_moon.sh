#!/usr/bin/env bash
set -euo pipefail
# Behavioral tests for scripts/check-moon.sh (vb-rdpft).
# Uses fake `moon` doubles on controlled PATHs plus live checks against the
# real system impostor (~/.cargo/bin/moon) and validated moon
# (~/.moon/bin/moon); live checks skip when the binary is absent.

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
SCRIPT="$ROOT/scripts/check-moon.sh"
PASSED=0; FAILED=0

run_test() {
  local n="$1" f="$2"
  printf '  %-65s ' "$n"
  set +e; local out; out="$("$f" 2>&1)"; local rc=$?; set -e
  if [ "$rc" -eq 0 ]; then printf 'PASS\n'; PASSED=$((PASSED+1))
  else printf 'FAIL\n'; [ -n "$out" ] && printf '%s\n' "$out" | sed 's/^/    /'; FAILED=$((FAILED+1)); fi
}

# make_fake DIR KIND: installs DIR/moon printing hello-world (impostor) or a
# version line (real), ignoring argv and exiting 0 like the observed binaries.
make_fake() {
  local dir="$1" kind="$2"
  mkdir -p "$dir"
  if [ "$kind" = "impostor" ]; then
    printf '#!/usr/bin/env bash\nprintf "Hello, world!\\n"; exit 0\n' > "$dir/moon"
  else
    printf '#!/usr/bin/env bash\nprintf "moon 9.9.9\\n"; exit 0\n' > "$dir/moon"
  fi
  chmod +x "$dir/moon"
}

with_path() {
  local dir="$1"; shift
  PATH="$dir:/usr/bin:/bin" MOON_BIN= bash "$SCRIPT" "$@" 2>&1
}

test_accepts_real_moon() {
  local dir; dir="$(mktemp -d)"
  make_fake "$dir" real
  local out rc
  out="$(with_path "$dir")"; rc=$?
  rm -rf "$dir"
  [ "$rc" -eq 0 ] || { printf 'expected exit 0 got %d: %s\n' "$rc" "$out"; return 1; }
  echo "$out" | grep -q 'moon 9.9.9' || { printf 'missing version identity: %s\n' "$out"; return 1; }
}

test_rejects_impostor() {
  local dir; dir="$(mktemp -d)"
  make_fake "$dir" impostor
  local out rc
  set +e; out="$(with_path "$dir")"; rc=$?; set -e
  rm -rf "$dir"
  [ "$rc" -eq 1 ] || { printf 'expected exit 1 got %d: %s\n' "$rc" "$out"; return 1; }
  echo "$out" | grep -qi 'impostor' || { printf 'missing impostor diagnostic: %s\n' "$out"; return 1; }
}

test_no_silent_substitution() {
  # Impostor first on PATH, real moon later: must still fail closed rather
  # than quietly switching to the later binary.
  local bad good; bad="$(mktemp -d)"; good="$(mktemp -d)"
  make_fake "$bad" impostor; make_fake "$good" real
  local out rc
  set +e; out="$(PATH="$bad:$good:/usr/bin:/bin" MOON_BIN= bash "$SCRIPT" 2>&1)"; rc=$?; set -e
  rm -rf "$bad" "$good"
  [ "$rc" -eq 1 ] || { printf 'expected exit 1 got %d: %s\n' "$rc" "$out"; return 1; }
}

test_missing_moon() {
  local dir; dir="$(mktemp -d)"
  local out rc
  set +e; out="$(with_path "$dir")"; rc=$?; set -e
  rm -rf "$dir"
  [ "$rc" -eq 1 ] || { printf 'expected exit 1 got %d: %s\n' "$rc" "$out"; return 1; }
}

test_nonzero_version_probe() {
  local dir; dir="$(mktemp -d)"
  mkdir -p "$dir"
  printf '#!/usr/bin/env bash\nprintf "moon 9.9.9\\n"; exit 3\n' > "$dir/moon"; chmod +x "$dir/moon"
  local out rc
  set +e; out="$(with_path "$dir")"; rc=$?; set -e
  rm -rf "$dir"
  [ "$rc" -eq 1 ] || { printf 'expected exit 1 got %d: %s\n' "$rc" "$out"; return 1; }
}

test_moon_bin_override_real() {
  local dir; dir="$(mktemp -d)"
  make_fake "$dir" real
  local out rc
  out="$(MOON_BIN="$dir/moon" bash "$SCRIPT" 2>&1)"; rc=$?
  rm -rf "$dir"
  [ "$rc" -eq 0 ] || { printf 'expected exit 0 got %d: %s\n' "$rc" "$out"; return 1; }
  echo "$out" | grep -q "$dir/moon" || { printf 'missing override path: %s\n' "$out"; return 1; }
}

test_moon_bin_override_impostor() {
  local dir; dir="$(mktemp -d)"
  make_fake "$dir" impostor
  local out rc
  set +e; out="$(MOON_BIN="$dir/moon" bash "$SCRIPT" 2>&1)"; rc=$?; set -e
  rm -rf "$dir"
  [ "$rc" -eq 1 ] || { printf 'expected exit 1 got %d: %s\n' "$rc" "$out"; return 1; }
}

test_live_system_impostor() {
  # Ties the guard to the real-world evidence: the actual ~/.cargo/bin/moon.
  [ -x "$HOME/.cargo/bin/moon" ] || { printf 'SKIP (no ~/.cargo/bin/moon)\n'; return 0; }
  local out rc
  set +e; out="$(MOON_BIN="$HOME/.cargo/bin/moon" bash "$SCRIPT" 2>&1)"; rc=$?; set -e
  [ "$rc" -eq 1 ] || { printf 'expected exit 1 got %d: %s\n' "$rc" "$out"; return 1; }
}

test_live_validated_moon() {
  [ -x "/home/lewis/.moon/bin/moon" ] || { printf 'SKIP (no validated moon)\n'; return 0; }
  local out rc
  out="$(MOON_BIN="/home/lewis/.moon/bin/moon" bash "$SCRIPT" 2>&1)"; rc=$?
  [ "$rc" -eq 0 ] || { printf 'expected exit 0 got %d: %s\n' "$rc" "$out"; return 1; }
}

main() {
  run_test "accepts version-reporting moon" test_accepts_real_moon
  run_test "rejects hello-world impostor exit 0" test_rejects_impostor
  run_test "fail-closed when impostor shadows real moon" test_no_silent_substitution
  run_test "rejects missing moon on PATH" test_missing_moon
  run_test "rejects nonzero --version probe" test_nonzero_version_probe
  run_test "MOON_BIN override accepts real moon" test_moon_bin_override_real
  run_test "MOON_BIN override rejects impostor" test_moon_bin_override_impostor
  run_test "live: rejects actual ~/.cargo/bin/moon" test_live_system_impostor
  run_test "live: accepts validated ~/.moon/bin/moon" test_live_validated_moon
  printf '\nCheck-moon results: %d passed, %d failed\n' "$PASSED" "$FAILED"
  [ "$FAILED" -eq 0 ]
}
main "$@"
