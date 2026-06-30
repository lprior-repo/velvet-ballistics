# Assurance Bundle - vb-qi37.5.3

## Claim

Runtime admission now carries and enforces idempotency evidence from accepted artifacts.

## Evidence Map

- R1: `VerificationProof::idempotency_verified` added in `crates/vb_storage/src/admission.rs`.
- R2: `admit_artifact_run_rejects_missing_idempotency_gate` proves failed idempotency status rejects.
- R3: `admit_artifact_run_rejects_keyed_action_without_attestation` proves missing keyed attestation rejects.
- R4: `admit_artifact_run_carries_idempotency_evidence_to_dispatch` proves `RunAdmission` exposes attested action IDs.
- R5: `submit_artifact_carries_idempotency_evidence_from_contracts` and `submit_artifact_rejects_failed_idempotency_contract` prove storage behavior.

## Raw Gates

- `moon ci`: PASS, 20 tasks completed.
- `rtk cargo test -p vb_runtime -p vb_storage --lib admission::tests`: PASS, 49 passed.
- `rtk cargo clippy -p vb_runtime -p vb_storage --lib ...`: PASS.
- `rtk cargo kani -p vb_compile --harness idempotency_gate_parity`: PASS, `VERIFICATION:- SUCCESSFUL`.

## Non-Blocking Classification

- All-target clippy over tests fails on pre-existing test lint debt, documented as `DEFERRED_GLOBAL` in `verification-ledger.jsonl`.
