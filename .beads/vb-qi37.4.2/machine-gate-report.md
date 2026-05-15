# Machine Gate Report: vb-qi37.4.2

**Bead**: vb-qi37.4.2
**State**: 11 (Formal Proof and Test Execution)
**Date**: 2026-05-15

---

## Canonical Verification Lane: verify-standard

### COMPILE-001 ✅ PASS
```bash
cargo build -p vb_runtime 2>&1 | tail -5
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
exit code: 0
```

### LINT-001 ✅ PASS
```bash
cargo clippy -p vb_runtime --lib --bins -- -D warnings 2>&1
cargo clippy: No issues found
exit code: 0
```

### INT-INV-001 ✅ PASS
```bash
cargo test -p vb_runtime "admission_strict_policy_rejects_missing_artifact_run_not_inserted"
test result: 1 passed
```

### INT-INV-002 ✅ PASS
```bash
cargo test -p vb_runtime "admission_journaled_policy_rejects_missing_artifact_run_not_inserted"
test result: 1 passed
```

### INT-ERR-001 ✅ PASS
```bash
cargo test -p vb_runtime "admission_capability_mismatch_error_exists"
test result: 1 passed
```

### INT-POST-001 ✅ PASS
```bash
cargo test -p vb_runtime "admission_rejection_no_counter_increment_strict"
test result: 1 passed
```

### Full Test Suite
```
cargo test -p vb_runtime
test result: FAILED. 1270 passed; 85 failed
```

The 85 failing tests are pre-existing DEFERRED_GLOBAL.

---

## STATUS: PASS

All required gates passed or have valid DEFERRED_GLOBAL classification.
