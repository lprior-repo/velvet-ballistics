#!/usr/bin/env bash
set -euo pipefail
# Integration tests for kani-list.sh (I01-I10)

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
SCRIPT="$ROOT/scripts/kani-list.sh"
PASSED=0; FAILED=0

run_test() {
  local n="$1" f="$2"
  printf '  %-65s ' "$n"
  set +e; local out; out="$("$f" 2>&1)"; local rc=$?; set -e
  if [ "$rc" -eq 0 ]; then printf 'PASS\n'; PASSED=$((PASSED+1))
  else printf 'FAIL\n'; [ -n "$out" ] && printf '%s\n' "$out" | sed 's/^/    /'; FAILED=$((FAILED+1)); fi
}

test_i01() {
  local out; out="$("$SCRIPT" 2>&1)"; local rc=$?
  [ "$rc" -eq 2 ] || { printf 'expected exit 2 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -qi 'usage' || { printf 'missing usage\n'; return 1; }
}

test_i02() {
  # Behavioral test: run kani-list.sh with cargo kani unavailable
  # Create a fake cargo that rejects everything, simulating kani missing
  local tmp; tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' RETURN
  local fake_cargo; fake_cargo="$tmp/cargo"
  cat > "$fake_cargo" <<'FAKE_CARGO'
#!/usr/bin/env bash
echo "fake cargo: $*" >&2
exit 1
FAKE_CARGO
  chmod +x "$fake_cargo"

  local out; out="$(PATH="$tmp:/usr/bin:/bin" "$SCRIPT" vb_core 2>&1)"; local rc=$?
  [ "$rc" -eq 1 ] || { printf 'expected exit 1 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -qi 'cargo kani is required' || { printf 'missing required cargo kani message\n'; return 1; }
}

test_i03() {
  if ! command -v cargo >/dev/null 2>&1; then printf 'SKIP: no cargo\n'; return 0; fi
  if ! cargo kani --version >/dev/null 2>&1; then printf 'SKIP: no cargo kani\n'; return 0; fi
  local tmp; tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' RETURN
  local out; out="$(KANI_LIST_DIR="$tmp" "$SCRIPT" vb_core 2>&1)"; local rc=$?
  [ "$rc" -eq 0 ] || { printf 'exit %d: %s\n' "$rc" "$(echo "$out" | head -3)"; return 1; }
  echo "$out" | grep -q 'KANI_LIST_OK' || { printf 'missing KANI_LIST_OK\n'; return 1; }
  [ -f "$tmp/vb_core.json" ] || { printf 'missing evidence file\n'; return 1; }
  python3 -m json.tool "$tmp/vb_core.json" >/dev/null 2>&1 || { printf 'invalid JSON\n'; return 1; }
  local cnt; cnt="$(python3 -c "import json; print(json.load(open('$tmp/vb_core.json')).get('totals',{}).get('standard-harnesses',0))" 2>/dev/null)"
  [ -n "$cnt" ] && [ "$cnt" -gt 0 ] || { printf 'zero harnesses\n'; return 1; }
  printf '  vb_core: %s harnesses\n' "$cnt"
}

test_i04() {
  if ! command -v cargo >/dev/null 2>&1; then printf 'SKIP: no cargo\n'; return 0; fi
  if ! cargo kani --version >/dev/null 2>&1; then printf 'SKIP: no cargo kani\n'; return 0; fi
  local tmp; tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' RETURN
  local out; out="$(KANI_LIST_DIR="$tmp" "$SCRIPT" vb_runtime 2>&1)"; local rc=$?
  [ "$rc" -eq 0 ] || { printf 'exit %d: %s\n' "$rc" "$(echo "$out" | head -3)"; return 1; }
  echo "$out" | grep -q 'KANI_LIST_OK' || { printf 'missing KANI_LIST_OK\n'; return 1; }
  [ -f "$tmp/vb_runtime.json" ] || { printf 'missing evidence file\n'; return 1; }
  python3 -m json.tool "$tmp/vb_runtime.json" >/dev/null 2>&1 || { printf 'invalid JSON\n'; return 1; }
  local cnt; cnt="$(python3 -c "import json; print(json.load(open('$tmp/vb_runtime.json')).get('totals',{}).get('standard-harnesses',0))" 2>/dev/null)"
  [ -n "$cnt" ] && [ "$cnt" -gt 0 ] || { printf 'zero harnesses\n'; return 1; }
  printf '  vb_runtime: %s harnesses\n' "$cnt"
}

test_i05() {
  if ! command -v cargo >/dev/null 2>&1; then printf 'SKIP: no cargo\n'; return 0; fi
  if ! cargo kani --version >/dev/null 2>&1; then printf 'SKIP: no cargo kani\n'; return 0; fi
  local tmp; tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' RETURN
  local out; out="$(KANI_LIST_DIR="$tmp" "$SCRIPT" nonexistent_package_xyz_12345 2>&1)"; local rc=$?
  [ "$rc" -eq 1 ] || { printf 'expected exit 1 got %d\n' "$rc"; return 1; }
  [ ! -f "$tmp/nonexistent_package_xyz_12345.json" ] || { printf 'evidence file exists for bad package\n'; return 1; }
}

test_i06() {
  if ! command -v cargo >/dev/null 2>&1; then printf 'SKIP: no cargo\n'; return 0; fi
  if ! cargo kani --version >/dev/null 2>&1; then printf 'SKIP: no cargo kani\n'; return 0; fi
  local tmp; tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' RETURN
  local out; out="$(KANI_LIST_DIR="$tmp" KANI_FEATURES='vb_runtime/kani-diagnostic-codes' "$SCRIPT" vb_runtime 2>&1)"; local rc=$?
  [ "$rc" -eq 1 ] || { printf 'expected exit 1 (fail-closed) got %d\n' "$rc"; return 1; }
  [ ! -f "$tmp/vb_runtime.json" ] || { printf 'evidence file should not exist\n'; return 1; }
}

test_i07() {
  if ! command -v cargo >/dev/null 2>&1; then printf 'SKIP: no cargo\n'; return 0; fi
  if ! cargo kani --version >/dev/null 2>&1; then printf 'SKIP: no cargo kani\n'; return 0; fi
  grep -q 'kani-diagnostic-codes' "$ROOT/crates/vb_core/Cargo.toml" 2>/dev/null || { printf 'SKIP: vb_core has no kani-diagnostic-codes\n'; return 0; }
  local tmp; tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' RETURN
  local out; out="$(KANI_LIST_DIR="$tmp" KANI_FEATURES='vb_core/kani-diagnostic-codes' "$SCRIPT" vb_core 2>&1)"; local rc=$?
  [ "$rc" -eq 0 ] || { printf 'expected exit 0 got %d\n' "$rc"; return 1; }
  echo "$out" | grep -q 'KANI_LIST_OK' || { printf 'missing KANI_LIST_OK\n'; return 1; }
  [ -f "$tmp/vb_core.json" ] || { printf 'missing evidence file\n'; return 1; }
}

test_i08() {
  if ! command -v cargo >/dev/null 2>&1; then printf 'SKIP: no cargo\n'; return 0; fi
  if ! cargo kani --version >/dev/null 2>&1; then printf 'SKIP: no cargo kani\n'; return 0; fi
  local tmp; tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' RETURN
  local out; out="$(KANI_LIST_DIR="$tmp" "$SCRIPT" vb_core 2>&1)"; local rc=$?
  [ "$rc" -eq 0 ] || { printf 'expected exit 0 got %d\n' "$rc"; return 1; }
  [ -f "$tmp/vb_core.json" ] || { printf 'evidence not in KANI_LIST_DIR\n'; return 1; }
  python3 -m json.tool "$tmp/vb_core.json" >/dev/null 2>&1 || { printf 'invalid JSON in override dir\n'; return 1; }
}

test_i09() {
  # Behavioral test: simulate cargo kani list producing empty JSON file
  # Create a fake cargo that produces real metadata but empty kani-list.json
  local tmp; tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' RETURN
  local real_cargo; real_cargo="$(command -v cargo)" || { printf 'SKIP: no cargo\n'; return 0; }
  local fake_cargo; fake_cargo="$tmp/cargo"
  cat > "$fake_cargo" <<FAKE_CARGO
#!/usr/bin/env bash
case "\$1" in
  metadata)
    exec ${real_cargo} "\$@" ;;
  kani)
    case "\$2" in
      list)
        # Create an empty kani-list.json file (zero bytes)
        touch "\$(pwd)/kani-list.json"
        exit 0 ;;
      *)
        # Allow --version and other kani subcommands
        printf 'cargo-kani 0.67.0 (cargo plugin)\n'
        exit 0 ;;
    esac ;;
esac
echo "fake cargo: \$*" >&2
exit 1
FAKE_CARGO
  chmod +x "$fake_cargo"

  local out; out="$(PATH="$tmp:/usr/bin:/bin" "$SCRIPT" vb_core 2>&1)"; local rc=$?
  [ "$rc" -eq 1 ] || { printf 'expected exit 1 from empty JSON, got %d\n' "$rc"; return 1; }
  echo "$out" | grep -qi 'did not produce' || { printf 'missing "did not produce" error\n'; return 1; }
}

test_i10() {
  # Behavioral test: simulate cargo kani list producing invalid JSON
  # Create a fake cargo that passes metadata through but produces invalid JSON from kani list
  local tmp; tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' RETURN
  local real_cargo; real_cargo="$(command -v cargo)" || { printf 'SKIP: no cargo\n'; return 0; }
  local fake_cargo; fake_cargo="$tmp/cargo"
  cat > "$fake_cargo" <<FAKE_CARGO
#!/usr/bin/env bash
case "\$1" in
  metadata)
    exec ${real_cargo} "\$@" ;;
  kani)
    case "\$2" in
      list)
        printf 'NOT JSON CONTENT\n' > "\$(pwd)/kani-list.json"
        exit 0 ;;
      *)
        printf 'cargo-kani 0.67.0 (cargo plugin)\n'
        exit 0 ;;
    esac ;;
esac
echo "fake cargo: \$*" >&2
exit 1
FAKE_CARGO
  chmod +x "$fake_cargo"

  local out; out="$(PATH="$tmp:/usr/bin:/bin" "$SCRIPT" vb_core 2>&1)"; local rc=$?
  [ "$rc" -ne 0 ] || { printf 'expected non-zero exit for invalid JSON, got %d\n' "$rc"; return 1; }
}

main() {
  run_test "I01: exits 2 with usage when no args" test_i01
  run_test "I02: exits 1 when cargo kani missing" test_i02
  run_test "I03: valid JSON for vb_core with non-zero harnesses" test_i03
  run_test "I04: valid JSON for vb_runtime with non-zero harnesses" test_i04
  run_test "I05: exits 1 for nonexistent package" test_i05
  run_test "I06: fails closed on undeclared KANI_FEATURES" test_i06
  run_test "I07: succeeds with declared KANI_FEATURES" test_i07
  run_test "I08: outputs to KANI_LIST_DIR override" test_i08
  run_test "I09: exits 1 on empty JSON output" test_i09
  run_test "I10: validates JSON with python3 json.tool" test_i10
  printf '\nKani-list results: %d passed, %d failed\n' "$PASSED" "$FAILED"
  [ "$FAILED" -eq 0 ]
}
main "$@"
