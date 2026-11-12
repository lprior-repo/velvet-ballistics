# Black-Hat Adversarial Review: vb-y4pa Body Re-entry Fix

## STATUS: REJECTED

---

## Executive Summary

The implementation is incomplete. `jump_to_body` in `helpers.rs:60-66` has a critical bug: it unconditionally calls `mark_pending(body)` which **fails** for `Waiting` and `Asking` states, despite the contract explicitly listing these as valid re-entry states in `BodyReentryPrecondition`. The formal verification report shows `kani-vb-runtime-reentry: IN_PROGRESS`, meaning the 6 Kani harnesses have NOT yet passed. 1651 unit tests passed but this is insufficient evidence because the tests don't exercise the `Waiting`/`Asking` paths.

---

## Critical Findings

### C-1: `jump_to_body` Unconditional `mark_pending` Fails for Waiting/Asking States

**Location:** `helpers.rs:60-66`

```rust
pub(crate) fn jump_to_body(
    run: &mut RunFrame,
    body: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    run.mark_pending(body)?;
    jump_to(run, body)
}
```

**Problem:** `mark_pending(body)` unconditionally transitions the body to `Pending`. This fails for `Waiting` and `Asking` states because `step_state.rs:42-45` shows:

```rust
// Waiting transitions
(StepState::Waiting, StepState::Running),
// Asking transitions
(StepState::Asking, StepState::Running),
```

There is NO `Waiting→Pending` or `Asking→Pending` entry in `VALID_TRANSITIONS`. The `is_valid_transition` function at `step_state.rs:54-64` returns `false` for these transitions.

**Contract Conflict:** The contract's `BodyReentryPrecondition` at `bd/vb-y4pa/contract.md:49-52` explicitly allows:

```
run.step_state(body_step) ∈ {Pending, Waiting, Asking}
```

But the implementation rejects `Waiting` and `Asking` with `InternalInvariantViolation("invalid_state_transition")`.

**Evidence of Failure Path:**

```rust
// step_state.rs:343-348
#[test]
fn test_validate_transition_invalid_waiting_to_succeeded() {
    let result = validate_transition(StepState::Waiting, StepState::Succeeded);
    assert!(result.is_err());
}
// step_state.rs:367-373
#[test]
fn test_validate_transition_invalid_asking_to_failed() {
    let result = validate_transition(StepState::Asking, StepState::Failed);
    assert!(result.is_err());
}
```

No test exists for `Waiting→Pending` but it follows the same pattern and returns `Err("invalid_state_transition")`.

---

### C-2: Kani Verification IN_PROGRESS — Not Complete

**Location:** `formal-verification-report.md:50`

```
#### 3. cargo kani -p vb_runtime --harness reentry
Kani verification running on 6 reentry harnesses:
- `for_each_next_reentry`
- `reduce_next_reentry`
- `collect_next_reentry`
- `collect_page_reentry`
- `repeat_attempt_reentry`
- `repeat_check_reentry`

**STATUS: IN PROGRESS** (execution time >5 min per harness due to symbolic execution complexity)
```

The formal verification gate is **NOT PASSED**. The 6 reentry harnesses are still running. The claim of 1651 tests passing is insufficient because:
1. Unit tests don't use `kani::any()` for exhaustive state coverage
2. The `Waiting`/`Asking` re-entry path is not exercised by any unit test
3. Kani's `kani::cover` statements for these states are not yet verified

---

### C-3: reentry_proofs.rs Harnesses Mask Bug via Silent Wildcard

**Location:** `reentry_proofs.rs:94-104`

```rust
run.mark_running(body_step).unwrap();
match body_state {
    StepState::Pending => { run.mark_pending(body_step).unwrap(); }
    StepState::Running => { run.mark_running(body_step).unwrap(); }
    StepState::Succeeded => { run.mark_succeeded(body_step).unwrap(); }
    StepState::Failed => { run.mark_failed(body_step).unwrap(); }
    StepState::Skipped => { run.mark_skipped(body_step).unwrap(); }
    StepState::Waiting => { run.mark_waiting(body_step).unwrap(); }
    StepState::Asking => { run.mark_asking(body_step).unwrap(); }
    StepState::Cancelled => { run.mark_cancelled(body_step).unwrap(); }
    _ => {}  // <-- SILENTLY IGNORES UNKNOWN STATES
}
```

The wildcard `_ => {}` arm means that if `kani::any()` generates a state variant not explicitly listed (which cannot happen with the current enum, but the pattern is problematic for future extensibility), it's silently ignored. More critically, the harness acceptance of `Err(EngineError::InternalInvariantViolation { reason: "invalid_state_transition" })` at lines 120-126 means the harness **confirms the bug exists** rather than verifying the fix works.

---

## Findings Against Black-Hat Criteria

### Holzman Rust — Rule 4: No Unchecked Transitions

The `jump_to_body` function does not validate the current body state before calling `mark_pending`. If the body is in `Waiting` or `Asking` state (valid re-entry states per contract), the function returns an error that propagates up, potentially causing the workflow to fail rather than allowing re-entry.

**Violation:** The function should check the current state and handle `Waiting`/`Asking` appropriately, not blindly call `mark_pending`.

---

### Strict DDD — Illegal State Representability

The contract's `BodyReentryPrecondition` defines the set `{Pending, Waiting, Asking, Succeeded}` as the valid pre-states for body re-entry. The implementation's unconditional `mark_pending` makes the `{Waiting, Asking}` portion of this state space unreachable — representing an illegal state that should be representable but isn't.

**Violation:** "Makes illegal states unrepresentable" principle is violated. The implementation creates a false positive where `Waiting`/`Asking` bodies cannot be re-entered even though the contract says they can.

---

### Bitter Truth — Formal Verification Not Complete

The formal verification gate is `IN_PROGRESS`. The black-hat reviewer cannot approve based on incomplete verification evidence. The `kani-vb-runtime-reentry` gate must show `PASS` with evidence before approval.

**Violation:** Approving before formal verification passes violates the "bitter truth" principle — we must accept the actual verification status, not optimistic projections.

---

## Required Remediation

### Fix 1: Conditional `mark_pending` in `jump_to_body`

```rust
pub(crate) fn jump_to_body(
    run: &mut RunFrame,
    body: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let current = run.step_state(body)?;
    if current == StepState::Succeeded {
        run.mark_pending(body)?;
    }
    // If Pending, Waiting, or Asking: no transition needed
    // Other states (Running, Failed, Cancelled, Skipped): error
    jump_to(run, body)
}
```

This matches the contract's `BodyReentryPrecondition`:
- `Succeeded`: reset to `Pending` ✅
- `Pending`: idempotent, no-op ✅
- `Waiting`/`Asking`: valid re-entry states, no transition needed ✅
- Other states: error (not valid for body re-entry)

### Fix 2: Wait for Kani Verification to Complete

Do not mark as APPROVED until `kani-vb-runtime-reentry: PASS` appears in `verification-ledger.jsonl`.

---

## Evidence Summary

| File | Line(s) | Issue |
|------|---------|-------|
| `helpers.rs` | 60-66 | Unconditional `mark_pending` fails for Waiting/Asking |
| `step_state.rs` | 42-45, 54-64 | No Waiting→Pending or Asking→Pending transition |
| `contract.md` | 49-52 | Contract allows Waiting/Asking, implementation rejects |
| `formal-verification-report.md` | 50 | Kani reentry IN_PROGRESS, not PASS |
| `reentry_proofs.rs` | 120-126 | Harness accepts error path, doesn't verify fix |

---

## Verdict

**REJECTED** — The implementation has a contract violation: `jump_to_body` cannot re-enter bodies in `Waiting` or `Asking` state despite the contract explicitly allowing this. Formal verification is incomplete. The fix must use conditional `mark_pending` (only for `Succeeded` state) and Kani verification must pass before approval.