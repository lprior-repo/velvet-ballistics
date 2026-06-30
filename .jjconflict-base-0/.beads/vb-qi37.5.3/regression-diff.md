# Regression Diff - vb-qi37.5.3

## Changed Source

- `crates/vb_storage/src/admission.rs`: adds idempotency proof status, evidence derivation from contracts, and invalid idempotency contract rejection.
- `crates/vb_runtime/src/admission.rs`: validates idempotency proof status and keyed/attested consistency; carries attested IDs into `RunAdmission`.

## Behavioral Delta

- Strict/Journaled admission now fails closed for missing/failed idempotency proof evidence.
- Strict/Journaled admission now fails closed when key-required actions are not attested.
- Runtime dispatch can inspect the admitted idempotency attestation set.

## Compatibility

- Relaxed runtime admission remains non-validating.
- Existing proof constructors default `idempotency_verified` to true.
