# Test Plan Review — vb-qi37.1.4

## Reviewer
- **State**: 9 (test-reviewer)
- **Bead**: vb-qi37.1.4
- **Date**: 2026-05-14

---

## MODE: 1 (Plan Inquisition)

Since cargo build fails due to verus dependency issue (not on crates.io), tests cannot be executed. Review is based on document analysis only.

---

## VERDICT: APPROVED WITH MINOR FINDINGS

---

## Axis 1 — Contract Parity

| Contract Function | Test Plan Coverage |
|---|---|
| `reject_unsupported_live_frame_state` | 6 BDD scenarios covering POST-001, POST-002, GAP-1, GAP-2 |
| `verify_digests` | 2 scenarios for workflow/IR mismatch at Full level |

**Assessment**: All critical functions have BDD scenarios. No missing functions.

---

## Axis 2 — Assertion Sharpness

| Scenario | Assertion | Status |
|---|---|---|
| `reject_returns_err_when_slot_taint_unsupported` | `Err(RuntimeError::InvalidRecoveryHydration)` | SHARP ✓ |
| `reject_returns_err_when_pending_actions_unsupported_and_not_empty` | `Err(RuntimeError::InvalidRecoveryHydration)` | SHARP ✓ |
| `reject_returns_ok_when_pending_actions_unsupported_but_empty` | `Ok(())` | SHARP ✓ |
| `verify_digests_full_returns_workflow_mismatch_error` | `Err(RecoveryError::WorkflowSourceDigestMismatch {...})` | SHARP ✓ |
| `verify_digests_full_returns_ir_mismatch_error` | `Err(RecoveryError::CompiledIrDigestMismatch {...})` | SHARP ✓ |

**Assessment**: All assertions specify exact error variants. No `is_ok()` or `is_err()` without exact values.

---

## Axis 3 — Trophy Allocation

| Layer | Planned | Status |
|---|---|---|
| Unit | 4 | ✓ |
| Integration | 2 | ✓ |
| Property | 0 | N/A — deterministic boolean conditions |
| Fuzz | 0 | N/A — no parsers/deserializers in scope |
| Kani | 1 | ✓ — RecoveryFrameSeed roundtrip |

**Assessment**: Allocation is reasonable. Ratio is appropriate for the scope.

---

## Axis 4 — Boundary Completeness

| Function | Boundaries Tested |
|---|---|
| `reject_unsupported_live_frame_state` | slot_taint=true, slot_values=true, action_payloads=true, pending_actions unsupported + non-empty, pending_actions unsupported + empty |
| `verify_digests` | Full level with workflow mismatch, IR mismatch, both match |

**Finding MINOR-1**: `reject_unsupported_live_frame_state` — minimum valid input (all flags false) is covered by existing tests but not explicitly named in test-plan.md. Not a blocker.

**Finding MINOR-2**: `verify_digests` — DigestCheck::WorkflowSourceOnly and DigestCheck::WorkflowAndIr levels not explicitly tested. These are covered by existing tests but not named in plan.

---

## Axis 5 — Mutation Survivability

Mental mutation check:

| Mutation | Test That Catches It |
|---|---|
| `|| seed.unsupported.slot_taint` removed | `reject_returns_err_when_slot_taint_unsupported` |
| `(!seed.pending_actions.is_empty() && seed.unsupported.pending_actions)` changed to just `seed.unsupported.pending_actions` | `reject_returns_err_when_pending_actions_unsupported_and_not_empty` + `reject_returns_ok_when_pending_actions_unsupported_but_empty` |
| `|| seed.unsupported.slot_values` removed | `reject_returns_err_when_slot_values_unsupported` |
| `|| seed.unsupported.action_payloads` removed | `reject_returns_err_when_action_payloads_unsupported` |
| `check_workflow_source_digest` removed | `verify_digests_full_returns_workflow_mismatch_error` |

**Assessment**: All critical mutations have corresponding tests.

---

## Axis 6 — Evidence Plan Audit

Test-plan.md correctly identifies:
- Preconditions in Given clauses
- Setup explicitly named
- Expected outcomes with exact error variants
- GAP-2 gap documented as negative test case

---

## Findings

### MINOR-1: Missing boundary test for minimum valid input
- **Location**: test-plan.md, Behavior Inventory
- **Problem**: All-flags-false (minimum valid seed) not explicitly named
- **Impact**: Low — existing tests cover this case
- **Required fix**: None — existing tests cover this

### MINOR-2: Intermediate DigestCheck levels not explicitly tested
- **Location**: test-plan.md, Behavior 5-6
- **Problem**: DigestCheck::WorkflowSourceOnly and WorkflowAndIr not named in scenarios
- **Impact**: Low — existing tests cover these
- **Required fix**: None — existing tests cover this

---

## Tooling Limitation Impact

Cargo build fails due to `verus = "^1"` not on crates.io. This prevents execution of:
- Tier 1: Test compile and nextest
- Tier 2: Coverage
- Tier 3: Mutation

Tests cannot be executed in current environment. Plan is sound based on document analysis.

---

## Summary

| Axis | Status |
|---|---|
| Contract Parity | PASS |
| Assertion Sharpness | PASS |
| Trophy Allocation | PASS |
| Boundary Completeness | MINOR-1, MINOR-2 |
| Mutation Survivability | PASS |
| Evidence Plan | PASS |

**VERDICT: APPROVED WITH MINOR FINDINGS** — 2 minor findings, no lethal or major. Tests are well-specified. Tooling limitation is environmental, not a test plan defect.

---

*test-plan-review: state 9 (test-reviewer) for vb-qi37.1.4*