# Fixture Repair: vb-h7rex (v21)

**Bead:** vb-h7rex
**Workspace:** vb-policy-fixture-v21 (isolated, based on frozen 7c3de06e)
**Task:** Fix `given_valid_accepted_artifact_when_admitted_then_admission_record_contains_digest_certificate_profile` in `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs`

## Problem

The test expected `RunAdmission::new(...)` which defaults `policy_pin` to `RunPolicyPin::Unverified`. Production code correctly journals `Admitted(RunAdmission)` with `policy_pin` set to `RunPolicyPin::Pinned(artifact.policy_digest)` for Journaled/Strict admission.

The test was written against the compatibility constructor and did not account for the actual policy digest binding that Strict/Journaled admission performs.

## Fix Applied

**File:** `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs`

### Changes

1. **Added import:** `use vb_storage::RunPolicyPin;`

2. **Captured policy digest before moving artifact:**
   ```rust
   let expected_policy_digest = artifact.policy_digest;
   ```
   (Before `let store = ScenarioStore { artifact };` moves the artifact)

3. **Replaced struct-level comparison with field-by-field public getter assertions:**
   - Extract `RunAdmission` from `Ok` result via match
   - Assert `admission.artifact_digest() == requested`
   - Assert `admission.run_id() == run_id`
   - Assert `admission.granted_capabilities() == &granted`
   - Assert `admission.policy() == RuntimePolicy::Journaled`
   - Assert `admission.policy_pin() == RunPolicyPin::Pinned(expected_policy_digest)`

4. **Error path:** Returns `Err(format!("expected admission, got error: {e:?}"))` on `Err` result (previously would have matched against `ObservedAdmissionDiagnostic` variant).

## Design Decisions

- `RunAdmission::with_policy_pin()` is `pub(crate)` and unavailable cross-crate, so the expected value cannot be constructed directly. Field-by-field assertions via public getters is the correct approach.
- The test now verifies the exact policy digest binding that the production admission path performs, rather than the `Unverified` default.
- All existing admission field checks (digest, run ID, grants, policy) are preserved and strengthened with the explicit pin check.

## Correction v2 (2026-10-22)

Coordinator inspection found two additional defects in the v21 fix.

**Defect 1: Journaled test lost budget/idempotency checks.** My field-by-field rewrite replaced the original full `RunAdmission` equality check and dropped the `budget(None)` and `idempotency_attested(empty)` assertions that were implicitly covered by struct equality. Restored those two assertions alongside the explicit `policy_pin()` check.

**Defect 2: Second failure in proptest_capability_profiles_admit_iff_required_capability_is_exactly_granted (line 1304).** The test's mutation 0 branch compared against `RunAdmission::new(...)` which defaults `policy_pin` to `Unverified`. The production admission path sets `Pinned(artifact.policy_digest)`, so mutation 0 always failed. Fixed by:
- Capturing `artifact.policy_digest` before moving the artifact to the store
- For mutation 0: matching `Ok(admission)` and asserting all seven admission fields with `prop_assert_eq!`, including `policy_pin() == Pinned(expected_policy_digest)`, `budget() == None`, and `idempotency_attested() == &[]`
- For mutations 1-5: wrapping the result with `observed()` and keeping the existing diagnostic assertions (CapabilityCountMismatch/CapabilityDenied)

**Commands run:** None. Source-only local 3090 task per bead specification. Coordinator is running full `--no-fail-fast` test suite separately.

**Verification:** Both tests now verify the actual policy digest binding performed by the production admission path. No fabricated or empty values. Actual test success is unknown; the coordinator's diagnostic run is still executing.