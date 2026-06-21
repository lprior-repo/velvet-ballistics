# RA-028: `admit_run` (no capability check, no certificate freshness) is publicly exported and silently weaker than `admit_artifact_run`

- **Severity**: Low
- **Category**: correctness (API contract)
- **Location**: `crates/vb_runtime/src/admission/admission.rs:26-54`
- **Confidence**: confirmed

## Description

`admit_run` is re-exported at `admission.rs:18-22` as a public symbol. Compared to `admit_artifact_run` (lines 70-161), it omits: capability coverage checks, verification digest binding, certificate freshness check, and the typed `ArtifactInvalidProofFlag { flag: "runtime_policy" }` for unknown policies. A caller that uses `admit_run` instead of `admit_artifact_run` gets silently weaker admission.

## Evidence

`admit_run` (lines 26-54):

```rust
pub fn admit_run(
    store: &dyn AcceptedArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
    run_id: RunId,
    caps: CapabilitySet,
) -> Result<RunAdmission, AdmissionError> {
    match policy {
        RuntimePolicy::Strict | RuntimePolicy::Journaled => {
            let artifact = store.load_accepted_artifact(digest).map_err(...)?;
            validate_accepted_artifact_envelope(&artifact).map_err(...)?;
            if artifact.digest != digest {
                return Err(AdmissionError::ArtifactDigestMismatch { ... });
            }
        }
        RuntimePolicy::Relaxed => {}
        _ => return Err(AdmissionError::ArtifactInvalidProofFlag { flag: "runtime_policy" }),
    }
    Ok(RunAdmission::new(digest, run_id, caps, policy))
}
```

The `caps` parameter is taken but never inspected — no `check_capability` call, no count check. The result is `RunAdmission::new` (no idempotency evidence, no budget).

`admit_artifact_run` (lines 70-161) on the same inputs:
- Validates the envelope.
- Checks `artifact.verification.digest != artifact.digest` (line 119-124).
- Checks `artifact.accepted_at_seq < required_at_least` for certificate freshness (line 126-132).
- Checks `caps.len() != artifact.required_capabilities.len()` (line 135-140).
- Iterates per-capability grants (line 141-143).
- Returns `RunAdmission::with_idempotency_evidence` (line 145-151).

## Adversarial Check

One could argue `admit_run` is a legacy entrypoint kept for backward compatibility. But it is re-exported alongside `admit_artifact_run` with no deprecation notice, no docstring warning, and no feature flag. A new embedder reading `admission.rs:18-22` sees two equally-prominent admission functions and may pick the simpler one. The fact that production code never calls `admit_run` (only `admission/tests.rs` and `kani_capability_harnesses.rs` do) is the strongest signal that it is dead — but the public export makes the weakness part of the API contract.

## Suggested Fix

Either (a) delete `admit_run` and update tests to use `admit_artifact_run` (the test-only callers can be moved to a test helper module); or (b) mark `admit_run` `#[deprecated]` and update the docstring to direct callers to `admit_artifact_run`; or (c) move `admit_run` behind `#[cfg(any(test, feature = "test-util"))]`. Option (a) is the cleanest.
