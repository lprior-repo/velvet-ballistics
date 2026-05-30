# ARCHITECTURAL DRIFT HAMMER REPORT
## Target: `vb_runtime/src/primitives/reentry_proofs.rs`
## Line Count: 592 (LIMIT: 300) — VIOLATION: 197% of limit

---

## EXECUTIVE SUMMARY

This file is a **VERIFICATION HARNESS ONLY** — 100% Kani proof code with zero production logic. The architectural violations are severe: primitive obsession dominance, DRY collapse, and structural chaos in a file nearly twice the size limit.

---

## VIOLATION 1: LINE COUNT (PRIMARY)

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 592 | 300 | **VIOLATION** |
| Overage | +292 | — | 197% of limit |
| Harmonic Mean Overage | 2.0x | — | CATASTROPHIC |

---

## VIOLATION 2: PRIMITIVE OBSESSION DOMINANCE

### 2.1 StepState Match Explosion (CRITICAL)

**SIX IDENTICAL copies** of the StepState→marker match block exist at:
- Lines 104-130 (`for_each_next_reentry`)
- Lines 192-218 (`reduce_next_reentry`)
- Lines 299-325 (`collect_next_reentry`)
- Lines 396-422 (`collect_page_reentry`)
- Lines 474-500 (`repeat_attempt_reentry`)
- Lines 545-571 (`repeat_check_reentry`)

```rust
match body_state {
    StepState::Pending   => { run.mark_pending(body_step).unwrap(); }
    StepState::Running  => { run.mark_running(body_step).unwrap(); }
    StepState::Succeeded=> { run.mark_succeeded(body_step).unwrap(); }
    StepState::Failed   => { run.mark_failed(body_step).unwrap(); }
    StepState::Skipped  => { run.mark_skipped(body_step).unwrap(); }
    StepState::Waiting  => { run.mark_waiting(body_step).unwrap(); }
    StepState::Asking   => { run.mark_asking(body_step).unwrap(); }
    StepState::Cancelled=>{ run.mark_cancelled(body_step).unwrap(); }
    _ => {}
}
```

**PRIMITIVE OBSESSION:** This is 8-arm matching on an enum's discriminant treated as primitive data. No domain transition model exists. The relationship between `StepState` variants and the `RunFrame` mutation is unmodeled.

**DRY COLLAPSE:** 6 copies × 27 lines = **162 lines of pure duplication**.

### 2.2 Dead Code: `step_state_from_u8` (Lines 46-57)

```rust
fn step_state_from_u8(v: u8) -> StepState {
    match v % 8 {
        0 => StepState::Pending,
        1 => StepState::Running,
        // ... 8 arms total
    }
}
```

**Never called anywhere.** A utility that should enable property-based state generation exists but is unused. This suggests incomplete abstraction — the harness author started to build a state factory but didn't complete the refactor.

### 2.3 Raw Slot Manipulation: `list_in_slot` (Lines 36-44)

```rust
fn list_in_slot(
    run: &mut RunFrame,
    store: &mut ValueStore,
    slot: SlotIdx,
    items: Vec<SlotValue>,
) {
    let id = store.insert_list(items.into_boxed_slice()).unwrap();
    run.write_slot(slot, SlotValue::List(id)).unwrap();
}
```

**PRIMITIVE OBSESSION:** Slots are raw `SlotIdx` integers manipulated directly. No domain type like `ListSourceSlot`, `IteratorSlot`, or `CollectorSlot` exists. The domain concept "a slot containing a list" is not modeled — only raw slot writes.

### 2.4 Packed Integer Encoding: `repeat_attempt_reentry` / `repeat_check_reentry` (Lines 461-463, 532-534)

```rust
let packed: i64 = (3_i64 << 32) | 1_i64;
run.write_slot(attempt_slot, SlotValue::I64(packed)).unwrap();
```

**PRIMITIVE OBSESSION:** Bit-shifted i64 encoding is raw manipulation. No `AttemptCount` or `RetryPolicy` value object. The packed format is a pure implementation detail exposed in test code.

---

## VIOLATION 3: DRY COLLAPSE — EVIDENCE

| Pattern | Copies | Lines Each | Total Wasted |
|---------|--------|------------|--------------|
| StepState match block | 6 | ~27 | ~162 |
| `fresh_frame` identical call | 6 | 1 | 6 |
| `kani::cover!(body_state == StepState::Succeeded, ...)` | 6 | 1 | 6 |
| `kani::cover!(body_state == StepState::Pending, ...)` | 2 | 1 | 2 |
| Result matching pattern | 6 | ~10 | ~60 |
| **TOTAL** | | | **~236 lines** |

**Net unique logic:** ~356 lines of 592 are pure duplication.

---

## VIOLATION 4: STRUCTURAL CHAOS

### 4.1 Single Module, No Organization

592 lines in one `pub mod reentry_harnesses`. No:
- Submodules for `state_transitions`, `harness_builders`, `proofStrategies`
- Trait for common harness behavior
- Shared `ReentryProof` trait or `StepContext` struct

### 4.2 Harness Result Inconsistency

| Harness | Error Path Behavior |
|---------|---------------------|
| `for_each_next_reentry` | Returns `()` on error match |
| `reduce_next_reentry` | Returns `()` on error match |
| `collect_next_reentry` | Returns `()` on error match |
| `collect_page_reentry` | Returns `()` on error match |
| `repeat_attempt_reentry` | Returns `()` on error match |
| `repeat_check_reentry` | Returns `()` on error match |

All handle errors identically, but **no shared error handling trait** exists. The `start_result.is_err()` check is handled differently:
- `collect_next_reentry` (line 286-288): `return;` on error
- `collect_page_reentry` (line 374): `_ =` discard on error

### 4.3 Step Index Hardcoding

All 6 harnesses use identical step indices:
```rust
let body = StepIdx::new(1);
let done = StepIdx::new(2);
```

No variation. No test of different topologies. The harness doesn't test boundary conditions for step indices.

---

## VIOLATION 5: WEAK VERIFICATION LOGIC

### 5.1 `kani::cover` Misuse

```rust
kani::cover!(
    body_state == StepState::Succeeded,
    "for_each_next re-entry with Succeeded body state"
);
```

**`kani::cover` only verifies code coverage — NOT correctness.** This proves the `Succeeded` branch was explored, not that `for_each_next` handles it correctly. The cover statement is verification theater.

### 5.2 Trivial Assertions

```rust
kani::assert(
    state.is_ok(),
    "step_state should be readable after for_each_next",
);
```

Reading a step state being "ok" proves nothing about re-entry behavior. The assertion is too weak to verify the actual bug described in the file header comment (lines 7-11):

> "Bug: When a loop body step completes (Succeeded) and control returns to the loop primitive, the step is still in Succeeded state. The loop primitive needs to transition Succeeded→Pending before re-entering the body, but this transition was missing."

**The bug description says "transition Succeeded→Pending" is missing. The proof does NOT assert that this transition occurs.**

### 5.3 No Negative Case Testing

```rust
Err(EngineError::InternalInvariantViolation { reason }) => {
    kani::assert(
        reason != "invalid_state_transition",
        "for_each_next re-entry should not fail with invalid_state_transition",
    );
}
```

This asserts the error *shouldn't* be "invalid_state_transition", but doesn't assert what error *should* occur (or that no error should occur). The assertion is backward.

---

## VIOLATION 6: SCOTT WLASCHIN DDD BREACHES

### 6.1 No Value Objects

The domain has:
- `StepIdx` — raw `u16` wrapper
- `SlotIdx` — raw `u16` wrapper
- `SlotValue` — untyped enum

But **no**:
- `BodyStep(StepIdx)` — typed step role
- `DoneStep(StepIdx)` — typed step role
- `IteratorSlot(SlotIdx)` — typed slot role
- `AccumulatorSlot(SlotIdx)` — typed slot role
- `CollectorSlot(SlotIdx)` — typed slot role

All roles are implicit in position conventions (e.g., slot 0 is always iterator, slot 1 is accumulator). This is primitive obsession — roles encoded as conventions rather than types.

### 6.2 No Domain Services

The harnesses test standalone functions:
- `for_each_next(...)`
- `reduce_next(...)`
- `collect_next(...)`
- `collect_page(...)`
- `repeat_attempt(...)`
- `repeat_check(...)`

These should be methods on a `LoopPrimitive` trait or similar domain service, but they're tested as free functions with no common interface.

### 6.3 No Aggregates

The concept "a loop with its body step, done step, and slot bindings" is not modeled as an aggregate. The harness manually manages `RunFrame`, `ValueStore`, and state as separate concerns rather than as a cohesive unit.

### 6.4 No Railway-Oriented Error Handling Model

Errors are matched as:
```rust
Err(EngineError::InternalInvariantViolation { reason }) => { ... }
```

No distinction between:
- Expected re-entry states (should transition gracefully)
- Unexpected states (should produce specific errors)
- Infrastructure errors (database full, slot not found)

The error lattice is flat, not hierarchical.

---

## VIOLATION 7: TESTING INFRASTRUCTURE ABSENCE

### 7.1 No Harness Builder Pattern

Every harness manually:
1. Creates `RunFrame` with `fresh_frame()`
2. Creates `ValueStore`
3. Optionally creates `CollectStates`
4. Sets up slot data with `list_in_slot` or raw writes
5. Sets step state with 27-line match block
6. Calls primitive
7. Matches result

**No `ReentryHarness::new(body_state, primitive_type)` builder exists.**

### 7.2 No Shared Verification Trait

```rust
trait ReentryProof {
    fn setup(run: &mut RunFrame, store: &mut ValueStore);
    fn trigger_reentry(run: &mut RunFrame, store: &mut ValueStore) -> Result<EngineSignal, EngineError>;
    fn verify_invariant(state: StepState) -> bool;
}
```

This trait does not exist. All verification is inline and duplicated.

### 7.3 No Property-Based State Generator

`step_state_from_u8` exists (lines 46-57) but:
- Is never called
- Has no `kani::Arbitrary` implementation
- Doesn't use `kani::any()` pattern properly

The state space exploration is manual and inconsistent across harnesses.

---

## REQUIRED REFACTORING

### Phase 1: Extract State Transition Logic (MANDATORY)

```rust
// reentry_state.rs — NEW FILE (~50 lines)
pub trait StepStateTransitions {
    fn apply_state(&mut self, state: StepState) -> Result<(), TransitionError>;
}

impl StepStateTransitions for RunFrame {
    fn apply_state(&mut self, state: StepState) -> Result<(), TransitionError> {
        match state {
            StepState::Pending   => self.mark_pending(body_step),
            // ... rest of match
        }
    }
}
```

### Phase 2: Create Value Object Wrappers (MANDATORY)

```rust
// domain/primitives.rs — NEW FILE
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyStep(StepIdx);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoneStep(StepIdx);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IteratorSlot(SlotIdx);
// ... etc
```

### Phase 3: Build Harness Framework (MANDATORY)

```rust
// reentry_harness.rs — NEW FILE (~100 lines)
pub struct ReentryHarness<P: LoopPrimitive> {
    run: RunFrame,
    store: ValueStore,
    states: P::AuxState,
    body_step: BodyStep,
    done_step: DoneStep,
}

impl<P: LoopPrimitive> ReentryHarness<P> {
    pub fn with_body_state(self, state: StepState) -> Self { ... }
    pub fn verify_reentry(self) -> Result<EngineSignal, EngineError> { ... }
}
```

### Phase 4: Shrink File to ≤300 Lines

| Original | After Refactor | Reduction |
|----------|---------------|-----------|
| 592 lines | ~280 lines | 53% |

---

## ARCHITECTURAL HEALTH SCORE

| Category | Score | Status |
|----------|-------|--------|
| Line Count | 0/100 | **FAIL** |
| Primitive Obsession | 5/100 | **CRITICAL** |
| DRY Compliance | 0/100 | **FAIL** |
| DDD Cohesion | 10/100 | **FAIL** |
| Verification Quality | 15/100 | **FAIL** |
| **OVERALL** | **6/100** | **UNACCEPTABLE** |

---

## MANDATORY ACTIONS

1. **IMMEDIATELY** break into 3+ files: `reentry_state.rs`, `reentry_harness.rs`, `reentry_proofs.rs`
2. **ELIMINATE** all 6 copies of StepState match block — replace with `StepStateTransitions` trait
3. **CREATE** domain value objects for step roles and slot roles
4. **IMPLEMENT** `kani::Arbitrary` for `StepState` using `step_state_from_u8`
5. **STRENGTHEN** assertions — verify Succeeded→Pending transition actually occurs, don't just check readability
6. **ELIMINATE** dead `step_state_from_u8` function or wire it into the harness generator

---

**REPORT GENERATED:** 2026-05-29
**ENFORCER:** architectural-drift agent
**STATUS:** ❌ RED — VIOLATIONS EXCEED THRESHOLD
