bead_id: vb-qi37.4.3
bead_title: runtime/storage: Persist run header before acknowledgement
phase: State 14 - final manual QA
updated_at: 2026-05-12T03:15:00Z

STATUS: PASS

## Manual QA Final Report

### Test Matrix

| ID | Category | Command/Request | Expected | Actual | Status |
|----|----------|----------------|----------|--------|--------|
| REC-HEADER-001 | Happy Path | `cargo test -p velvet_ballastics --test admission_evidence_integration restart_lookup_finds_persisted_header` | PASS | `1 passed, 7 filtered out` | PASS |
| TEST-DUR-001 | Failure Path | `cargo test -p velvet_ballastics --test admission_evidence_integration storage_failure_before_header_prevents_ack` | PASS | `1 passed, 7 filtered out` | PASS |
| TEST-PRE-001 | Duplicate Rejection | `cargo test -p vb_runtime shard::tests::submit_rejects_duplicate_run_id` | PASS | `1 passed, 1441 filtered out` | PASS |
| TEST-PRE-002 | Admission Rejection | `cargo test -p vb_runtime admission_rejection_does_not_insert_run_state` | PASS | `1 passed, 1441 filtered out` | PASS |
| DUR-ACK | Durability Before Ack | `cargo test -p vb_runtime submit_direct_returns_durability_error_before_ack_when_header_cannot_persist` | PASS | `1 passed, 1441 filtered out` | PASS |
| Full Suite | Integration | `cargo test -p velvet_ballastics --test admission_evidence_integration` | All 8 Pass | `8 passed (1 suite, 0.05s)` | PASS |
| Moon CI | Release Gate | `moon ci` | All tasks pass | `Tasks: 19 completed (2 cached) Time: 54s 987ms` | PASS |

### Verbatim Command Evidence

#### 1. Happy Path: Persisted Header / Restart Lookup

**Command:**
```
cargo test -p velvet_ballastics --test admission_evidence_integration restart_lookup_finds_persisted_header
```
**Output:**
```
cargo test: 1 passed, 7 filtered out (1 suite, 0.01s)
```

#### 2. Failure Before Header Prevents Ack

**Command:**
```
cargo test -p velvet_ballastics --test admission_evidence_integration storage_failure_before_header_prevents_ack
```
**Output:**
```
cargo test: 1 passed, 7 filtered out (1 suite, 0.00s)
```

#### 3. Duplicate Submit Rejection

**Command:**
```
cargo test -p vb_runtime shard::tests::submit_rejects_duplicate_run_id
```
**Output:**
```
cargo test: 1 passed, 1441 filtered out (7 suites, 0.01s)
```

#### 4. Admission Rejection Before State Allocation

**Command:**
```
cargo test -p vb_runtime admission_rejection_does_not_insert_run_state
```
**Output:**
```
cargo test: 1 passed, 1441 filtered out (7 suites, 0.00s)
```

#### 5. Durability Error Before Ack

**Command:**
```
cargo test -p vb_runtime submit_direct_returns_durability_error_before_ack_when_header_cannot_persist
```
**Output:**
```
cargo test: 1 passed, 1441 filtered out (7 suites, 0.01s)
```

#### 6. Full Admission Evidence Integration Suite

**Command:**
```
cargo test -p velvet_ballastics --test admission_evidence_integration
```
**Output:**
```
cargo test: 8 passed (1 suite, 0.05s)
```

#### 7. Moon CI Release Gate

**Command:**
```
moon ci
```
**Output:**
```
Tasks: 19 completed (2 cached)
 Time: 54s 987ms
```

### Findings

All tests passed. The durability/admission/header workflow is correct:

1. **Happy Path**: `restart_lookup_finds_persisted_header` confirms that after successful submit, the run header/admission metadata is durably persisted and recoverable by run id and digest.

2. **Failure Before Header Prevents Ack**: `storage_failure_before_header_prevents_ack` confirms that if journal append for header/admission fails, the exact durability RuntimeError is returned and no active run exists.

3. **Duplicate Submit Rejection**: `submit_rejects_duplicate_run_id` confirms that submitting with an existing run id returns `Err(RuntimeError::RunAlreadyExists)` before allocating a second runtime state.

4. **Admission Rejection**: `admission_rejection_does_not_insert_run_state` confirms that admission policy rejection returns a typed RuntimeError before runtime state allocation.

### Summary

- Total tests executed: 7 distinct test commands + moon ci
- PASS: 7
- FAIL: 0
- Severity breakdown: N/A (no failures)
- Moon CI: PASS (19 tasks, 54s)