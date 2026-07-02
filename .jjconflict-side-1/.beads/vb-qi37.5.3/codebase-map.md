# Codebase Map - vb-qi37.5.3

## Scope

- Bead: `vb-qi37.5.3` runtime: carry idempotency evidence into admission.
- Source checkout: `/home/lewis/src/velvet-ballistics`.
- Isolated workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5-3`.
- Parent repaired to dependency bookmark `go-skill-p0-vb-qi37-5` before implementation.

## Relevant Files

- `crates/vb_storage/src/admission.rs`: `VerificationProof`, `AcceptedArtifact`, `submit_artifact_with_contracts`.
- `crates/vb_runtime/src/admission.rs`: `ArtifactEnvelopeError`, `RunAdmission`, `AcceptedArtifactStore`, `admit_artifact_run`.
- `crates/vb_compile/src/kani_idempotency_parity.rs`: existing all-45 compile/validate idempotency parity harness.
- `crates/vb_validate/src/idempotency_contract.rs`: canonical idempotency decision table.

## Risk Tags

- `runtime-admission`: strict/journaled admission must reject artifacts missing required proof metadata.
- `serde-schema`: `VerificationProof` gains an explicit idempotency proof flag.
- `dispatch-inspection`: `RunAdmission` must expose attested action IDs after admission.
- `proof-parity`: storage-side local decision table must stay consistent with the validated all-45 model.

## Out Of Scope

- Main merge/landing is intentionally out of scope for this request.
- Fuzz target admission_fuzz remains a known tooling waiver inherited from related idempotency work.
