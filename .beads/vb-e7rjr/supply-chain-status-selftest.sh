#!/usr/bin/env bash
set -uo pipefail

# Full actual supply-chain script selftest.
# Extracts the real script from .moon/tasks/all.yml and executes it
# with PATH fixtures keyed by tool name. Never copies or rewrites the
# audit block or run_advisory_report.
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/../../" && pwd -P)"
TEST_DIR=$(mktemp -d /tmp/vb-selftest-XXXXXX)
WORK="$TEST_DIR/work"
mkdir -p "$WORK"
trap 'rm -rf "$TEST_DIR"' EXIT

PASS=0
FAIL=0
EVIDENCE="$WORK/evidence"
mkdir -p "$EVIDENCE"

pass() { echo "PASS: $*"; PASS=$((PASS+1)); }
fail() { echo "FAIL: $*"; FAIL=$((FAIL+1)); }

# Extract the actual supply-chain script from .moon/tasks/all.yml
python3 - "$REPO_ROOT/.moon/tasks/all.yml" "$WORK/actual_supply_chain.sh" <<'PYEOF'
import sys
lines = open(sys.argv[1]).read().splitlines()
out = open(sys.argv[2], 'w')
i = 0
while i < len(lines) and lines[i] != '  supply-chain:':
    i += 1
while i < len(lines) and lines[i] != '    script: |':
    i += 1
i += 1
body = []
while i < len(lines):
    line = lines[i]
    if line.startswith('    ') and not line.startswith('      '):
        break
    body.append(line[6:] if line.startswith('      ') else line)
    i += 1
out.write('\n'.join(body) + '\n')
out.close()
PYEOF

chmod +x "$WORK/actual_supply_chain.sh"
echo "Extracted actual supply-chain script to $WORK/actual_supply_chain.sh"

# Build PATH fixtures: a rustup shim that dispatches by tool name
mkdir -p "$WORK/bin"
cat > "$WORK/bin/rustup" <<'EOF'
#!/usr/bin/env bash
sub=""
for a in "$@"; do case "$a" in audit|deny|vet|geiger|machete) sub="$a";; esac; done
case "$sub" in
  audit) exit "${AUDIT_EXIT:-0}";;
  deny) exit "${DENY_EXIT:-0}";;
  vet) exit "${VET_EXIT:-0}";;
  geiger) exit "${GEIGER_EXIT:-0}";;
  machete) exit "${MACHETE_EXIT:-0}";;
esac
exit 0
EOF
chmod +x "$WORK/bin/rustup"
export PATH="$WORK/bin:$PATH"

# Create all 11 package fixture directories (matching actual workspace packages)
mkdir -p "$WORK/crates/workspace_tests" "$WORK/fuzz"
for p in vb_compile vb_core vb_expr vb_yaml vb_ipc vb_runtime vb_storage vb_validate velvet_ballistics; do
  mkdir -p "$WORK/crates/$p"
done

cd "$WORK"

echo ""
echo "=== FULL ACTUAL SCRIPT: all tools exit 0 -> task exits 0 ==="
AUDIT_EXIT=0 DENY_EXIT=0 VET_EXIT=0 GEIGER_EXIT=0 MACHETE_EXIT=0 bash actual_supply_chain.sh > "$EVIDENCE/allok.out" 2> "$EVIDENCE/allok.err"
rc=$?
echo "task exit code: $rc (expect 0)"
[[ "$rc" == "0" ]] && pass "all tools ok -> exit 0" || fail "all tools ok got exit $rc"
[[ "$(cat target/supply-chain/audit.status)" == "0" ]] && pass "audit.status=0" || fail "audit.status=$(cat target/supply-chain/audit.status 2>/dev/null)"
n_geiger=$(cat target/supply-chain/geiger/*/geiger.status 2>/dev/null | sort -u | tr '\n' ' ')
[[ "$n_geiger" == "0 " ]] && pass "all 11 geiger statuses=0" || fail "geiger statuses: $n_geiger"
cnt=$(ls target/supply-chain/geiger/*/geiger.status 2>/dev/null | wc -l)
[[ "$cnt" == "11" ]] && pass "11 per-package geiger status files" || fail "geiger count=$cnt (expect 11)"

echo ""
echo "=== FULL ACTUAL SCRIPT: advisory tools fail, audit ok -> task exits 0 ==="
rm -rf "$WORK/target"
AUDIT_EXIT=0 DENY_EXIT=1 VET_EXIT=2 GEIGER_EXIT=1 MACHETE_EXIT=1 bash actual_supply_chain.sh > "$EVIDENCE/advisory_fail.out" 2> "$EVIDENCE/advisory_fail.err"
rc=$?
echo "task exit code: $rc (expect 0, advisory failures do not fail task)"
[[ "$rc" == "0" ]] && pass "advisory failures -> exit 0" || fail "advisory failures got exit $rc"
[[ "$(cat target/supply-chain/deny.status)" == "1" ]] && pass "deny.status=1 (actual exit code preserved)" || fail "deny.status=$(cat target/supply-chain/deny.status 2>/dev/null)"
[[ "$(cat target/supply-chain/vet.status)" == "2" ]] && pass "vet.status=2" || fail "vet.status=$(cat target/supply-chain/vet.status 2>/dev/null)"
[[ "$(cat target/supply-chain/machete.status)" == "1" ]] && pass "machete.status=1" || fail "machete.status=$(cat target/supply-chain/machete.status 2>/dev/null)"
n_geiger=$(cat target/supply-chain/geiger/*/geiger.status 2>/dev/null | sort -u | tr '\n' ' ')
[[ "$n_geiger" == "1 " ]] && pass "all 11 geiger statuses=1 (real exit, not relabeled)" || fail "geiger statuses: $n_geiger"

echo ""
echo "=== FULL ACTUAL SCRIPT: audit exit 1 -> task exits 1 (fail-closed) ==="
rm -rf "$WORK/target"
AUDIT_EXIT=1 DENY_EXIT=0 VET_EXIT=0 GEIGER_EXIT=0 MACHETE_EXIT=0 bash actual_supply_chain.sh > "$EVIDENCE/audit1.out" 2> "$EVIDENCE/audit1.err"
rc=$?
echo "task exit code: $rc (expect 1)"
[[ "$rc" == "1" ]] && pass "audit exit 1 -> task exit 1" || fail "audit exit 1 got task exit $rc"
[[ "$(cat target/supply-chain/audit.status)" == "1" ]] && pass "audit.status=1" || fail "audit.status=$(cat target/supply-chain/audit.status 2>/dev/null)"

echo ""
echo "=== FULL ACTUAL SCRIPT: audit exit 3 -> task exits 3 (fail-closed) ==="
rm -rf "$WORK/target"
AUDIT_EXIT=3 DENY_EXIT=0 VET_EXIT=0 GEIGER_EXIT=0 MACHETE_EXIT=0 bash actual_supply_chain.sh > "$EVIDENCE/audit3.out" 2> "$EVIDENCE/audit3.err"
rc=$?
echo "task exit code: $rc (expect 3)"
[[ "$rc" == "3" ]] && pass "audit exit 3 -> task exit 3" || fail "audit exit 3 got task exit $rc"
[[ "$(cat target/supply-chain/audit.status)" == "3" ]] && pass "audit.status=3" || fail "audit.status=$(cat target/supply-chain/audit.status 2>/dev/null)"

echo ""
echo "=== FUNCTION-LEVEL: real run_advisory_report exit-code capture ==="
# Extract just the function from the actual script and source it
python3 - "$REPO_ROOT/.moon/tasks/all.yml" "$WORK/fn_extracted.sh" <<'PYEOF'
import sys
lines = open(sys.argv[1]).read().splitlines()
out = open(sys.argv[2], 'w')
i = 0
while i < len(lines) and lines[i] != '  supply-chain:':
    i += 1
while i < len(lines) and lines[i] != '    script: |':
    i += 1
i += 1
body = []
while i < len(lines):
    line = lines[i]
    if line.startswith('    ') and not line.startswith('      '):
        break
    body.append(line[6:] if line.startswith('      ') else line)
    i += 1
script = '\n'.join(body)
start = script.find('run_advisory_report()')
if start != -1:
    # Find matching closing brace
    depth = 0
    i = start
    while i < len(script):
        if script[i] == '{': depth += 1
        elif script[i] == '}':
            depth -= 1
            if depth == 0:
                out.write(script[start:i+1])
                break
        i += 1
out.close()
PYEOF

source "$WORK/fn_extracted.sh"
mkdir -p "$WORK/target/supply-chain"

for code in 0 1 2; do
  rm -f "$WORK/target/supply-chain/advisory_$code.status"
  mkdir -p "$WORK/stubs"
  cat > "$WORK/stubs/tool_$code" <<EOF
#!/usr/bin/env bash
exit $code
EOF
  chmod +x "$WORK/stubs/tool_$code"
  run_advisory_report "advisory_$code" "$WORK/stubs/tool_$code"
  actual_rc=$(cat "$WORK/target/supply-chain/advisory_$code.status" 2>/dev/null)
  [[ "$actual_rc" == "$code" ]] && pass "run_advisory_report preserves exit $code" || fail "run_advisory_report expected $code got $actual_rc"
done

# Test 124 with real timeout
rm -f "$WORK/target/supply-chain/advisory_124.status"
run_advisory_report "advisory_124" timeout 1 sleep 3
actual_rc=$(cat "$WORK/target/supply-chain/advisory_124.status" 2>/dev/null)
[[ "$actual_rc" == "124" ]] && pass "run_advisory_report preserves exit 124 (timeout)" || fail "run_advisory_report expected 124 got $actual_rc"

# Test 127 with actual missing binary
rm -f "$WORK/target/supply-chain/advisory_127.status"
run_advisory_report "advisory_127" no-such-tool-xyz
actual_rc=$(cat "$WORK/target/supply-chain/advisory_127.status" 2>/dev/null)
[[ "$actual_rc" == "127" ]] && pass "run_advisory_report preserves exit 127 (missing)" || fail "run_advisory_report expected 127 got $actual_rc"

echo ""
echo "=== RESULTS ==="
echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [[ $FAIL -gt 0 ]]; then
  exit 1
fi
echo "All supply-chain script tests passed."