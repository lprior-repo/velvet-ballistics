# vb-90e4k Independent Code Review

Reviewer: Local 5090
Commit: a244efa285b0da825f31fdd87e2e86db96869619
Date: 2026-10-21

## Changed Files
- `crates/vb_storage/src/recovery/hydrate.rs` (production)
- `crates/vb_runtime/tests/recovery_bdd_tests.rs` (test)
- `crates/vb_runtime/tests/recovery_hydration_tests.rs` (test)

## Production Change Analysis

### `crates/vb_storage/src/recovery/hydrate.rs`

Lines 455-465 (new):
```rust
SnapshotRecoveryInputViolation::SnapshotRunMismatch {
    expected_run,
    snapshot_seq,
    ..
} => RecoveryError::CorruptSnapshot {
    run: expected_run,
    seq: snapshot_seq,
},
```

**Correctness:** Maps `SnapshotRunMismatch` to `CorruptSnapshot` with `run = expected_run` (the run being recovered, not the one in the snapshot) and `seq = snapshot_seq` (the snapshot's sequence number). This matches the contract exactly: "The run field is the expected run (not the one in the snapshot)" and "The seq field is the snapshot's sequence number."

**Tail divergence preserved:** Lines 466-490 map `TailRunMismatch`, `TailSeqNotAfterSnapshot`, and `TailSeqGap` to `ReplayDivergence` with descriptive detail strings. These paths are unchanged, preserving the distinction between snapshot corruption and tail replay issues.

**Pattern completeness:** The `..` pattern correctly captures unused `snapshot_run` field from `SnapshotRecoveryInputViolation::SnapshotRunMismatch`. No unused-variable warnings are generated.

## Test Change Analysis

### `crates/vb_runtime/tests/recovery_hydration_tests.rs` (lines 774-779)

```rust
assert!(matches!(
    result,
    Err(RecoveryError::CorruptSnapshot { run: err_run, seq: err_seq })
        if err_run == run && err_seq == EventSeq::new(1)
));
```

**Assertion strength:** Exact variant and exact field values (run and seq) are asserted via matches! guard. The previous `let Err(...) = result else { panic!(...) }` form only checked the variant, not the payload. The test provides `run = RunId::new(10500)` and `wrong_run = RunId::new(99999)` for the snapshot, so the guard correctly validates that `err_run` is the expected run (10500), not the snapshot's run (99999).

### `crates/vb_runtime/tests/recovery_bdd_tests.rs` (lines 1299-1304)

```rust
assert!(matches!(
    result,
    Err(RecoveryError::CorruptSnapshot { run: err_run, seq: err_seq })
        if err_run == run && err_seq == EventSeq::new(1)
));
```

**Assertion strength:** Same improvement. Provides `run = RunId::new(42)` and `wrong_run = RunId::new(999)` for the snapshot. Validates exact variant and payload.

### Mutation resistance assessment

If the production change were reverted (mapping `SnapshotRunMismatch` back to `ReplayDivergence`), both tests would fail because `matches!` with the `CorruptSnapshot` pattern would not bind, the guard would not execute, and `matches!` would return `false`. The old panic fallback blocks have been replaced with `matches!` guard assertions, eliminating the possibility of silent acceptance.

The test `incremental_recovery_rejects_tail_event_before_watermark` continues to verify that tail path errors return `ReplayDivergence`, preserving the snapshot/tail distinction.

## Findings

**No unresolved findings.** The implementation correctly maps the snapshot run_id mismatch to the `CorruptSnapshot` error variant with the expected run and snapshot sequence number, preserving the distinction between snapshot corruption and tail replay divergence. Test assertions have been strengthened from variant-only checks to exact field-value validation.

STATUS: APPROVED
