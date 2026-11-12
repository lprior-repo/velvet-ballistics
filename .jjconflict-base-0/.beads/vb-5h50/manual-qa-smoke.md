bead_id: vb-5h50
bead_title: storage: Trim journal events after durable snapshots
phase: state-7-manual-qa
updated_at: 2026-05-09T00:00:00Z

# Manual QA Smoke Report

## Target
- Crate: `vb_storage`
- Module: `trimming.rs`
- Interface: `FjallJournal::trim_events_for_run()`, `FjallJournal::trim_all_eligible_runs()`

## Test Matrix

| ID | Category | Test | Expected | Actual | Status |
|---|---|---|---|---|---|
| 1 | Happy Path | smoke_happy_path_trim | Trim 3 events, keep 3 | Trimmed 3, remaining 3 | PASS |
| 2 | Error Path | smoke_retention_policy_blocks | RetentionPolicyBlocks error | RetentionPolicyBlocks error | PASS |
| 3 | Error Path | smoke_no_snapshot_fails_closed | NoDurableSnapshot error | NoDurableSnapshot error | PASS |
| 4 | Idempotency | smoke_idempotency | First trim=Trimmed, second=NoOp | First=Trimmed, second=NoOp | PASS |

## Evidence

### Test 1: Happy Path Trim
```
running 4 tests
test smoke_happy_path_trim ... ok
test smoke_idempotency ... ok
test smoke_no_snapshot_fails_closed ... ok
test smoke_retention_policy_blocks ... ok
cargo test: 4 passed (1 suite, 0.01s)
```

### Test 2: Retention Policy Blocks
Command: `cargo test -p vb_storage --test manual_qa_smoke smoke_retention_policy_blocks -- --nocapture`
Result: `RetentionPolicyBlocks` error returned as expected. No events deleted.

### Test 3: No Snapshot Fails Closed
Command: `cargo test -p vb_storage --test manual_qa_smoke smoke_no_snapshot_fails_closed -- --nocapture`
Result: `NoDurableSnapshot` error returned as expected. No events deleted.

### Test 4: Idempotency
Command: `cargo test -p vb_storage --test manual_qa_smoke smoke_idempotency -- --nocapture`
Result: First trim deleted 2 events (status=Trimmed), second trim deleted 0 (status=NoOp).

## Findings

No CRITICAL or MAJOR findings. All happy paths and error paths behave as specified.

## Summary
- Total tests: 4
- PASS: 4
- FAIL: 0
- Status: All smoke tests green.

STATUS: PASS
