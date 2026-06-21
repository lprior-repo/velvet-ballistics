# RA-001: `map_admission_error` collapses 10+ distinct admission failures into `AdmissionArtifactInvalid`

- **Severity**: High
- **Category**: correctness (error swallowing)
- **Location**: `crates/vb_runtime/src/runtime/admission/admission_result.rs:105-129`
- **Confidence**: confirmed

## Description

`map_admission_error` only translates three admission variants (`ArtifactNotFound`, `CapabilityDenied`, `BudgetExceeded`) into typed `RuntimeError`s. Every other variant — including all aggregate-budget exhaustion and overflow paths — is funneled into the `_ =>` catch-all arm and surfaced as `RuntimeError::AdmissionArtifactInvalid { digest }`, destroying the original error class.

## Evidence

```rust
pub(super) fn map_admission_error(
    error: crate::admission::AdmissionError,
    workflow_digest: WorkflowDigest,
) -> crate::RuntimeError {
    match error {
        crate::admission::AdmissionError::ArtifactNotFound { digest } => ...
        crate::admission::AdmissionError::CapabilityDenied { .. } => ...
        crate::admission::AdmissionError::BudgetExceeded { actual, limit } => ...
        _ => crate::RuntimeError::AdmissionArtifactInvalid { digest: workflow_digest },
    }
}
```

The variants silently collapsed to "artifact invalid":

- `ResourceCapacityExceeded { resource, requested, available }` — runtime is out of capacity.
- `BudgetPolicyExceeded { resource, actual, limit }` — request exceeds policy ceiling.
- `ResourceBudgetOverflow { resource }` / `ResourceBudgetUnderflow { resource }`.
- `ResourceBudgetInvalidCapacity { resource }`.
- `ResourceStepCeilingExceeded { requested, limit }` / `ResourcePerTickCeilingExceeded { requested, limit }`.
- `ArtifactEnvelopeDecodeFailed`, `ArtifactInvalidGateCount`, `ArtifactInvalidProofFlag`.
- `ArtifactDigestMismatch { requested, found }`.

This is the only error path surfaced by the three production preflight gates (`preflight_artifact_gate`, `preflight_budget_gate`) and by `submit_artifact` (via `validate_artifact_capabilities` and `validate_artifact_envelope` at `admission_check.rs:343,405`). An operator scraping `RuntimeError` therefore cannot distinguish a corrupted artifact from a transient resource exhaustion event with the same digest.

## Adversarial Check

One could argue this is intentional because `RuntimeError` lacks dedicated variants for `ResourceCapacityExceeded`, `BudgetPolicyExceeded`, etc. But `RuntimeError::AdmissionBudgetExceeded` already exists (`error/types.rs:235`) and is used for the step-count failure — the same shape (`{actual, limit}`) fits `BudgetPolicyExceeded`, `ResourceStepCeilingExceeded`, and `ResourcePerTickCeilingExceeded`. Mapping capacity-exhaustion to "artifact invalid" is actively misleading: the artifact is valid, the shard is full. At minimum the typed `{actual, limit, resource}` triple should be preserved or wrapped in `RuntimeError::Core { source }`.

## Suggested Fix

Add per-variant arms that preserve the typed information (or extend `RuntimeError` with `AdmissionResourceCapacityExceeded`, `AdmissionBudgetPolicyExceeded`, etc.), replacing the `_ =>` fallback with an exhaustive match. If new variants must remain open via `#[non_exhaustive]`, the fallback should still distinguish "budget-class" from "artifact-class" failures instead of unconditionally returning `AdmissionArtifactInvalid`.
