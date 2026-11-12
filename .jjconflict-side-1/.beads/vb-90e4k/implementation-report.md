# vb-90e4k Implementation Report

## Summary
Fixed error mapping for snapshot run_id mismatch: `hydrate_run_frame` now returns
`RecoveryError::CorruptSnapshot { run, seq }` instead of `ReplayDivergence` when
the snapshot's run_id differs from the expected run being recovered.

## Changes

### Production (1 file)
- `crates/vb_storage/src/recovery/hydrate.rs`: Modified `snapshot_input_violation_to_error`
  to map `SnapshotRecoveryInputViolation::SnapshotRunMismatch` to `CorruptSnapshot`
  with `run=expected_run` and `seq=snapshot_seq`. Used `..` pattern to discard
  the unused `snapshot_run` binding in the error construction.

### Tests (2 files)
- `crates/vb_runtime/tests/recovery_hydration_tests.rs`: Strengthened
  `corrupt_snapshot_run_mismatch_rejected` to assert exact `run` and `seq`
  fields instead of wildcards.
- `crates/vb_runtime/tests/recovery_bdd_tests.rs`: Strengthened
  `corrupt_snapshot_returns_corrupt_snapshot_error` to assert exact `run` and
  `seq` fields instead of named wildcards.

## Test Results

### Bounded recovery tests
- `recovery_bdd_tests`: 82 passed, 0 failed, 0 ignored
- `recovery_hydration_tests`: 43 passed, 0 failed, 1 ignored (crash_child_marker)

### Full runtime test suite
- 34 test binaries
- **2445 passed, 0 failed, 1 ignored**
- Baseline was 2437 passed, 2 failed, 1 ignored; both previously-failing tests now pass

## Preservation Checks
- `incremental_recovery_rejects_tail_event_before_watermark` still returns
  `ReplayDivergence` (unchanged tail path)
- No changes to math, proofs, or verification harnesses
- No new test files or invented APIs

## Evidence
- `.beads/vb-90e4k/evidence/recovery-errors.log` — bounded recovery test output
- `.beads/vb-90e4k/evidence/runtime-package.log` — full runtime test output
