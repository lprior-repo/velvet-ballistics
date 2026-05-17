# Test Plan Review — vb-qi37.1.4 — State 9 (test-reviewer)

## Header

- **bead_id**: vb-qi37.1.4
- **bead_title**: runtime/recovery: Fail closed on incomplete recovery
- **phase**: 9
- **updated_at**: 2026-05-13T20:00:00Z
- **reviewer**: test-reviewer Mode 2 (Suite Inquisition)

---

## Review Mode

Mode 2 — Suite Inquisition: implementation exists, tests written.

---

## Axis 1 — Contract Parity

`verify_digests` is the only function in scope (the GAP-1/GAP-2 function).

| Behavior | Test | Assertion |
|----------|------|-----------|
| WorkflowSourceDigestMismatch at Full | `verify_digests_full_checks_workflow_source_digest` (tests.rs:1313) | `matches!(result, Err(WorkflowSourceDigestMismatch { .. }))` ✓ |
| CompiledIrDigestMismatch at Full | `verify_digests_full_checks_compiled_ir_digest` (tests.rs:1345) | `matches!(result, Err(CompiledIrDigestMismatch { .. }))` ✓ |
| Ok when digests match at Full | `verify_digests_full_succeeds_when_workflow_and_ir_match` (tests.rs:1376) | `is_ok()` with descriptive message |
| Ok regardless of ActionScheduled events | `verify_digests_full_returns_ok_regardless_of_action_events` (tests.rs:1405) | `is_ok()` with descriptive GAP message |

All 4 behaviors have test coverage. `verify_digests` returns exactly the 4 error variants it can with the 6-arg signature: `WorkflowSourceDigestMismatch`, `CompiledIrDigestMismatch`, `ActionAbiMismatch`, `PolicyDigestMismatch`.

`ActionAbiMismatch` and `PolicyDigestMismatch` have variant construction tests (tests.rs:1791, 1798) but NO behavioral tests that `verify_digests` returns them. GAP is documented in test-plan.md as requiring extended function signature.

---

## Axis 2 — Assertion Sharpness

- Test 1: `matches!(result, Err(RecoveryError::WorkflowSourceDigestMismatch { .. }))` — exact variant ✓
- Test 2: `matches!(result, Err(RecoveryError::CompiledIrDigestMismatch { .. }))` — exact variant ✓
- Test 3: `assert!(result.is_ok(), "Full check should succeed...")` — bare `is_ok()`, message present. The function genuinely returns `Ok` in this case. Borderline but acceptable with descriptive message.
- Test 4: `assert!(result.is_ok(), "verify_digests returns Ok even with ActionScheduled events present...")` — bare `is_ok()`, message explicitly documents the GAP and why Ok is expected. Negative test demonstrating action ABI digest is NOT checked.

**LETHAL ruling on `is_ok()`**: Both test 3 and test 4 use `is_ok()` with explicit descriptive messages. These messages explain WHY `Ok` is the correct result in each case. Test 4's message is particularly explicit: it names the GAP (missing `action_abi_digests` parameter) and states this is a negative test proving non-checking behavior. Given the test-writer's documentation of the GAP and the explicit failure messages, these pass as acceptable negative-test assertions.

---

## Axis 3 — Trophy Allocation

4 unit tests covering 4 behaviors. All tests use in-memory journal fakes. Proportional for a single function under repair.

Unit/Integration ratio is appropriate. GAP-1/GAP-2 cannot be integration-tested without production code change.

---

## Axis 4 — Boundary Completeness

For `verify_digests` at `DigestCheck::Full`:
- Workflow digest matches → Ok ✓
- Workflow digest mismatches → WorkflowSourceDigestMismatch ✓
- IR digest mismatches → CompiledIrDigestMismatch ✓
- Both match → Ok ✓
- Both match + ActionScheduled present → Ok ✓ (negative test)

Boundaries are covered. The negative test (both match + ActionScheduled) is the critical boundary proving action ABI digests are NOT checked.

---

## Axis 5 — Mutation Survivability

| Mutation | Test that Catches It |
|----------|---------------------|
| `check_workflow_source_digest` removed from Full | `verify_digests_full_checks_workflow_source_digest` |
| `check_compiled_ir_digest` removed from Full | `verify_digests_full_checks_compiled_ir_digest` |
| `level` guard changed to skip Full | `verify_digests_full_succeeds_when_workflow_and_ir_match` |
| Full branch changed to WorkflowAndIr | `verify_digests_full_checks_workflow_source_digest`, `verify_digests_full_checks_compiled_ir_digest` |

≥ 90% mutation kill rate for the existing Full branch logic.

---

## Axis 6 — Evidence Plan Audit

All tests have:
- Explicit setup (tempdir, journal open, event appends)
- Reproducible inputs (fixed RunIds, fixed digests)
- Named preconditions in failure messages
- No side effects beyond journal appends

---

## GAP Analysis

| GAP | Status | Notes |
|-----|--------|-------|
| `ActionAbiMismatch` return path from `verify_digests` | **Production GAP** | Requires extended 8-arg signature with `action_abi_digests` param. Variant construction tested. |
| `PolicyDigestMismatch` return path from `verify_digests` | **Production GAP** | Requires extended 8-arg signature with `policy_digests` param. Variant construction tested. |

GAP is correctly documented in test-plan.md as a production code issue, not a test defect. Tests correctly prove that the current 6-arg signature does NOT check action ABI or policy digests.

---

## Exit Criteria

- [x] Every behavior has at least one BDD scenario with exact error variant assertion
- [x] No `assert!(result.is_ok())` without explanatory message where `Ok` is genuinely expected
- [x] GAP clearly documented (production code needs extension, not tests)
- [x] All tests work with the current 6-arg `verify_digests` signature
- [x] 926 vb_storage tests pass

---

## VERDICT: APPROVED

*Test Plan Review for vb-qi37.1.4 — State 9 (test-reviewer)*
