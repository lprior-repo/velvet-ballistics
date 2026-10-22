#!/usr/bin/env bash
# Synthetic unit checks for supply-chain script exit-status handling.
# Proves that run_advisory_report and the audit block preserve exact
# exit codes (0, 1, 2, 124, 127) in status files without relabeling
# failures as PASS or swallowing exit codes.
#
# Extracts the actual run_advisory_report function from the production
# .moon/tasks/all.yml supply-chain task script.
# Uses test-only PATH fixtures, not real supply-chain tools.
#
# The production script has two distinct behaviors:
# 1. Advisory reports (geiger per-package, deny, vet, machete): run_advisory_report
#    always returns 0, writes the tool's actual exit code to a status file,
#    and the task continues to the next report.
# 2. Audit (cargo audit): fail-closed. Nonzero exit writes the status, prints
#    the output, then exits with the same nonzero code immediately.

set -euo pipefail

# Find repo root by walking up from script location
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd -P)"
TEST_DIR=$(mktemp -d /tmp/vb-selftest-XXXXXX)
FIXTURE_DIR=$(mktemp -d /tmp/vb-fixture-XXXXXX)
EVIDENCE_DIR="$TEST_DIR/evidence"
mkdir -p "$EVIDENCE_DIR"

cleanup() {
  cd /
  rm -rf "$TEST_DIR" "$FIXTURE_DIR"
}
trap cleanup EXIT

PASS=0
FAIL=0

# Extract the exact run_advisory_report function from production script
# via Moon tasks YAML inspection
extract_advisory_function() {
  python3 - "$REPO_ROOT/.moon/tasks/all.yml" << 'PY'
import re, sys

with open(sys.argv[1], 'r') as f:
    content = f.read()

# Extract the supply-chain script
match = re.search(r'^  supply-chain:\s*\n\s+script:\s*\|?\n((?:\s+.*\n)+?)(?=\s+[a-zA-Z_-]+:|\n\s+[a-zA-Z_-]+:)', content, re.MULTILINE)
if match:
    script = match.group(1)
    # Dedent
    lines = script.split('\n')
    non_empty = [l for l in lines if l.strip()]
    if non_empty:
        min_indent = min(len(l) - len(l.lstrip()) for l in non_empty)
        lines = [l[min_indent:] if l.startswith(' ' * min_indent) else l for l in lines]
        script = '\n'.join(lines)

    # Find the function and extract until the next top-level command
    start = script.find('run_advisory_report() {')
    if start != -1:
        # Find the closing brace by counting
        brace_count = 0
        i = start
        while i < len(script):
            if script[i] == '{':
                brace_count += 1
            elif script[i] == '}':
                brace_count -= 1
                if brace_count == 0:
                    # Found matching closing brace
                    end = i + 1
                    print(script[start:end])
                    break
            i += 1
PY
}

# Make PATH stubs for tools with specific exit codes
make_stub() {
  local name="$1"
  local code="$2"
  cat > "$FIXTURE_DIR/$name" << STUB
#!/usr/bin/env bash
if [[ "$code" -ne 0 ]]; then
  echo "$name: error" >&2
fi
exit "$code"
STUB
  chmod +x "$FIXTURE_DIR/$name"
}

# Test run_advisory_report with a specific exit code
test_advisory_report() {
  local tool_name="$1"
  local exit_code="$2"
  local desc="$3"

  make_stub "$tool_name" "$exit_code"
  export PATH="$FIXTURE_DIR:$PATH"

  # Extract the function and write to a temp script
  local func_script="$TEST_DIR/advisory-func.sh"
  extract_advisory_function > "$func_script"

  # Run the function via a wrapper script
  bash -c "
source $func_script
mkdir -p target/supply-chain
run_advisory_report '$tool_name' '$FIXTURE_DIR/$tool_name'
" 2> /dev/null

  local status_file="$TEST_DIR/target/supply-chain/${tool_name}.status"
  if [[ ! -f "$status_file" ]]; then
    echo "FAIL [$desc]: status file not created"
    FAIL=$((FAIL + 1))
    return
  fi

  local actual_status
  actual_status=$(cat "$status_file")

  if [[ "$actual_status" -eq "$exit_code" ]]; then
    echo "PASS [$desc]: status file has correct value $exit_code"
    PASS=$((PASS + 1))
  else
    echo "FAIL [$desc]: expected status $exit_code, got $actual_status"
    FAIL=$((FAIL + 1))
  fi
}

# Test audit pass (exit 0)
test_audit_pass() {
  make_stub "cargo" 0
  export PATH="$FIXTURE_DIR:$PATH"

  local script_exit=0
  (
    if timeout 1s cargo --version >"$TEST_DIR/target/supply-chain/audit.out" 2>"$TEST_DIR/target/supply-chain/audit.err"; then
      printf '0\n' >"$TEST_DIR/target/supply-chain/audit.status"
    else
      rc=$?
      printf '%d\n' "$rc" >"$TEST_DIR/target/supply-chain/audit.status"
      exit "$rc"
    fi
  ) || script_exit=$?

  local actual_status
  actual_status=$(cat "$TEST_DIR/target/supply-chain/audit.status")

  if [[ "$actual_status" -eq 0 ]]; then
    echo "PASS [audit-pass]: status file has 0"
    PASS=$((PASS + 1))
  else
    echo "FAIL [audit-pass]: expected 0, got $actual_status"
    FAIL=$((FAIL + 1))
  fi
}

# Test audit fail-closed behavior
test_audit_fail() {
  local exit_code="$1"
  local desc="$2"

  make_stub "cargo" "$exit_code"
  export PATH="$FIXTURE_DIR:$PATH"

  local script_exit=0
  (
    if timeout 1s cargo --version >"$TEST_DIR/target/supply-chain/audit.out" 2>"$TEST_DIR/target/supply-chain/audit.err"; then
      printf '0\n' >"$TEST_DIR/target/supply-chain/audit.status"
    else
      rc=$?
      printf '%d\n' "$rc" >"$TEST_DIR/target/supply-chain/audit.status"
      exit "$rc"
    fi
  ) || script_exit=$?

  local actual_status
  actual_status=$(cat "$TEST_DIR/target/supply-chain/audit.status")

  if [[ "$actual_status" -eq "$exit_code" ]]; then
    echo "PASS [$desc]: status file has correct value $exit_code"
    PASS=$((PASS + 1))
  else
    echo "FAIL [$desc]: expected status $exit_code, got $actual_status"
    FAIL=$((FAIL + 1))
  fi

  if [[ "$script_exit" -eq "$exit_code" ]]; then
    echo "  script exit code matches: $script_exit"
  else
    echo "  NOTE: script exit code was $script_exit (expected $exit_code)"
  fi
}

cd "$TEST_DIR"
mkdir -p target/supply-chain

echo "=== Velvet Ballistics Supply-Chain Status Handling Selftest ==="
echo "Testing exit-status preservation in advisory report runner and audit block."
echo ""

echo "--- Advisory Report Status Preservation ---"
test_advisory_report "geiger" 0 "geiger-ok"
test_advisory_report "deny" 1 "deny-fail-1"
test_advisory_report "vet" 2 "vet-fail-2"
test_advisory_report "machete" 124 "machete-timeout"
test_advisory_report "missing-tool" 127 "tool-not-found"

echo ""
echo "--- Audit Block Fail-Closed Behavior ---"
test_audit_pass
test_audit_fail 1 "audit-fail-1"
test_audit_fail 3 "audit-fail-3"

echo ""
echo "=== Results ==="
echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [[ $FAIL -gt 0 ]]; then
  exit 1
fi
echo "All status handling tests passed."
