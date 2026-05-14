# Formal Verification Report

STATUS: APPROVED

## Inputs
- proof-obligations.jsonl: 8 obligations
- contract-verification-review.md: STATUS: APPROVED
- tla-report.md: STATUS: APPROVED

## Tool Availability
- tlc / TLC: TLC2 Version 2.19 of 08 August 2024
- moon: verify-proof task available and executed
- cargo kani: available; no proof harnesses found by exact `moon run :verify-proof`
- verus: not installed; `WAIVER-VERUS-DIAG-TOTALITY` remains approved

## Obligation Results

| id | result | command | evidence |
|---|---|---|---|
| ERR-ADM-CAUSES-001 | PASS | `cargo test -p vb_runtime admission_durability_error_variants_are_exhaustive` | 1 passed, 1324 filtered out |
| ERR-ADM-PERSIST-001 | PASS | `cargo test -p vb_runtime admission_header_persistence_failure_has_dedicated_diagnostic` | 1 passed, 1324 filtered out |
| ERR-ADM-001 | PASS | `cargo test -p vb_runtime admission_durability_errors_have_stable_codes` | 2 passed, 1323 filtered out |
| API-ADM-001 | PASS | `cargo test -p velvet_ballastics --test admission_evidence_integration api_envelope_preserves_admission_durability_code` | 1 passed, 6 filtered out |
| TLA-ERR-001 | PASS | `moon run :verify-proof`; `tlc -config specs/admission_header_before_ack.cfg specs/admission_header_before_ack.tla` | moon PASS; TLC PASS, 8 generated, 4 distinct, diameter 2 |
| ERR-IDEMP-001 | PASS | `cargo test -p vb_runtime duplicate_run_id_preserves_stable_diagnostic_code` | 1 passed, 1324 filtered out |
| REL-GATE-004 | DEFERRED_GLOBAL | `moon ci` | unchanged unrelated global debt per prior regression-diff.md |
| WAIVER-VERUS-DIAG-TOTALITY | WAIVED | `review waiver metadata in verification-layers.md` | waiver approved with compensating evidence |

## TLA Evidence
- module file: `specs/admission_header_before_ack.tla`
- config file: `specs/admission_header_before_ack.cfg`
- bounded constants: `ErrorCodes = {HeaderPersistenceFailed, QueueFull}`, `NoCode = NoCode`
- invariants checked: `TypeOK`, `FailurePreventsAck`
- temporal property checked: `FailureEventuallyRejected`
- fairness: weak fairness on `AdmissionReject` and `StorageFail`
- deadlock: checked with explicit terminal stutter
- TLC result: PASS; 8 states generated, 4 distinct states, diameter 2, no errors

## Ledger Counts
- PASS: 6
- WAIVED: 1
- DEFERRED_GLOBAL: 1
- FAIL_LOCAL: 0
- BLOCKING required failures: 0

## Decision
All bead-local required blockers are PASS. The remaining `REL-GATE-004` item is classified as `DEFERRED_GLOBAL` and non-blocking for this bead. `STATUS: APPROVED` is exact because required obligations are PASS/WAIVED/DEFERRED_GLOBAL non-blocking only.
