# vb-90e4k: Snapshot Run Mismatch Error Mapping

## Problem
`hydrate_run_frame` returns `RecoveryError::ReplayDivergence` for snapshot run_id
mismatch. The intended behavior is to return `RecoveryError::CorruptSnapshot` for
this case, distinguishing it from tail/journal replay divergence.

## Expected Behavior
When the snapshot's `run_id` differs from the expected run being recovered:
- Return `CorruptSnapshot { run: expected_run, seq: snapshot_seq }`
- The `run` field is the expected run (not the one in the snapshot)
- The `seq` field is the snapshot's sequence number
- Tail validation errors (TailRunMismatch, TailSeqNotAfterSnapshot, TailSeqGap)
  continue to return `ReplayDivergence`

## Implementation
- File: `crates/vb_storage/src/recovery/hydrate.rs`
- Function: `snapshot_input_violation_to_error`
- Change: Map `SnapshotRunMismatch` to `CorruptSnapshot` instead of `ReplayDivergence`
- Use `..` pattern for unused `snapshot_run` binding in the error variant

## Test Strategy
- Strengthen existing test `recovery_hydration_tests::corrupt_snapshot_run_mismatch_rejected`
  to assert exact `run` and `seq` fields (not wildcards)
- Strengthen existing test `recovery_bdd_tests::corrupt_snapshot_returns_corrupt_snapshot_error`
  to assert exact `run` and `seq` fields
- Verify that `recovery_hydration_tests::incremental_recovery_rejects_tail_event_before_watermark`
  still returns `ReplayDivergence` (unchanged tail path)
- No new test files; no invented test IDs or APIs

## Scope
- Production: `hydrate.rs` only
- Tests: the two existing failing integration tests only
- No math, no proof, no verification harness changes
