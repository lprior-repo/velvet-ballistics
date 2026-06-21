# RA-018: `capability_count_mismatch_error` fabricates a synthetic `Capability` and mislabels count mismatches as `CapabilityDenied`

- **Severity**: Low
- **Category**: correctness (error fidelity)
- **Location**: `crates/vb_runtime/src/admission/guards.rs:29-40` (used by `admit_artifact_run` at `admission/admission.rs:135-140`)
- **Confidence**: confirmed

## Description

When `caps.len() != artifact.required_capabilities.len()`, the helper constructs a synthetic `Capability::new("__capability_count_mismatch__".into(), ActionId::new(0))` and returns it as the `required` field of a `CapabilityDenied` error. The fabricated capability is unrelated to any real required capability, the action ID `0` is a placeholder that has no relation to the artifact, and the count-mismatch classification is lost — the caller sees the same variant as a genuine "this specific capability was not granted."

## Evidence

```rust
pub(crate) fn capability_count_mismatch_error(
    required: &[Capability],
    granted: &CapabilitySet,
) -> AdmissionError {
    let fallback = Capability::new("__capability_count_mismatch__".into(), ActionId::new(0));
    let required_capability = required.first().cloned().unwrap_or(fallback);
    AdmissionError::CapabilityDenied {
        action: required_capability.action_id(),
        required: required_capability,
        granted: granted.clone(),
    }
}
```

Through `map_admission_error` (admission_result.rs:113-121), this surfaces to the operator as `RuntimeError::AdmissionCapabilityDenied { action: ActionId(0), required: Capability("__capability_count_mismatch__"), granted: ... }`. The error message becomes "admission rejected: capability denied for action ActionId(0)" — actively wrong if the artifact actually requires capabilities on actions 5, 12, and 47.

If `required` happens to be empty (zero required, non-zero granted), the synthetic fallback is used; if non-empty, the *first* required capability is reported as "denied" even though it may in fact be granted (the failure was a count mismatch, not a per-capability denial).

## Adversarial Check

One could argue the helper is internal-only and the message fidelity does not matter. But `map_admission_error` propagates the `CapabilityDenied` payload verbatim into `RuntimeError::AdmissionCapabilityDenied` (admission_result.rs:113-121), which is observable to operators. A debugging operator seeing this error would look for a missing grant on `ActionId(0)` and find nothing relevant — the actual problem (count mismatch) is hidden behind fabricated data. Also, the rule "no fabricated constants in error payloads" is a standard holzman-rust principle: errors should be honest evidence.

## Suggested Fix

Add a dedicated `AdmissionError::CapabilityCountMismatch { required_count: usize, granted_count: usize }` variant, mapped to a new `RuntimeError::AdmissionCapabilityCountMismatch`. Delete `capability_count_mismatch_error` and update `admit_artifact_run` to return the typed count-mismatch error directly.
