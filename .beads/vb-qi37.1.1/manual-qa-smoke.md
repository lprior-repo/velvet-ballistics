# Manual QA Smoke Report — vb-qi37.1.1

**Bead:** vb-qi37.1.1
**Title:** runtime/recovery: Journal deterministic step lifecycle
**State:** IN_PROGRESS
**Date:** 2026-05-09

## Contract Evidence

**File:** `.beads/vb-qi37.1.1/contract.md`
**Status:** NOT FOUND

```
$ ls -la .beads/vb-qi37.1.1/
ls: cannot access '.beads/vb-qi37.1.1/': No such file or directory
```

Bead exists in Dolt remote but no local artifact directory.

## Test Plan Evidence

**File:** `.beads/vb-qi37.1.1/test-plan.md`
**Status:** NOT FOUND

## Implementation Evidence

**File:** `.beads/vb-qi37.1.1/implementation.md`
**Status:** NOT FOUND

## Test Execution

**Command:**
```bash
cargo nextest run --test vb_qi37_1_1_red_recovery_contract_test
```

**Output:**
```
error: no test target named `vb_qi37_1_1_red_recovery_contract_test` in default-run packages
help: available test targets:
    diagnostic_code_ranges_test
    phase0_scaffold_test
    vb_fzx7_benchmark_groups
    vb_fzx7_budget_arithmetic
    vb_fzx7_error_variants
    vb_fzx7_evidence_gate
    vb_fzx7_invariants
error: command `... cargo test --no-run ... --test vb_qi37_1_1_red_recovery_contract_test` exited with code 101
```

**Exit Code:** 101 (compilation error — test target not found)

## Findings

### CRITICAL — Test Does Not Exist
- **Evidence:** `cargo nextest run --test vb_qi37_1_1_red_recovery_contract_test` → exit 101, "no test target named `vb_qi37_1_1_red_recovery_contract_test`"
- **Bead contract.md, test-plan.md, implementation.md:** Nonexistent — no local files
- **Action Required:** Implement the test before smoke can pass

### CRITICAL — Compilation Errors in Test Suite
- **Evidence:** `cargo nextest run --test vb_fzx7_evidence_gate` fails to compile with 13 errors
  - `error[E0609]: no field 'environment' on type 'Result<BenchmarkMetadata, EvidenceError>'`
  - `error[E0609]: no field 'budget_us' on type 'Result<BenchmarkMetadata, EvidenceError>'`
  - `error[E0609]: no field 'baseline_us' on type 'Result<BenchmarkMetadata, EvidenceError>'`
  - `error[E0609]: no field 'result_us' on type 'Result<BenchmarkMetadata, EvidenceError>'`
  - `error[E0382]: borrow of moved value: 'result'`
- **Action Required:** Fix vb_fzx7_evidence_gate.rs compilation errors before any nextest tests can run

## VERDICT: FAIL

**Reason:** Requested test `vb_qi37_1_1_red_recovery_contract_test` does not exist. Bead has no local artifacts. Downstream `vb_fzx7_evidence_gate` test suite fails to compile (13 errors), blocking all test execution.

---
STATUS: FAIL
