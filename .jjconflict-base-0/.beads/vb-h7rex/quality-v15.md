# vb-h7rex Quality Evidence V15

Fresh immutable gate runs. All pass.

| Gate | Command | Exit | Seconds |
|------|---------|------|---------|
| QUALITY_V15_01 | `rustup run nightly-2026-04-28 cargo test -p vb_runtime --test policy_pin_durable_recovery` | 0 | 0.888 |
| QUALITY_V15_02 | `rustup run nightly-2026-04-28 cargo test -p vb_runtime --test policy_pin_durable_recovery -- --exact admission_policy_pin_mismatch_rejected_by_recover_retry` | 0 | 0.070 |
| QUALITY_V15_03 | `rustup run nightly-2026-04-28 cargo fmt --all -- --check` | 0 | 1.485 |

## Test counts (QUALITY_V15_01)
- Tests: 5
- Passed: 5
- Failed: 0
- Ignored: 0
- Measured: 0

## Source changes
- `recovery/full/admission.rs`: Made `validate_recovered_policy_pin` pub(crate)
- `recovery/full.rs`: Added `validate_recovered_policy_pin` to pub(crate) re-exports
- `recovery.rs`: Added `validate_recovered_policy_pin` to pub(crate) re-exports
- `shard/lifecycle/chunk_001.rs`: Replaced 20-line duplicated policy pin validation with call to shared `validate_recovered_policy_pin`

## Test fix (admission_policy_pin_mismatch_rejected_by_recover_retry)
- Added positive control: DTO helper with original pin produces admission matching original
- Added journal close/reopen after forging; verify persisted checkpoint equals forged
- Replaced event_count assert with full-vec equality and original prefix integrity check
- Fixed stale comment "We use the original pin as the forged pin value" (was false)