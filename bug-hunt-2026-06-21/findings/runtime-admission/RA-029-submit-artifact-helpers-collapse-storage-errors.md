# RA-029: `submit_artifact` helpers collapse every storage-layer error into `AdmissionArtifactInvalid`

- **Severity**: Medium
- **Category**: correctness (error swallowing)
- **Location**: `crates/vb_runtime/src/runtime/admission/admission_check.rs:293-313` and `crates/vb_runtime/src/runtime/admission/admission_check.rs:361-396`
- **Confidence**: confirmed

## Description

The internal helpers backing `Runtime::submit_artifact` (`validate_compiled_ir_record`, `decode_compiled_ir_artifact`, `decode_artifact_workflow`, `capability_set_from_slice`) all use `.map_err(|_| RuntimeError::AdmissionArtifactInvalid { digest })`. The discarded errors span storage-layer validation failures, postcard decode failures, `try_from_parts` failures, and even allocation failure from `try_reserve_exact`. An operator sees "artifact invalid" for any of these, with no way to distinguish a corrupted journal record from an OOM.

## Evidence

`admission_check.rs:293-302`:

```rust
fn validate_compiled_ir_record(
    record: &vb_storage::CompiledIrRecord,
    artifact_digest: WorkflowDigest,
) -> RuntimeResult<()> {
    vb_storage::admission::validate_compiled_ir_record(record).map_err(|_| {
        RuntimeError::AdmissionArtifactInvalid {
            digest: artifact_digest,
        }
    })
}
```

`admission_check.rs:304-313`:

```rust
fn decode_compiled_ir_artifact(
    record: &vb_storage::CompiledIrRecord,
    artifact_digest: WorkflowDigest,
) -> RuntimeResult<vb_storage::admission::AcceptedArtifact> {
    vb_storage::admission::decode_accepted_artifact_envelope(&record.ir).map_err(|_| {
        RuntimeError::AdmissionArtifactInvalid {
            digest: artifact_digest,
        }
    })
}
```

`admission_check.rs:382-396`:

```rust
fn capability_set_from_slice(
    capabilities: &[Capability],
    artifact_digest: WorkflowDigest,
) -> RuntimeResult<CapabilitySet> {
    let mut grants = Vec::new();
    grants.try_reserve_exact(capabilities.len()).map_err(|_| {
        RuntimeError::AdmissionArtifactInvalid {     // <-- allocation failure labeled artifact-invalid
            digest: artifact_digest,
        }
    })?;
    ...
}
```

The `try_reserve_exact` failure path is particularly misleading: an allocator exhaustion event becomes "the artifact is invalid," which would send an operator chasing a non-existent artifact bug instead of investigating memory pressure.

## Adversarial Check

One could argue that from the *caller's* perspective, all of these failures mean "we could not load and validate the artifact, treat it as invalid." But the `RuntimeError::AdmissionArtifactInvalid { digest }` variant has no inner source field — unlike `RuntimeError::StorageJournalAppend { source }` or `RuntimeError::Core { source }` — so the underlying error is permanently lost. Even a `tracing::error!` would preserve it in logs, but the helpers do not log. The `AdmissionArtifactInvalid` docstring (`error/types.rs:89-93`) literally says "the accepted artifact envelope is invalid" — that contract is broken when the helper returns it for an allocator failure.

## Suggested Fix

Either (a) extend `RuntimeError::AdmissionArtifactInvalid` to carry an optional `source: Box<dyn Error>` and pass the underlying error through; or (b) introduce a `RuntimeError::AdmissionArtifactDecodeFailed { digest, stage: &'static str }` variant that names which decode step failed (e.g., "compiled_ir_record", "envelope", "workflow_parts"); or (c) at minimum, log the discarded error at WARN before mapping. The allocation-failure path should return a dedicated `RuntimeError::AllocationFailed` variant rather than re-using `AdmissionArtifactInvalid`.
