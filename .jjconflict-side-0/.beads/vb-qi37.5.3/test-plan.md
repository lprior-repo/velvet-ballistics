# Test Plan - vb-qi37.5.3

## Runtime Admission

- Reject accepted artifact when `idempotency_verified` is false.
- Reject accepted artifact when a keyed action is absent from `idempotency_attested`.
- Admit valid artifact and expose attested action IDs through `RunAdmission::idempotency_attested()`.

## Storage Admission

- Persist keyed/attested idempotency evidence derived from valid action contracts.
- Reject statically invalid idempotency contracts before accepted artifact persistence.
- Preserve `VerificationProof` serde roundtrip with the new status flag.

## Regression Gates

- `rtk cargo fmt --check`.
- `rtk cargo test -p vb_runtime -p vb_storage --lib admission::tests`.
- `rtk cargo clippy -p vb_runtime -p vb_storage --lib -- -D warnings -D clippy::unwrap_used -D clippy::panic -D clippy::expect_used`.
- `rtk cargo kani -p vb_compile --harness idempotency_gate_parity`.
