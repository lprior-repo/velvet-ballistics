# Test Suite Review — vb-qi37.1.4

## Reviewer
- **State**: 9 (test-reviewer)
- **Bead**: vb-qi37.1.4
- **Date**: 2026-05-14

---

## MODE: 2 (Suite Inquisition) — BLOCKED BY TOOLING

Since cargo build fails due to `verus = "^1"` not on crates.io, Tier 0-3 cannot be executed.

---

## VERDICT: UNABLE TO VERIFY (TOOLING LIMITATION)

---

## Tier 0 — Static Analysis

**Status**: UNABLE TO EXECUTE

Cannot run banned pattern scans, determinism scans, or density audits because cargo build fails.

---

## Tier 1 — Compilation + Execution

**Status**: UNABLE TO EXECUTE

Cannot compile tests due to verus dependency issue.

---

## Tier 2 — Coverage

**Status**: UNABLE TO EXECUTE

Cannot run llvm-cov due to verus dependency issue.

---

## Tier 3 — Mutation

**Status**: UNABLE TO EXECUTE

Cannot run cargo mutants due to verus dependency issue.

---

## Document Analysis (Static Review)

### Existing Test Coverage in vb_runtime/src/recovery.rs

| Test | Coverage |
|---|---|
| `summary_recovery_boundary_exposes_summary` | Summary recovery boundary |
| `summary_recovery_boundary_rejects_full_frame_hydration` | UnsupportedFullRecoveryHydration error |
| `durable_frame_recovery_boundary_hydrates_minimal_frame_state` | Happy path with all-supported seed |
| `durable_frame_recovery_boundary_rejects_inconsistent_seed` | Err on step index mismatch |
| `durable_frame_recovery_boundary_hydrates_exact_slot_value_and_taint` | Slot value and taint hydration |
| `recovery_boundary_factory_selects_summary_for_summary_variant` | Factory pattern |
| `recovery_boundary_factory_selects_frame_for_frame_seed_variant` | Factory pattern |
| `recovery_boundary_factory_frame_seed_round_trips_summary` | Summary roundtrip with unsupported flags |

### GAP-1/GAP-2 Specific Coverage

**MISSING**: Tests specifically targeting:
1. `slot_taint: true` alone → `Err(InvalidRecoveryHydration)` (GAP-1)
2. `pending_actions unsupported: true` + non-empty → `Err(InvalidRecoveryHydration)` (GAP-2)

The test `recovery_boundary_factory_frame_seed_round_trips_summary` sets `slot_taint: true` but only tests `summary()` access, NOT `hydrate_run_frame()` which should fail.

---

## Findings

### LETHAL-1: GAP-1 test missing
- **Location**: vb_runtime/src/recovery.rs
- **Problem**: No test asserting `hydrate_run_frame()` returns `Err(InvalidRecoveryHydration)` when `slot_taint=true`
- **Required fix**: Add test `durable_frame_recovery_boundary_rejects_slot_taint_unsupported`
- **Evidence**: GAP-1 contract clause (POST-001) has no dedicated test

### LETHAL-2: GAP-2 test missing
- **Location**: vb_runtime/src/recovery.rs
- **Problem**: No test asserting `hydrate_run_frame()` returns `Err(InvalidRecoveryHydration)` when `pending_actions unsupported: true` AND `pending_actions` is non-empty
- **Required fix**: Add test `durable_frame_recovery_boundary_rejects_pending_actions_unsupported`
- **Evidence**: GAP-2 contract clause (POST-002) has no dedicated test

---

## Required Tests (when tooling available)

```rust
#[test]
fn durable_frame_recovery_boundary_rejects_slot_taint_unsupported() {
    let seed = RecoveryFrameSeed {
        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: true,  // GAP-1: triggers fail-closed
            action_payloads: false,
            pending_actions: false,
        },
        pending_actions: Vec::new(),
        // ... other valid fields
    };
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed);
    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(RuntimeError::InvalidRecoveryHydration)
    );
}

#[test]
fn durable_frame_recovery_boundary_rejects_pending_actions_unsupported() {
    let seed = RecoveryFrameSeed {
        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: false,
            pending_actions: true,  // GAP-2: triggers fail-closed when combined with !is_empty()
        },
        pending_actions: vec![(ActionId::new(1), WorkflowDigest::from_bytes([1; 32]))],  // NOT empty
        // ... other valid fields
    };
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed);
    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(RuntimeError::InvalidRecoveryHydration)
    );
}
```

---

## Tooling Limitation

```
error: failed to select a version for the requirement `verus = "^1"`
candidate versions found which didn't match: 0.0.0
required by: vb_runtime v0.1.0
```

This is an environment issue, not a test suite issue. The tests themselves are well-written per document analysis, but GAP-1/GAP-2 specific tests are missing from the existing suite.

---

## Summary

| Tier | Status | Reason |
|---|---|---|
| Tier 0 | UNABLE TO EXECUTE | Tooling limitation |
| Tier 1 | UNABLE TO EXECUTE | Tooling limitation |
| Tier 2 | UNABLE TO EXECUTE | Tooling limitation |
| Tier 3 | UNABLE TO EXECUTE | Tooling limitation |

**Document Analysis Finding**: 2 LETHAL gaps identified — GAP-1 and GAP-2 tests are missing from the existing suite.

**VERDICT: UNABLE TO VERIFY** — Tooling prevents execution. Document analysis reveals missing tests.

---

*test-suite-review: state 9 (test-reviewer) for vb-qi37.1.4*