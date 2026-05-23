# Test Suite Review: vb-jpq7.3

STATUS: APPROVED

## Findings

1. **Resolved: snapshot key/payload authority agreement now has direct behavior coverage.**
   - `crates/vb_storage/src/trimming/tests.rs:362` covers payload run mismatch and asserts exact `TrimError::Journal(JournalError::WrongRun { expected, actual })` fields plus diagnostic code.
   - `crates/vb_storage/src/trimming/tests.rs:394` covers payload sequence mismatch and asserts exact `TrimError::Journal(JournalError::SequenceGap { expected, actual })` fields plus diagnostic code.
   - These tests kill mutations that delete the `snapshot.run != run` or `snapshot.seq != key_seq` checks in `trimming/logic.rs:37-48`.

2. **Non-blocking note: workspace source-scan tests remain supplemental, not primary proof.**
   - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:159-196` scans strings for `latest_durable_snapshot_seq(run)?` and `RecoveryError::SlotTaintReadFailed`.
   - The taint path has a direct internal behavior test at `crates/vb_storage/src/recovery/tests.rs:2077`, so that scan is supplemental.
   - The repaired direct tests cover the previously missing behavior. The source scans no longer carry the behavior proof alone.

3. **Resolved: latest snapshot decode failures fail closed before tail replay.**
   - `crates/vb_storage/src/journal/tests.rs:1786` asserts `JournalError::PayloadDigestMismatch` for a corrupt latest snapshot payload while a valid tail event exists.
   - `crates/vb_storage/src/journal/tests.rs:1831` asserts `JournalError::PostcardDecodeFailed` for an undecodable latest snapshot payload while a valid tail event exists.
   - Existing `crates/vb_storage/src/journal/tests.rs:1743` covers bad magic; these collectively exercise decode-family fail-closed behavior before any tail replay can launder corruption.

## Executed Targeted Checks

- `rtk cargo test -p vb_storage latest_durable_snapshot_seq --all-features` — PASS, 4 tests.
- `rtk cargo test -p vb_storage events_for_run_rejects_latest_snapshot --all-features` — PASS, 2 tests.
- `rtk cargo test -p vb_storage events_for_run --all-features` — PASS, 24 tests.
- `rtk cargo test -p velvet-ballastics-workspace-tests --test vb_jpq7_3_fail_closed_storage_recovery_contract` — PASS, 9 tests.
- `rtk cargo test -p vb_storage close_propagates_persist_errors --all-features` — PASS, 1 test.
- `rtk cargo test -p vb_storage apply_tail_events_fails_closed_when_taint_read_fails --all-features` — PASS, 1 test.

## Remaining Test Gaps

No blocking vb-jpq7.3 behavior-test gaps remain in the inspected scope. Tests are deterministic, use direct behavior assertions for the repaired fail-closed paths, assert exact typed errors where variants carry fields, and include mutation-resistant checks for snapshot authority, replay bounds, tail gaps, latest-snapshot corruption, explicit close error propagation, and taint read fail-closed recovery.
