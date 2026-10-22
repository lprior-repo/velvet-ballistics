#!/usr/bin/env bash
# Synthetic unit checks for supply-chain script exit-status handling.
# Proves that run_advisory_report and the audit block preserve exact
# exit codes (0, 1, 2, 124, 127) in status files without relabeling
# failures as PASS or swallowing exit codes.
#
# Uses test-only PATH fixtures, not real supply-chain tools.
# Actual Moon supply-chain task remains the production verification.

set -euo pipefail

PASS=0
FAIL=0

TEST_DIR=$(mktemp -d /tmp/vb-selftest-XXXXXX)
FIXTURE_DIR=$(mktemp -d /tmp/vb-fixture-XXXXXX)
cd "$TEST_DIR"
mkdir -p target/supply-chain

cleanup() {
  cd /
  rm -rf "$TEST_DIR" "$FIXTURE_DIR"
}
trap cleanup EXIT

make_fixture() {
  local name="$1"
  local code="$2"
  cat > "$FIXTURE_DIR/$name" <<EOF
#!/usr/bin/env bash
echo "fixture-$name output"
if [[ "$code" -ne 0 ]]; then
  echo "fixture-$name error" >&2
fi
exit "$code"
EOF
  chmod +x "$FIXTURE_DIR/$name"
}

# The exact run_advisory_report function from production supply-chain script
run_advisory_report() {
  tool_name="$1"
  shift
  local out_file="target/supply-chain/${tool_name}.out"
  local err_file="target/supply-chain/${tool_name}.err"
  local status_file="target/supply-chain/${tool_name}.status"

  printf 'Running advisory report: %s\n' "$tool_name" >&2
  if "$@" >"$out_file" 2>"$err_file"; then
    rc=0
  else
    rc=$?
  fi
  printf '%d\n' "$rc" >"$status_file"
  printf 'Advisory report %s completed with status %d\n' "$tool_name" "$rc" >&2
  return 0
}

test_advisory_status() {
  local tool_name="$1"
  local exit_code="$2"
  local desc="$3"

  make_fixture "$tool_name" "$exit_code"

  # Call the fixture by its full path in the fixture dir
  run_advisory_report "$tool_name" "$FIXTURE_DIR/$tool_name"

  local status_file="target/supply-chain/${tool_name}.status"
  local out_file="target/supply-chain/${tool_name}.out"
  local err_file="target/supply-chain/${tool_name}.err"

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

  if [[ -f "$out_file" ]]; then
    echo "  stdout captured: $(wc -c < "$out_file") bytes"
  fi
  if [[ -f "$err_file" ]]; then
    echo "  stderr captured: $(wc -c < "$err_file") bytes"
  fi
}

test_audit_pass() {
  make_fixture "cargo" 0
  export PATH="$FIXTURE_DIR:$PATH"

  if timeout 1s cargo --version >target/supply-chain/audit.out 2>target/supply-chain/audit.err; then
    printf '0\n' >target/supply-chain/audit.status
  else
    rc=$?
    printf '%d\n' "$rc" >target/supply-chain/audit.status
    exit "$rc"
  fi

  local actual_status
  actual_status=$(cat target/supply-chain/audit.status)

  if [[ "$actual_status" -eq 0 ]]; then
    echo "PASS [audit-pass]: status file has 0"
    PASS=$((PASS + 1))
  else
    echo "FAIL [audit-pass]: expected 0, got $actual_status"
    FAIL=$((FAIL + 1))
  fi
}

test_audit_fail() {
  local exit_code="$1"
  local desc="$2"

  make_fixture "cargo" "$exit_code"
  export PATH="$FIXTURE_DIR:$PATH"

  local script_exit=0
  (
    if timeout 1s cargo --version >target/supply-chain/audit.out 2>target/supply-chain/audit.err; then
      printf '0\n' >target/supply-chain/audit.status
    else
      rc=$?
      printf '%d\n' "$rc" >target/supply-chain/audit.status
      exit "$rc"
    fi
  ) || script_exit=$?

  local actual_status
  actual_status=$(cat target/supply-chain/audit.status)

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

echo "=== Velvet Ballistics Supply-Chain Status Handling Selftest ==="
echo "Testing exit-status preservation in advisory report runner and audit block."
echo ""

echo "--- Advisory Report Status Preservation ---"
test_advisory_status "geiger" 0 "geiger-ok"
test_advisory_status "deny" 1 "deny-fail-1"
test_advisory_status "vet" 2 "vet-fail-2"
test_advisory_status "machete" 124 "machete-timeout"
test_advisory_status "missing-tool" 127 "tool-not-found"

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
