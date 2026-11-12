# Proof Review: vb-y4pa re-entry proof artifacts

## STATUS: REJECTED

---

## Executive Summary

**6/6 Kani harnesses are inadequate.** The `step_state_from_u8` function is **dead code** (never called). All harnesses hardcode `Succeeded` state instead of using `kani::any()`. Critical `kani::cover` statements for Succeeded→Running rejection are missing. PO-003 (`jump_to_body` helper) does not exist in `helpers.rs` — the primitives under test still use `jump_to` which lacks the Succeeded→Pending transition. The proof artifacts cannot demonstrate proof obligations until the implementation is fixed.

---

## PO Coverage Analysis

| PO | Obligation | Evidence Required | Status |
|----|------------|-------------------|--------|
| PO-001 | Succeeded→Pending in VALID_TRANSITIONS | `ValidTransition(Succeeded,Pending)==true` | ✓ DONE (step_state.rs:48) |
| PO-001 | `ValidTransition(Succeeded,Running)==false` (regression) | `kani::cover` statement | ✗ MISSING |
| PO-002 | `mark_pending` in RunFrame | Unit test + Kani harness | ✓ DONE (frame.rs:382) |
| PO-003 | `jump_to_body` helper | Unit test + Kani harness | ✗ **NOT IMPLEMENTED** |
| PO-004 | for_each_next fix | 2-item list iterates twice | ✗ **NOT FIXED** — for_each.rs:84 uses `jump_to` |
| PO-005 | reduce_next fix | reduce re-entry succeeds | ✗ **NOT FIXED** — reduce.rs:82 uses `jump_to` |
| PO-006 | collect_next fix | page2→body re-entry succeeds | ✗ **NOT FIXED** — collect.rs:521 uses `jump_to` |
| PO-007 | collect_page fix | page re-entry succeeds | ✗ **NOT FIXED** — collect.rs:397 uses `jump_to` |
| PO-008 | repeat_attempt fix | attempt re-entry succeeds | ✗ NOT FIXED |
| PO-009 | repeat_check fix | check routes to body | ✗ NOT FIXED |

---

## Critical Findings

### 1. DEAD CODE: `step_state_from_u8` Never Called

**Location:** `reentry_proofs.rs:41-52`

```rust
fn step_state_from_u8(v: u8) -> StepState {
    match v % 8 {
        0 => StepState::Pending,
        1 => StepState::Running,
        2 => StepState::Succeeded,
        3 => StepState::Failed,
        4 => StepState::Skipped,
        5 => StepState::Waiting,
        6 => StepState::Asking,
        _ => StepState::Cancelled,
    }
}
```

This function is **never invoked** in any of the 6 harnesses. It exists to support `kani::any()` state generation but is dead code. This violates the GOD RULES directive that Kani harnesses MUST use `kani::any()` for structural inputs.

### 2. No `kani::any()` Usage — All States Hardcoded

All 6 harnesses use the same pattern:
```rust
let body_step = StepIdx::new(1);
run.mark_running(body_step).unwrap();
run.mark_succeeded(body_step).unwrap();
```

This hardcodes **only** the Succeeded state. The `kani::proof` cannot explore arbitrary initial states. Each harness should use:
```rust
let body_state = kani::any::<StepState>();
run.write_step_state(body_step, body_state).unwrap();
```

### 3. Missing `kani::cover` for Succeeded→Running Rejection

PO-001 explicitly requires proving `ValidTransition(Succeeded, Running)==false (regression)`. None of the 6 harnesses contain a `kani::cover` statement to verify this invalid transition is rejected.

Required addition:
```rust
kani::cover!(
    run.step_state(body_step) == StepState::Succeeded,
    "body re-entry reached with Succeeded state"
);
// Then call primitive under test
// Finally verify Succeeded→Running is impossible:
kani::assert(
    !is_valid_transition(StepState::Succeeded, StepState::Running),
    "Succeeded→Running must be invalid"
);
```

### 4. PO-003 NOT Implemented — `jump_to_body` Does Not Exist

The proof obligation PO-003 requires a `jump_to_body(run, body)` helper that performs Succeeded→Pending before jump. This function does **not exist** in `crates/vb_runtime/src/primitives/helpers.rs`.

The primitives still use plain `jump_to`:
- `for_each.rs:84`: `jump_to(run, body)`
- `reduce.rs:82`: `jump_to(run, body)`
- `collect.rs:521`: `jump_to(run, body)`
- `collect.rs:397`: `jump_to(run, body)`

### 5. Weak Assertions — Only State Readability Checked

```rust
let state = run.step_state(body_step);
kani::assert(state.is_ok(), "step_state should be readable after for_each_next");
```

This only verifies the state is **readable**, not that it **transitioned correctly**. The assertions do not verify Succeeded→Pending transition occurred.

---

## Harness-by-Harness Review

### K-REENTRY-FE-1: `for_each_next_reentry` (lines 64-122)

| Requirement | Status |
|-------------|--------|
| Uses `kani::any()` for state | ✗ No — hardcodes Succeeded |
| `kani::cover` Succeeded→Running rejection | ✗ Missing |
| Harnesses PO-001 regression | ✗ No |
| Harnesses PO-004 fix | ✗ `jump_to_body` not implemented |
| Assertion verifies transition | ✗ Weak — only readability |

### K-REENTRY-RD-1: `reduce_next_reentry` (lines 126-166)

Same issues as FE-1. No `kani::any()`, no `kani::cover`, weak assertion.

### K-REENTRY-CL-1: `collect_next_reentry` (lines 170-237)

Same issues. `collect_next` uses `jump_to` at line 521 — fix not applied.

### K-REENTRY-CP-1: `collect_page_reentry` (lines 241-295)

Same issues. `collect_page` uses `jump_to` at line 397 — fix not applied.

### K-REENTRY-RPA-1: `repeat_attempt_reentry` (lines 299-330)

Same issues. `repeat_attempt` fix not verified.

### K-REENTRY-RPC-1: `repeat_check_reentry` (lines 334-371)

Same issues. `repeat_check` fix not verified.

---

## Unit Tests Review

### Test Names Don't Match PO-ID Format

| Unit Test | Should Be Named |
|-----------|-----------------|
| `for_each_two_item_reentry` | `PO-004-reentry` |
| `reduce_reentry` | `PO-005-reentry` |
| `collect_next_reentry` | `PO-006-reentry` |
| `collect_page_reentry` | `PO-007-reentry` |
| `repeat_attempt_reentry` | `PO-008-reentry` |
| `repeat_check_reentry` | `PO-009-reentry` |

### Missing Succeeded→Running Rejection Tests

No unit test verifies that the primitive correctly rejects or handles the Succeeded→Running invalid transition.

### Tests Pass Without Fix

The unit tests pass `Ok(EngineSignal::Continue)` but this is coincidental — the primitives still use `jump_to` without Succeeded→Pending transition. The tests do not actually verify the bug is fixed.

---

## Required Remediation

### Phase 1: Implement PO-003 (`jump_to_body`)
```rust
// helpers.rs
pub(crate) fn jump_to_body(
    run: &mut RunFrame,
    target: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    // Transition Succeeded→Pending before jump for loop body re-entry
    for step in 0..run.step_count() {
        let step_idx = StepIdx::new(step);
        if let Ok(state) = run.step_state(step_idx) {
            if state == StepState::Succeeded {
                run.mark_pending(step_idx)?;
            }
        }
    }
    run.set_pc(target)?;
    run.increment_executed()?;
    Ok(vb_core::EngineSignal::Continue)
}
```

### Phase 2: Fix Primitives
Replace `jump_to(run, body)` with `jump_to_body(run, body)` in:
- `for_each_next` (for_each.rs:84)
- `reduce_next` (reduce.rs:82)
- `collect_next` (collect.rs:521)
- `collect_page` (collect.rs:397)
- `repeat_attempt`
- `repeat_check`

### Phase 3: Rewrite Kani Harnesses
Replace hardcoded state with `kani::any()`:
```rust
#[kani::proof]
fn for_each_next_reentry() {
    let mut run = fresh_frame(4, 8);
    let mut store = ValueStore::new();
    // ... setup ...

    let body_state = kani::any::<StepState>();
    run.write_step_state(body_step, body_state).unwrap();

    // kani::cover to track Succeeded state reached
    kani::cover!(body_state == StepState::Succeeded, "Succeeded state before re-entry");

    let result = for_each_next(...);

    // Verify Succeeded→Running is invalid (regression check)
    kani::assert(
        !is_valid_transition(StepState::Succeeded, StepState::Running),
        "Succeeded→Running must be rejected"
    );

    // Verify transition occurred correctly
    match result {
        Ok(vb_core::EngineSignal::Continue) => {
            let new_state = run.step_state(body_step).unwrap();
            kani::assert(
                new_state == StepState::Running || new_state == StepState::Pending,
                "State must be Running or Pending after re-entry"
            );
        }
        Err(EngineError::InternalInvariantViolation { reason }) => {
            kani::assert(reason != "invalid_state_transition", "Re-entry must succeed");
        }
        _ => {}
    }
}
```

### Phase 4: Add `kani::cover` Statements
Each harness needs `kani::cover!` for:
- Body step in Succeeded state before re-entry
- Successful re-entry path
- Error path if re-entry fails

---

## Evidence

- `step_state_from_u8`: `reentry_proofs.rs:41-52` — defined but never called
- `kani::any()` usage: None in any of 6 harnesses
- `kani::cover` statements: None in any of 6 harnesses
- `jump_to_body`: NOT in `helpers.rs`
- Primitive fixes: NOT applied (all use `jump_to`)
- Test names: Do not match PO-ID format

---

## Verdict

**REJECTED** — The proof artifacts cannot demonstrate proof obligations because:

1. The implementation fix (PO-003 through PO-009) has not been applied
2. The Kani harnesses contain dead code and lack `kani::any()` state generation
3. Critical `kani::cover` statements for Succeeded→Running rejection are missing
4. Unit test names do not match PO-ID format
5. Unit tests pass coincidentally without verifying the actual bug fix

**Harnesses are not ready for cargo kani execution.**
