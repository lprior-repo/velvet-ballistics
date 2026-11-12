# TEST Review: V21 (vb-0kqmx)
**Reviewer:** local5090 (independent; author was a3090)
**Beard:** vb-0kqmx
**Scope:** Two fixture expectation corrections in `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs`
**Status:** APPROVED

---

## Scope Summary

This review covers V21, which corrects two failing admission tests after the V20 historical wire + read-only reviews (V19: code; V20: wire layout). Both failures originated from the same root cause: the old expected `ObservedAdmissionDiagnostic::Admitted(RunAdmission::new(...))` comparison used the compatibility constructor which sets `policy_pin` to `RunPolicyPin::Unverified`, but the actual returned `RunAdmission` from `admit_artifact_run` now carries `RunPolicyPin::Pinned(artifact.policy_digest)`.

The actual canonical V20 run showed: 14454 pass, 2 fail, 27 skips (full no-fail-fast suite). Both failures were in this file. V21 corrected the fixture expectations; the actual ROOT_V21_FIXTURE run passed with 22 passed, 0 failed, 0 ignored, 0 filtered (exit 0, 0.854s with -Dwarnings).

---

## Root Cause Analysis

### Production flow (verified from source)

`crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:163-175` constructs the returned `RunAdmission` via:

```rust
Ok(RunAdmission::with_idempotency_evidence(
    admitted_digest, run_id, caps, policy,
    artifact.verification.idempotency_attested,
).with_policy_pin(RunPolicyPin::Pinned(artifact.policy_digest)))
```

The `.with_policy_pin(RunPolicyPin::Pinned(artifact.policy_digest))` call explicitly binds the actual policy identity from the accepted artifact. This is the intended production behavior for Strict/Journaled admission.

### Old test failure

The old fixture used `RunAdmission::new(requested, run_id, granted, RuntimePolicy::Journaled)` as the expected object. `RunAdmission::new` sets `policy_pin: RunPolicyPin::Unverified` (compatibility constructor). Comparing a returned object with `Pinned(expected_policy_digest)` against an expected object with `Unverified` failed, even though the admission itself was correct and the policy pin was properly bound.

---

## Changes Verified

### Change 1: `given_valid_accepted_artifact_when_admitted_then_admission_record_contains_digest_certificate_profile` (lines ~817-850)

**Before (V20):**
```rust
let requested = artifact.digest;
// ...
assert_eq!(
    observed(result),
    ObservedAdmissionDiagnostic::Admitted(RunAdmission::new(
        requested, run_id, granted, RuntimePolicy::Journaled,
    ))
);
```

**After (V21):**
```rust
let requested = artifact.digest;
let expected_policy_digest = artifact.policy_digest;  // NEW: capture BEFORE move
// ...
let admission = match result {
    Ok(a) => a,
    Err(e) => return Err(format!("expected admission, got error: {e:?}")),
};
assert_eq!(admission.artifact_digest(), requested);
assert_eq!(admission.run_id(), run_id);
assert_eq!(admission.granted_capabilities(), &granted);
assert_eq!(admission.policy(), RuntimePolicy::Journaled);
assert_eq!(
    admission.policy_pin(),
    RunPolicyPin::Pinned(expected_policy_digest)  // NEW: explicit pin check
);
assert_eq!(admission.budget(), None);
assert_eq!(admission.idempotency_attested(), &[]);
```

**Assessment:**

- `expected_policy_digest` is captured from `artifact.policy_digest` before the artifact is consumed (moved into `ScenarioStore`).
- Individual field assertions replace the monolithic `==` comparison.
- All 7 fields of `RunAdmission` are now verified: `artifact_digest`, `run_id`, `granted_capabilities`, `policy`, `policy_pin`, `budget`, `idempotency_attested`.
- The `policy_pin` assertion confirms the actual production behavior: Strict/Journaled admission binds the artifact's real policy identity.

### Change 2: `proptest_capability_profiles_admit_iff_required_capability_is_exactly_granted` (lines ~1315-1352)

**Before (V20):**
```rust
let result = observed(admit_artifact_run(...));
if mutation == 0 {
    prop_assert_eq!(result, ObservedAdmissionDiagnostic::Admitted(
        RunAdmission::new(artifact_digest, RunId::new(501), granted, RuntimePolicy::Strict,)));
}
```

**After (V21):**
```rust
let expected_policy_digest = artifact.policy_digest;  // NEW
// ...
let result = admit_artifact_run(...);  // NO observed() wrapper for success path
if mutation == 0 {
    let admission = result.map_err(|error| TestCaseError::fail(...))?;
    prop_assert_eq!(admission.artifact_digest(), artifact_digest);
    prop_assert_eq!(admission.run_id(), RunId::new(501));
    prop_assert_eq!(admission.granted_capabilities(), &granted);
    prop_assert_eq!(admission.policy(), RuntimePolicy::Strict);
    prop_assert_eq!(admission.policy_pin(), RunPolicyPin::Pinned(expected_policy_digest));  // NEW
    prop_assert_eq!(admission.budget(), None);
    prop_assert_eq!(admission.idempotency_attested(), &[]);
} else if mutation == 2 || mutation == 5 {
    // CapabilityCountMismatch path unchanged (observed() still used)
} else {
    // CapabilityDenied path unchanged (observed() still used)
}
```

**Assessment:**

- Same improvement as Change 1, applied to the property-based test.
- `observed()` is no longer applied to the success path (mutation 0); the returned `RunAdmission` is destructured and its fields asserted individually.
- Error paths (mutations 1-5) still use `observed()` for their `ObservedAdmissionDiagnostic` comparisons — correct because those are not `RunAdmission` objects.
- The proptest generators and case counts are unchanged (mutation in 0..6, name_suffix in 1..10).

---

## Verification Against Acceptance Criteria

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Authentic `artifact.policy_digest` captured before move | ✅ | Both tests capture `expected_policy_digest` from the artifact before consuming it |
| Independent expected digest (not `admission.policy_pin()` getter tautology) | ✅ | Expected value comes from `artifact.policy_digest`, not from the returned `RunAdmission` |
| All 7 fields retained from former `RunAdmission::new(...)` comparison | ✅ | Both tests assert `artifact_digest`, `run_id`, `granted_capabilities`, `policy`, `policy_pin`, `budget`, `idempotency_attested` |
| Deterministic: `Journaled` policy in unit test | ✅ | `given_valid_accepted_artifact_when_admitted...` uses `RuntimePolicy::Journaled` |
| Property: `Strict` policy in proptest | ✅ | `proptest_capability_profiles...` uses `RuntimePolicy::Strict` |
| Mutation 0 (allowed): full 7-field check | ✅ | Both tests check all fields on the success path |
| Mutations 1-5 (denied): existing error diagnostics unchanged | ✅ | Error paths still use `observed()` and compare `ObservedAdmissionDiagnostic` variants |
| Proptest generators/case counts unchanged | ✅ | `name_suffix in 1u8..10`, `mutation in 0u8..6` unchanged |
| Other synthetic diagnostic constructors unchanged | ✅ | `observed()` function and `ObservedAdmissionDiagnostic` enum unmodified |
| No broad `is_ok()`/`is_err()` substitution | ✅ | Specific assertions remain for both success and error paths |
| No test deletion | ✅ | Both original test functions preserved |
| Public APIs only | ✅ | All assertions use public accessor methods (`artifact_digest()`, `run_id()`, etc.) |

---

## Assertions Strength

The new assertions are strictly stronger than the old `RunAdmission::new(...)` comparison:

1. **Policy pin validation:** The old test could not detect a regression where `with_policy_pin` is removed or reset to `Unverified`. The new test explicitly asserts the pin is `Pinned(expected_policy_digest)`.

2. **Individual field isolation:** If a future change breaks one field (e.g., `policy()` returns wrong value), the individual assertion pinpoints it. The old monolithic `==` only reported "mismatch" without identifying which field.

3. **Non-tautological expectation:** The expected `policy_digest` is captured from the artifact *before* admission, not read back from the returned `RunAdmission`. This prevents the test from trivially passing regardless of whether the pin is actually bound.

---

## Conclusion

The two V21 fixture corrections are authentic, stronger, and verify the actual production behavior of `admit_artifact_run` including the policy pin binding that the old `RunAdmission::new(...)` comparison could not detect. No blocking defects found.

**APPROVED** for merge.

---

## Scope Limits

This is a TEST-only addendum to the V19 code and V20 historical wire + read-only reviews. This review does not claim full v1/recovery/physical disk acceptance. Final canonical evidence (including full integration test suite and formal verification artifacts) remains required per the bead tracker before landing.
