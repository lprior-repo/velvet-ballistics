# Test Repair Guide — vb-rpch: Durability and Recovery Acceptance Scenarios

This guide specifies the exact changes required to move from REJECTED to APPROVED.

---

## LETHAL-1: Fix Bare `is_ok()` Assertion

**File:** `crates/vb_storage/tests/recovery_bdd_tests.rs`
**Function:** `snapshot_plus_tail_applies_tail_after_watermark`
**Location:** Lines 247–306

### Current (BROKEN):
```rust
let result = hydrate_run_frame(&snapshot, &tail, run);
assert!(
    result.is_ok(),
    "hydrate_run_frame should succeed when tail events are after snapshot seq: {result:?}"
);
} // ← ends without inspecting frame
```

### Required Fix:
Add frame validation after the `is_ok()` guard:
```rust
let result = hydrate_run_frame(&snapshot, &tail, run);
let frame = result.expect("hydrate_run_frame should succeed when tail events are after snapshot seq");
// Verify tail step was recorded
assert_eq!(
    frame.pc(),
    StepIdx::new(1),
    "PC must advance to step 1 after tail StepStarted"
);
assert_eq!(
    frame.step_count(),
    1,
    "step_count must reflect tail events"
);
```

---

## LETHAL-2: Increase Test Density

**Gap:** 35 tests exist, 70 required (5× × 14 contract functions). Need +35 tests.

### Priority 1: Pure Function Unit Tests

Add to `crates/vb_storage/src/recovery/replay/summary.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use vb_core::{ActionId, SlotIdx, StepIdx};
    use vb_storage::JournalEvent;

    // INV: apply_summary_event counter monotonicity
    proptest! {
        #[test]
        fn apply_summary_event_counters_monotonic(events: Vec<JournalEvent>) {
            // Filter to valid seq/run combinations to avoid precondition failures
            let mut summary = RecoveryRuntimeSummary::default();
            for event in &events {
                apply_summary_event(&mut summary, event);
            }
            // All counters should be non-decreasing
            prop_assert!(summary.steps_started >= 0);
            prop_assert!(summary.steps_succeeded <= summary.steps_started);
            prop_assert!(summary.actions_resolved <= summary.actions_scheduled);
        }
    }

    // INV: dimension_count overflow safety
    #[test]
    fn dimension_count_overflow_at_u16_max() {
        // When max_idx.index() + 1 would overflow u16, returns Err
        let result = dimension_count(StepIdx::new(u16::MAX), RunId::new(1));
        prop_assert!(result.is_err());
    }

    #[test]
    fn dimension_count_valid_at_u16_max_minus_1() {
        // One below overflow boundary should succeed
        let result = dimension_count(StepIdx::new(u16::MAX - 1), RunId::new(1));
        prop_assert!(result.is_ok());
        assert_eq!(result.unwrap(), (u16::MAX - 1) as usize + 1);
    }
}
```

Add to `crates/vb_storage/src/recovery/types.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // INV: UnsupportedRecoveryState::union algebraic properties
    proptest! {
        #[test]
        fn union_is_commutative(a: UnsupportedRecoveryState, b: UnsupportedRecoveryState) {
            assert_eq!(a.union(b), b.union(a));
        }

        #[test]
        fn union_is_idempotent(a: UnsupportedRecoveryState) {
            assert_eq!(a.union(a), a);
        }

        #[test]
        fn union_is_associative(a: UnsupportedRecoveryState, b: UnsupportedRecoveryState, c: UnsupportedRecoveryState) {
            assert_eq!(a.union(b).union(c), a.union(b.union(c)));
        }

        #[test]
        fn union_with_supported_returns_original(a: UnsupportedRecoveryState) {
            assert_eq!(a.union(UnsupportedRecoveryState::SUPPORTED), a);
            assert_eq!(UnsupportedRecoveryState::SUPPORTED.union(a), a);
        }
    }

    // INV: ActionReplayTracker::is_resolved monotonicity
    proptest! {
        #[test]
        fn is_resolved_monotonic_after_mark_completed(action: ActionId, step: StepIdx) {
            let mut tracker = ActionReplayTracker::new();
            tracker.mark_completed(action, step);
            assert!(tracker.is_resolved(action, step));
            // Calling again should not revert
            assert!(tracker.is_resolved(action, step));
        }
    }
}
```

### Priority 2: Missing Boundary Tests

Add to `crates/vb_storage/tests/recovery_bdd_tests.rs`:

```rust
// verify_digests at WorkflowSourceOnly level with mismatch
#[test]
fn verify_digests_workflow_source_only_rejects_mismatch() {
    // ... (test verify_digests at WorkflowSourceOnly level with wrong digest)
}

// hydrate_run_frame_from_events with empty events
#[test]
fn hydrate_from_events_empty_returns_no_recovery_data() {
    let result = hydrate_run_frame_from_events(&[], RunId::new(1));
    let Err(RecoveryError::NoRecoveryData { .. }) = result else {
        panic!("expected NoRecoveryData for empty events");
    };
}

// recover_all_incomplete_runs excludes runs with terminal events
#[test]
fn recover_all_incomplete_runs_excludes_finished_runs() {
    // ... (journal with RunFinished; result should be empty)
}
```

---

## LETHAL-3: Resolve TerminalStateMismatch

**Option A (Preferred — unblock the test):**

Add to `crates/vb_storage/src/recovery/recover.rs`:
```rust
/// Like `recover_runtime_summary` but validates the recovered terminal state
/// against an expected value, returning `TerminalStateMismatch` if they differ.
pub fn recover_runtime_summary_with_expected(
    journal: &FjallJournal,
    run: RunId,
    expected_terminal: Option<RecoveryTerminalState>,
) -> RecoveryResult<RecoveryHydration> {
    let hydration = recover_runtime_summary(journal, run)?;
    if let (Some(expected), RecoveryHydration::Summary(summary)) = (expected_terminal, &hydration) {
        if summary.terminal != expected {
            return Err(RecoveryError::TerminalStateMismatch {
                expected,
                found: summary.terminal,
            });
        }
    }
    Ok(hydration)
}
```

Then add test:
```rust
#[test]
fn recover_runtime_summary_detects_terminal_state_mismatch() {
    // ... journal with RunFinished, call recover_runtime_summary_with_expected
    // with a different expected terminal, assert TerminalStateMismatch
}
```

**Option B (Formal Waiver):**

Record a DEFERRED_GLOBAL waiver in `proof-obligations.planned.jsonl`:
```json
{
  "id": "PO-VB-039",
  "requirement_id": "ERR-TerminalStateMismatch",
  "contract_clause": "POST-004",
  "risk": "low",
  "verifier": "none",
  "artifact": "N/A",
  "command": "N/A",
  "expected_evidence": "DEFERRED_GLOBAL: no expected-terminal parameter in public API",
  "assumptions": ["recover_runtime_summary takes no expected_terminal parameter"],
  "required": false,
  "mode": "none",
  "owner_state": 4,
  "status": "waived",
  "waiver": {
    "clause_id": "POST-004",
    "reason": "No expected-terminal parameter exists in public API; TerminalStateMismatch cannot be triggered without API addition",
    "compensating_evidence": "Contract signature confirms no expected_terminal param; GAP tracked in vb-ty9",
    "owner": "vb-rpch"
  }
}
```

---

## MAJOR-3: Fix Assertion Sharpness

### `wait_identity_and_state_survive_across_restart` (lines 476-479)

**Before:**
```rust
assert!(summary.suspensions >= 1, "one wait suspension must be counted");
```

**After:**
```rust
assert_eq!(summary.suspensions, 1, "one wait suspension must be counted");
```

### `unsequenced_lifecycle_events_do_not_change_recovered_state` (lines 1043-1046)

**Before:**
```rust
assert!(summary.steps_started >= 1, ...);
```

**After:**
```rust
assert_eq!(summary.steps_started, 1, "step count must ignore unsequenced events");
```

### `resolved_action_not_reexecuted_on_restart` (lines 653-661)

**Before (multi-outcome):**
```rust
assert!(result2.is_ok() || matches!(result2, Err(RecoveryError::NonIdempotentActionBlocked {...})));
```

**After (single sharp outcome — choose ONE per contract):**
```rust
// Per POST-009: replay_blocks already-resolved non-idempotent actions
let Err(RecoveryError::NonIdempotentActionBlocked { action, step }) = result2 else {
    panic!("expected NonIdempotentActionBlocked for already-resolved action, got: {result2:?}");
};
assert_eq!(action, action_id);
assert_eq!(step, StepIdx::ZERO);
```

Or if the intended behavior is `Ok` (idempotent replay), assert `Ok` with specific field checks.

---

## Sync Workdir to Source Checkout

The workdir test file has 31 tests; source checkout has 35. The 2 `#[ignore]` tests (action_abi_mismatch, policy_digest_mismatch) should be removed (source checkout removed them), and 4 additional tests appear to have been added.

Ensure the workdir `recovery_bdd_tests.rs` matches the source checkout version before resubmission.

---

## Summary of Required Changes

| Priority | Change | Impact |
|----------|--------|--------|
| P0 | Fix `snapshot_plus_tail_applies_tail_after_watermark` frame validation | Resolves LETHAL-1 |
| P0 | Add 35 unit/integration tests | Resolves LETHAL-2 density |
| P0 | Resolve TerminalStateMismatch (API or waiver) | Resolves LETHAL-3 |
| P1 | Implement 4 proptest invariants | Resolves MAJOR-1 |
| P1 | Fix 5+ sharp assertion violations | Resolves MAJOR-3 |
| P1 | Add 3 missing boundary tests | Resolves MAJOR-5 |
| P2 | Sync workdir to source checkout | Resolves MINOR-1 |
| P2 | Add `recover_run_admission` scenario | Resolves MINOR |
