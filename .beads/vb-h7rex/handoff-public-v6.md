# PUBLIC V6 Repair Author Handoff

**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-vb-h7rex-local5090-public-recovery`
**Bead:** vb-h7rex (policy pin durable recovery)
**Worker:** Local5090 repair author
**Date:** 2026-10-22

## Summary

Complete repair of the policy pin durable recovery integration test suite. All four gates pass. No production changes needed.

## Changes Made

Rewrote `crates/vb_runtime/tests/policy_pin_durable_recovery.rs` with the following fixes:

1. **ShardConfig policy setting:** Both tests now use `shard_config_for(policy)` which sets `ShardConfig.policy` explicitly. The Strict test uses `RuntimePolicy::Strict`, the Journaled test uses `RuntimePolicy::Journaled`. This fixes the original V5 bug where `ShardConfig::default()` (Strict) was used for both.

2. **Journaled recovery cycle:** The Journaled test now completes the full durable recovery cycle: submit artifact, admit run, fail action, capture checkpoint, drop all handles, cold reopen journal, recover_retry, tick, complete action, and verify pin retention. The original V5 Journaled test only checked the first scope (admission) and did not perform the reopen/recover path.

3. **Scenario helper refactor:** The two positive recovery tests are refactored into a shared `policy_pin_survives_recovery_scenario(policy, run)` function parameterized by `RuntimePolicy`. Both Strict and Journaled exercise identical end-to-end paths.

4. **get_latest_checkpoint fix:** Now uses the actual `vb_runtime::run_checkpoint::decode_checkpoint_payload` version-aware decoder instead of raw postcard deserialization. This correctly rejects historical version 1 payloads.

5. **unwrap/expect elimination:** Replaced all `unwrap()` and `expect()` calls with proper `ok_or_else`, Result propagation, and pattern matching with typed error returns. No panics, no unsafe, no index/slice/arithmetic casts.

6. **Negative tampered pin test:** Added `tampered_policy_pin_rejected_by_recover_retry` test. Uses a test-local serde DTO (`ForgedRunAdmission`) with the same field layout as `RunAdmission` to construct a checkpoint with a forged policy pin (`WorkflowDigest::from_bytes([0xFF; 32])`). The `recover_retry` public API correctly rejects it with error "retry checkpoint does not match durable RetryCheckpoint evidence" and the journal is verified to be unmodified after rejection.

7. **Post-recovery pin verification:** Both positive recovery scenarios now verify that a new checkpoint produced after recovery (when the retried action completes) still contains the same policy pin, proving retention through the recovery path.

8. **Historical prefix preservation:** Both positive scenarios verify that the original `RunAdmission` event (seq 1) is still present in the final journal after recovery, confirming the journal prefix is preserved.

9. **RuntimeError formatting:** Fixed to use `{runtime_error:?}` (Debug) instead of `{runtime_error}` (Display), since `RuntimeError` only implements `Debug`.

10. **RunAdmission pin forgery:** Uses test-local serde DTO (`ForgedRunAdmission`) with the same layout as `RunAdmission` to produce a hostile checkpoint with a forged pin. The `with_policy_pin` method on `RunAdmission` is `pub(crate)` and cannot be called from integration tests.

## Test Results

| Gate | Command | Result |
|------|---------|--------|
| PUBLIC_V6_01 | `cargo test -p vb_runtime --test policy_pin_durable_recovery` | PASS (4/4) |
| PUBLIC_V6_02 | `cargo test -p vb_runtime checkpoint` | PASS |
| PUBLIC_V6_03 | `cargo clippy -p vb_runtime --lib -- -D warnings` | PASS |
| PUBLIC_V6_04 | `cargo test -p vb_runtime --test durable_retry_execution` | PASS (originals unchanged) |
| PUBLIC_V6_05 | `cargo test -p vb_runtime --test durable_retry_boundary` | PASS (originals unchanged) |

### New test suite results (PUBLIC_V6_01)

```
running 4 tests
test legacy_version_1_checkpoint_is_explicitly_rejected ... ok
test tampered_policy_pin_rejected_by_recover_retry ... ok
test strict_admission_policy_pin_survives_durable_recovery ... ok
test journaled_admission_policy_pin_survives_durable_recovery ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Tests by Category

### Positive Scenarios (2)
- `strict_admission_policy_pin_survives_durable_recovery`: End-to-end Strict recovery
- `journaled_admission_policy_pin_survives_durable_recovery`: End-to-end Journaled recovery

Both verify:
- RunAdmission produces `RunPolicyPin::Pinned(actual_policy_digest)`
- Policy pin equals `compute_policy_digest(workflow)` and differs from artifact digest
- Checkpoint carries the same policy pin
- Journal reopen preserves both admission pin and checkpoint pin
- recover_retry succeeds and produces run_attempt 2
- Completed retried action produces a new checkpoint with the same pin
- Original RunAdmission event (seq 1) remains in the final journal

### Negative Scenario (1)
- `tampered_policy_pin_rejected_by_recover_retry`: Forged checkpoint with `0xFF` policy pin is rejected with exact error message. Journal and original checkpoint verified unchanged.

### Version Rejection (1)
- `legacy_version_1_checkpoint_is_explicitly_rejected`: Version 1 payload rejected with `UnsupportedVersion` error.

## Production Changes

None. All changes are in the integration test file only.

## Known Gaps

None. All requirements from the repair scope are satisfied. The forged policy pin test exercises the actual public `recover_retry` API boundary via a serde DTO with the same layout as `RunAdmission`, since `with_policy_pin` is `pub(crate)` and the `admission` field on `RunCheckpointV1` is public and replaceable.