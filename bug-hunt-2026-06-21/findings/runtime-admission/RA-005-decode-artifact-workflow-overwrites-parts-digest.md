# RA-005: `decode_artifact_workflow` silently overwrites the deserialized workflow digest with the artifact digest

- **Severity**: Medium
- **Category**: correctness (integrity / defense-in-depth)
- **Location**: `crates/vb_runtime/src/runtime/admission/admission_check.rs:361-380`
- **Confidence**: confirmed

## Description

When decoding the `WorkflowParts` from the artifact IR, `decode_artifact_workflow` unconditionally executes `parts.digest = artifact.digest` after deserialization and before `CompiledWorkflow::try_from_parts`. This destroys any discrepancy between the workflow's internally-recorded digest and the artifact's digest instead of treating the mismatch as a hard integrity failure.

## Evidence

```rust
fn decode_artifact_workflow(
    artifact: &vb_storage::admission::AcceptedArtifact,
    artifact_digest: WorkflowDigest,
) -> RuntimeResult<CompiledWorkflow> {
    let (mut parts, remaining) = postcard::take_from_bytes::<vb_core::workflow::WorkflowParts>(
        &artifact.ir,
    )
    .map_err(|_| RuntimeError::AdmissionArtifactInvalid {
        digest: artifact_digest,
    })?;
    if !remaining.is_empty() {
        return Err(RuntimeError::AdmissionArtifactInvalid {
            digest: artifact_digest,
        });
    }
    parts.digest = artifact.digest;
    CompiledWorkflow::try_from_parts(parts).map_err(|_| RuntimeError::AdmissionArtifactInvalid {
        digest: artifact_digest,
    })
}
```

`CompiledWorkflow::try_from_parts` (`vb_core/src/workflow/mod.rs:46-62`) does not re-validate `parts.digest` against the serialized content; it is taken as authoritative. So the resulting `CompiledWorkflow::digest()` is `artifact.digest` no matter what `parts.digest` was at deserialization time.

The earlier `validate_artifact_ir_digest` (`admission_check.rs:346-359`) only checks `blake3(&artifact.ir) == artifact.digest`. It does not check `parts.digest == blake3(&artifact.ir)`. The override at line 376 therefore silently masks the case where the artifact's serialized `WorkflowParts.digest` field disagrees with the bytes it was deserialized from.

## Adversarial Check

One could argue `validate_artifact_ir_digest` already transitively guarantees `parts.digest == artifact.digest` for any honestly-compiled workflow because the compiler sets `parts.digest = blake3(serialized_parts)`. But the entire point of the typed `AdmissionArtifactDigestMismatch` / `AdmissionDigestMismatch` error variants (`error/types.rs:95,116`) is to reject *crafted* artifacts whose internal fields disagree. A crafted artifact with `parts.digest = X` but bytes that hash to `Y` would: (1) pass `validate_artifact_ir_digest` (bytes hash to Y, artifact.digest = Y), (2) have its `parts.digest = X` silently replaced by Y, and (3) produce a `CompiledWorkflow` whose `digest()` is Y while any cached state keyed on the original X digest is now incorrect. The override is a defense-in-depth regression even if no current production code path exploits it.

## Suggested Fix

Either (a) replace the silent override with an explicit check:

```rust
if parts.digest != artifact.digest {
    return Err(RuntimeError::AdmissionArtifactDigestMismatch {
        requested: artifact.digest,
        found: parts.digest,
    });
}
```

or (b) document why the override is safe and add the check as a debug-only assertion. Variant (a) is strongly preferred given the existing `AdmissionArtifactDigestMismatch` error variant is purpose-built for this case.
