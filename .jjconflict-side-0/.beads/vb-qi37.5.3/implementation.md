# Implementation - vb-qi37.5.3

## Files Changed

- `crates/vb_storage/src/admission.rs`.
- `crates/vb_runtime/src/admission.rs`.

## Changes

- Added `VerificationProof::idempotency_verified` and defaulted it true for accepted proof construction.
- Derived `idempotency_keyed` and `idempotency_attested` from action contracts during artifact submission.
- Rejected statically invalid idempotency contracts before persisting accepted artifacts.
- Added runtime envelope validation for `idempotency_verified` and keyed/attested consistency.
- Added `RunAdmission::with_idempotency_evidence` and `RunAdmission::idempotency_attested()` for runtime dispatch inspection.

## Requirement Mapping

- R1: `VerificationProof::idempotency_verified`.
- R2/R3: `validate_accepted_artifact_envelope`.
- R4: `RunAdmission::idempotency_attested`.
- R5: `idempotency_evidence_from_contracts` and `is_contract_idempotency_accepted`.
