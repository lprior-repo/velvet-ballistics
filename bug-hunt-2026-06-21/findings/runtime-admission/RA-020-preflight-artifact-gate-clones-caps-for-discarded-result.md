# RA-020: `preflight_artifact_gate` clones the caller's `CapabilitySet` only to discard the admission result

- **Severity**: Info
- **Category**: perf / simplification
- **Location**: `crates/vb_runtime/src/runtime/admission/admission_check.rs:52-67`
- **Confidence**: confirmed

## Description

`preflight_artifact_gate` calls `admit_artifact_run(... caps.clone())` and then `.map(|_admission| ())` — the resulting `RunAdmission` is dropped. The clone of `caps` is therefore wasted: the admission function only needs `&CapabilitySet`, but the public signature of `admit_artifact_run` takes `CapabilitySet` by value, forcing the clone.

## Evidence

```rust
fn preflight_artifact_gate(
    shard: &Shard,
    run: RunId,
    digest: WorkflowDigest,
    caps: &CapabilitySet,
) -> RuntimeResult<()> {
    crate::admission::admit_artifact_run(
        shard.artifact_store.as_ref(),
        shard.policy,
        run,
        digest,
        caps.clone(),                              // <-- clone
    )
    .map(|_admission| ())                          // <-- result discarded
    .map_err(|error| super::admission_result::map_admission_error(error, digest))
}
```

`CapabilitySet` clone allocates (it owns a `Box<[Capability]>`). Each submit-through-the-preflight path therefore pays an allocation that is immediately dropped. The preflight is called on every submit, so this is a per-submit hot-path allocation.

Same pattern in `preflight_budget_gate` (lines 92-110) which also clones `caps` and discards the resulting `RunAdmission`.

## Adversarial Check

One could argue `admit_artifact_run` needs to OWN the caps because it constructs a `RunAdmission` that owns them — and that `RunAdmission` is the production contract. But the preflight gate explicitly does not need the `RunAdmission` (it discards it); the production run-state attachment happens later in the shard tick handler. The clone is purely a signature-induced artifact, not a semantic requirement. The per-submit hot-path justification is met: `submit_direct` is the public entrypoint and runs this clone on every call.

## Suggested Fix

Either (a) add a `&self`-taking variant of the admission function for preflight use (e.g., `admit_artifact_run_check(store, policy, run, digest, &caps) -> Result<(), AdmissionError>`) that does not construct a `RunAdmission`; or (b) change `admit_artifact_run` to take `&CapabilitySet` and have the `RunAdmission`-constructing callers clone at that point. Option (a) is the lower-impact change and matches the existing pattern of separating "check" from "commit."
