# RA-023: `admit_artifact_run_with_certificate_floor` short-circuits on count mismatch before checking the per-capability grants

- **Severity**: Low
- **Category**: correctness (admission ordering)
- **Location**: `crates/vb_runtime/src/admission/admission.rs:134-143`
- **Confidence**: confirmed

## Description

The capability admission gate in `admit_artifact_run_with_certificate_floor` checks `caps.len() != artifact.required_capabilities.len()` *before* iterating per-capability grants. The short-circuit returns a misleading "CapabilityDenied" error (see RA-018) that names the FIRST required capability — which may in fact be properly granted — instead of the actually-missing one. The iteration order also bypasses the more-useful per-capability diagnostic that would fire if the count check were absent.

## Evidence

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

For a caller that grants `{cap_a, cap_b, cap_c}` against an artifact requiring `{cap_a, cap_b, cap_d}`, the count check passes (3 == 3) and the loop correctly reports `cap_d` as denied. For a caller that grants `{cap_a, cap_b, cap_c, cap_e}` (a superset), the count check fires first and reports `cap_a` (the first required) as "denied" — but `cap_a` IS granted. The caller sees a fabricated denial of a granted capability and never learns about the per-capability miss or extra-grant situation.

## Adversarial Check

One could argue that since the contract is "exact set" (per RA-002), the count check is correct and the per-capability loop is the fallback for the equal-count-but-different-set case. But the error message returned to operators (via `RuntimeError::AdmissionCapabilityDenied { action: required_capability.action_id(), required: required_capability, ... }`) names the FIRST required capability, which is not necessarily the denied one — the actual denial is "set equality violated." The per-capability loop, if run, would surface the specific missing/extra cap. The current ordering optimizes for the count-mismatch fast path at the cost of useful diagnostics.

## Suggested Fix

Either (a) drop the count check (per RA-002's superset semantics) and rely on the per-capability loop, or (b) keep the count check but return a typed `CapabilityCountMismatch` error (per RA-018) instead of fabricating a `CapabilityDenied` on the first required cap.
