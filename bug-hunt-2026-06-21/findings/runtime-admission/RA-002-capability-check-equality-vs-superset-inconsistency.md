# RA-002: Capability check uses exact-count equality in `admit_artifact_run` but superset in `validate_artifact_capabilities`

- **Severity**: Medium
- **Category**: correctness (API inconsistency)
- **Location**: `crates/vb_runtime/src/admission/admission.rs:135-143` versus `crates/vb_runtime/src/runtime/admission/admission_check.rs:398-409`
- **Confidence**: confirmed

## Description

The crate-level admission function `admit_artifact_run` rejects any caller whose `CapabilitySet` length is not exactly equal to the artifact's `required_capabilities` length, while the runtime-facade helper `validate_artifact_capabilities` performs a pure superset check. The same `submit_artifact` call runs both gates in sequence, so a caller that grants extra capabilities passes the first gate and fails the second with a misleading "capability denied" error.

## Evidence

`admission/admission.rs:135-143` (equality + per-cap iteration):

```rust
if caps.len() != artifact.required_capabilities.len() {
    return Err(capability_count_mismatch_error(
        &artifact.required_capabilities,
        &caps,
    ));
}
for required_cap in artifact.required_capabilities.iter() {
    check_capability(required_cap.action_id(), required_cap, &caps)?;
}
```

`runtime/admission/admission_check.rs:398-409` (superset only):

```rust
fn validate_artifact_capabilities(
    artifact: &vb_storage::admission::AcceptedArtifact,
    caps: &CapabilitySet,
    artifact_digest: WorkflowDigest,
) -> RuntimeResult<()> {
    for required in artifact.required_capabilities.iter() {
        crate::admission::check_capability(required.action_id(), required, caps).map_err(
            |error| super::admission_result::map_admission_error(error, artifact_digest),
        )?;
    }
    Ok(())
}
```

`submit_artifact` (`admission_check.rs:209-224`) invokes `validate_artifact_capabilities` (superset) and then `enqueue_decoded_artifact` → `preflight_direct_admission` → `preflight_artifact_gate` → `admit_artifact_run` (equality). A caller passing legitimate extra capabilities is accepted by the first gate and rejected by the second.

The error returned by `capability_count_mismatch_error` (`admission/guards.rs:29-40`) is `CapabilityDenied` carrying a synthetic `Capability::new("__capability_count_mismatch__", ActionId::new(0))`, which further obscures the real reason.

## Adversarial Check

One could argue the equality check is a deliberate least-privilege enforcement: callers must grant exactly the required set, no more. But the runtime facade's own `validate_artifact_capabilities` does NOT enforce least-privilege — it accepts supersets. If least-privilege were the contract, both gates would enforce it. The asymmetric enforcement means the contract is unspecified and the second gate's failure mode is misleading. Either both should be equality (and document least-privilege) or both should be superset (and remove the equality check).

## Suggested Fix

Pick one semantics. If superset is intended (the more conventional capability model), drop the `caps.len() != ...` check in `admit_artifact_run` and rely on the per-capability `check_capability` loop. If least-privilege is intended, add the same equality check to `validate_artifact_capabilities`. Either way, replace the synthetic `__capability_count_mismatch__` capability in `capability_count_mismatch_error` with a dedicated `AdmissionError::CapabilityCountMismatch { required, granted }` variant.
