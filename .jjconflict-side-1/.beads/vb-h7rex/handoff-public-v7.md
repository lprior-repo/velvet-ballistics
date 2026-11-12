# Public V7 Recovery Handoff

## Completed Repairs

Two focused test defects in `crates/vb_runtime/tests/policy_pin_durable_recovery.rs` repaired.

### Repair 1: Eliminated forbidden expect/unwrap in forge_admission_with_pin

Changed `forge_admission_with_pin` return type from `RunAdmission` to `Result<RunAdmission, String>`. Both postcard serialization operations now use `map_err` and `?` propagation. Updated the single caller in `tampered_policy_pin_rejected_by_recover_retry` to propagate with `?`. Zero expect/unwrap/panic/todo remains in the file.

### Repair 2: Policy-aware runtime journal construction

Added test helper `runtime_journal_for_policy` that matches the actual `RuntimePolicy` to the correct `StorageRuntimeJournal` variant:
- `RuntimePolicy::Strict` → `shared_strict`
- `RuntimePolicy::Journaled` → `shared_journaled`
- Unsupported variants → explicit error

Applied at all four normal/reopened/negative-case runtime constructions in the test scenarios. `ShardConfig.policy` remains explicit.

## Test Outcomes

PUBLIC_V7_01: All four public test cases pass:
- strict_admission_policy_pin_survives_durable_recovery ... ok
- journaled_admission_policy_pin_survives_durable_recovery ... ok
- legacy_version_1_checkpoint_is_explicitly_rejected ... ok
- tampered_policy_pin_rejected_by_recover_retry ... ok

## Format Outcomes

PUBLIC_V7_02 (initial `cargo fmt --all --check`): Exit 1. Unrelated preexisting formatting drift in `crates/vb_runtime/src/run_checkpoint_tests.rs` and candidate formatting in the changed test file.

PUBLIC_V7_03 (scoped format): Ran `cargo fmt -- crates/vb_runtime/tests/policy_pin_durable_recovery.rs` only. Exit 0.

PUBLIC_V7_04 (re-verification): All four tests pass after formatting.

## Notes

- Unrelated preexisting formatting drift in `crates/vb_runtime/src/run_checkpoint_tests.rs` remains unformatted per instruction to format only changed candidate files.
- No production code changes. No VCS mutations. No bead tracker updates.