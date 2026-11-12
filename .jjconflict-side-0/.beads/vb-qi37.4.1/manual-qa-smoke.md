# Manual QA Smoke Report: vb-qi37.4.1

## Command
```
cargo nextest run -p vb_storage --test accepted_artifact_red_phase
```

## Execution Evidence
```
27 tests run: 10 passed, 17 failed, 0 skipped
exit code: non-zero (test run failed)
```

## Test Results

### Phase: Red Phase Contract Tests

| # | Test | Result | Exit |
|---|------|--------|------|
| 1 | `accepted_artifact_validator_rejects_warning_gate_sixteen` | PASS | 0 |
| 2 | `accepted_artifact_validator_accepts_warning_gate_fifteen` | PASS | 0 |
| 3 | `accepted_artifact_validator_uses_fifteen_gate_v1_upper_bound` | PASS | 0 |
| 4 | `accepted_artifact_encoder_records_fifteen_gate_proof_when_policy_is_strict` | PASS | 0 |
| 5 | `accepted_artifact_encoder_binds_ir_digest_to_ir_bytes_not_workflow_parts_digest` | PASS | 0 |
| 6 | `accepted_artifact_store_payload_is_nested_accepted_artifact_not_raw_workflow_parts` | PASS | 0 |
| 7 | `accepted_artifact_encoder_records_fifteen_gate_proof_when_policy_is_journaled` | PASS | 0 |
| 8 | `accepted_artifact_encoder_rejects_relaxed_raw_submit_when_accepted_artifacts_are_required` | PASS | 0 |
| 9 | `accepted_artifact_validator_requires_taint_safe_flag` | FAIL | 1 |
| 10 | `accepted_artifact_validator_requires_idempotency_attestation` | FAIL | 1 |
| 11 | `runtime_admission_returns_secret_unavailable_when_required_secret_is_absent` | FAIL | 1 |
| 12 | `accepted_artifact_validator_requires_replayable_flag` | FAIL | 1 |
| 13 | `runtime_admission_returns_frame_allocation_failed_when_frame_pool_is_exhausted` | FAIL | 1 |
| 14 | `accepted_artifact_validator_requires_bounded_flag` | FAIL | 1 |
| 15 | `runtime_admission_returns_active_run_capacity_exceeded_when_capacity_is_full` | FAIL | 1 |
| 16 | `runtime_admission_returns_admission_journal_failed_when_run_events_cannot_be_recorded` | FAIL | 1 |
| 17 | `accepted_artifact_validator_rejects_legacy_two_gate_proof` | FAIL | 1 |
| 18 | `runtime_admission_returns_run_already_exists_when_run_is_active_or_accepted` | FAIL | 1 |
| 19 | `runtime_admission_returns_strict_durability_failed_when_sync_all_fails` | FAIL | 1 |
| 20 | `accepted_artifact_validator_requires_retry_safe_flag` | FAIL | 1 |
| 21 | `runtime_admission_returns_clock_unavailable_when_clock_cannot_supply_timestamp` | FAIL | 1 |
| 22 | `runtime_admission_returns_input_schema_mismatch_when_input_fails_schema` | FAIL | 1 |
| 23 | `runtime_admission_returns_capability_denied_when_required_capability_is_missing` | FAIL | 1 |
| 24 | `runtime_admission_returns_artifact_invalid_when_store_validation_fails` | FAIL | 1 |
| 25 | `runtime_admission_returns_input_too_large_when_input_exceeds_bound` | FAIL | 1 |

### Failure Pattern Analysis

**10 passing / 17 failing**

#### Category A — Validator proof-flag tests (5 tests)
```
accepted_artifact_validator_requires_{bounded,taint_safe,retry_safe,replayable}_flag
accepted_artifact_validator_requires_idempotency_attestation
```
All five fail with:
```
assertion `left == right` failed
  left:  "VerificationProof { digest: ..., gate_count: 15, durable: true, warnings: [] }"
  right: "VerificationProofV1 { gate_count: 15, bounded: true }"  (or taint_safe/retry_safe/replayable/idempotency_attested)
```
**Root cause**: Tests assert `encode_artifact(...)` returns `Err(ArtifactEnvelopeError::MissingRequiredProofFlag { flag })` for a specific flag set to false, but the encoder returns `Ok(VerificationProof { durable: true, ... })` instead. The validation is not rejecting artifacts with individual proof flags set to false.

#### Category B — Validator legacy gate-count test (1 test)
```
accepted_artifact_validator_rejects_legacy_two_gate_proof
```
Fails with:
```
left:  "VerificationProof { ...gate_count: 15, durable: true... }"
right: "Err(ArtifactEnvelopeError::InvalidGateCount { found: 2 })"
```
**Root cause**: The encoder is not rejecting artifacts with `gate_count = 2`; it accepts and returns success instead of the expected `InvalidGateCount` error.

#### Category C — Admission error-path tests (11 tests)
```
runtime_admission_returns_{secret_unavailable,frame_allocation_failed,
active_run_capacity_exceeded,admission_journal_failed,run_already_exists,
strict_durability_failed,clock_unavailable,input_schema_mismatch,
capability_denied,artifact_invalid,input_too_large}_...
```
All fail with:
```
left:  "Ok(AcceptedArtifact { digest: ..., ir: [...], verification: ... })"
right: "Err(AdmissionError::{Variant})"
```
**Root cause**: Every admission test that expects an error return instead gets `Ok(AcceptedArtifact)`. The admission logic is not enforcing any of the error conditions (capacity exceeded, journal failure, clock unavailable, schema mismatch, capability denial, etc.).

### Root Cause Summary

The `vb_storage` implementation has not wired validation/admission error paths into the public API under test. The encoder returns `Ok` for all inputs regardless of proof-flag states and gate counts. The admission function (`submit_artifact` / `admit_run`) returns `Ok(AcceptedArtifact)` for all calls, never propagating the expected `AdmissionError` variants. This matches the implementation report: "I did not add test-name-dependent production behavior."

## Findings

### CRITICAL (block merge)

1. **Validation not enforcing proof flags** — `encode_artifact`/`validate_accepted_artifact_v1` does not check that `bounded`, `taint_safe`, `retry_safe`, `replayable`, and `idempotency_attested` fields in `VerificationProofV1` are all `true`. Tests expecting `MissingRequiredProofFlag` errors get success responses. Contract Section 6.1 item 6 and Section 7.2 item 4 are unimplemented.

2. **Validation not rejecting legacy gate counts** — `InvalidGateCount { found: 2 }` is never returned for 2-gate proofs (Contract Section 9.1 / Scenario 6). The encoder accepts `gate_count = 2` and stores it without error. Contract Section 6.1 item 4 and Section 9.1 item `InvalidGateCount` are unimplemented.

3. **Admission never returns error variants** — All 11 admission error-path tests expect `Err(AdmissionError::...)` but receive `Ok(AcceptedArtifact)`. The admission boundary does not check: run capacity, frame pool, journal append, clock availability, strict sync, duplicate runs, input size/schema, artifact validity, capabilities, or secrets. Contract Sections 6.3, 7.3, and 9.2 are unimplemented.

4. **No panics detected** — Tests compile and run without panic; the failures are assertion mismatches, not runtime panics. This is the only non-critical observation.

## Artifact Path
```
/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/.beads/vb-qi37.4.1/manual-qa-smoke.md
```

## VERDICT: FAIL

STATUS: FAIL
