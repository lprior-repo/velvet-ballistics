# Contract: for_each Body Re-entry State Reset

## Bug Statement

**VB-Y4PA**: `for_each_next` jumps to `body` step without resetting body state from `Succeeded` to `Pending`. State machine rejects `Succeeded → Running`.

### Root Cause Analysis

1. `for_each_next(run, store, iterator_slot, body, done, output)` calls `jump_to(run, body)` which returns `EngineSignal::Continue`
2. `mark_step_after_signal` (helpers.rs:21) maps `RuntimeSignal::Continue` → `run.mark_succeeded(step)` on the current step (not the target body)
3. After body completes, its step is in `Succeeded` state
4. On second iteration, `for_each_next` calls `jump_to(run, body)` again
5. Engine scheduler picks up body step and attempts `Succeeded → Running` transition
6. `VALID_TRANSITIONS` in `step_state.rs` does NOT contain `(Succeeded, Running)` → `validate_transition` returns `Err("invalid_state_transition")`

### Affected State Machine Transitions

```rust
// VALID_TRANSITIONS — MISSING entry:
(StepState::Succeeded, StepState::Pending), // ← REQUIRED for loop re-entry
```

### Affected Primitives (Same Bug Pattern)

| Primitive | Function | Jump to Body | Resets Body State? |
|---|---|---|---|
| `for_each` | `for_each_next` | `jump_to(run, body)` | **NO** ❌ |
| `reduce` | `reduce_next` | `jump_to(run, body)` | **NO** ❌ |
| `collect` | `collect_next` | `jump_to(run, body)` | **NO** ❌ |
| `collect` | `collect_page` | `jump_to(run, body)` | **NO** ❌ |
| `repeat` | `repeat_attempt` | `jump_to(run, body)` | **NO** ❌ |
| `repeat` | `repeat_check` (when looping) | `jump_to(run, body_entry)` | **NO** ❌ |

---

## Contract: Body Re-entry Protocol

### INVARIANT: Loop Body State Reset

```
FORALL step: StepIdx, primitive: {for_each, reduce, collect, repeat}
  WHEN primitive jumps to body_step via jump_to(run, body_step)
  THEN ensure body_step.state == Pending BEFORE set_pc(body_step)
```

### Formal Specification

```
BodyReentryPrecondition(body_step, run):
  run.step_state(body_step) ∈ {Pending, Waiting, Asking}
  OR run.step_state(body_step) == Succeeded
     AND run.mark_pending(body_step) == Ok(())

BodyReentryPostcondition(body_step, run):
  run.step_state(body_step) == Pending
  AND run.pc() == body_step
  AND run.executed() == old(run.executed()) + 1
```

### State Machine Extension Required

```rust
// crates/vb_proof_kernels/src/step_state.rs
// Add to VALID_TRANSITIONS:
(StepState::Succeeded, StepState::Pending), // enables loop body reset
```

### Frame API Extension Required

```rust
// crates/vb_core/src/frame.rs
/// Marks a step as Pending (enables loop body re-entry from Succeeded).
pub fn mark_pending(&mut self, step: StepIdx) -> CoreResult<()> {
    self.write_step_state(step, StepState::Pending)
}
```

### Runtime Helper Addition

```rust
// crates/vb_runtime/src/primitives/helpers.rs
/// Jumps to body step AND resets it to Pending if it was Succeeded.
/// This is required for loop primitives where body may be re-entered
/// after a previous Succeeded completion.
pub(crate) fn jump_to_body(
    run: &mut RunFrame,
    body: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    // Reset body from Succeeded → Pending to allow re-entry
    if let Ok(StepState::Succeeded) = run.step_state(body) {
        run.mark_pending(body)?;
    }
    run.set_pc(body)?;
    run.increment_executed()?;
    Ok(vb_core::EngineSignal::Continue)
}
```

---

## Given-When-Then Scenarios

### GWT-1: for_each two-item list body re-entry

```
Given:
  - A for_each primitive over [Item1, Item2]
  - for_each_start ran: body_step is Pending, iterator = [Item1, Item2]
  - Body executed Item1 and returned Continue → body_step marked Succeeded

When:
  - Engine calls for_each_next(iterator_slot=[Item2], body=body_step, done, output)

Then:
  - for_each_next calls jump_to_body(run, body_step)
  - body_step transitions Succeeded → Pending (no error)
  - body_step is set to Running by engine scheduler
  - Body executes Item2
  - for_each_next returns EngineSignal::Continue
```

### GWT-2: for_each body already Pending (no reset needed)

```
Given:
  - A for_each primitive over [Item1]
  - for_each_start ran: body_step is Pending, iterator = [Item1]

When:
  - Engine calls for_each_next(iterator_slot=[], body=body_step, done, output)

Then:
  - for_each_next sees iterator is empty → jumps to done
  - body_step is NOT reset (already Pending from start)
```

### GWT-3: for_each body Succeeded → Running transition is rejected without fix

```
Given:
  - A for_each primitive over [Item1, Item2]
  - Body executed Item1 → body_step is Succeeded
  - WITHOUT the fix: jump_to does NOT reset body_step

When:
  - Engine scheduler picks body_step to run
  - Engine attempts body_step.state: Succeeded → Running

Then:
  - validate_transition returns Err("invalid_state_transition")
  - Engine panics or returns error
```

### GWT-4: repeat body re-entry after attempt

```
Given:
  - A repeat primitive with max_attempts=3
  - repeat_start ran: body_step is Pending, attempt=0
  - Body attempt 1 ran and succeeded → body_step is Succeeded

When:
  - repeat_check increments attempt to 1 (still < max)
  - repeat_check calls jump_to_body(run, body_entry)

Then:
  - body_entry transitions Succeeded → Pending
  - Body attempt 2 can run
```

---

## Audit: Other Primitives for Same Bug

### reduce_next (reduce.rs:56-82)

**Same bug**: `reduce_next` calls `jump_to(run, body)` without resetting body.
- On list [A, B, C]: start binds A, body runs, next binds B, body runs, next binds C, body runs
- After first body run: body is Succeeded
- Second `reduce_next` call would fail with invalid transition

**Fix**: Replace `jump_to(run, body)` with `jump_to_body(run, body)`

### collect_next (collect.rs:496-521)

**Same bug**: `collect_next` calls `jump_to(run, body)` without resetting body.
- On pagination with page_size=2 over [A,B,C,D]: 
  - start: page [A,B], body runs, 
  - next: page [C,D], body runs → second body re-entry fails

**Fix**: Replace `jump_to(run, body)` with `jump_to_body(run, body)`

### collect_page (collect.rs:388-398)

**Same bug**: `collect_page` calls `jump_to(run, body)` without resetting body.
- When a page body completes and loops back to collect_page, body step is Succeeded

**Fix**: Replace `jump_to(run, body)` with `jump_to_body(run, body)`

### repeat_attempt (repeat.rs:78-88)

**Same bug**: `repeat_attempt` calls `jump_to(run, body)` without resetting body.
- After body attempt completes, re-entering attempts would fail

**Fix**: Replace `jump_to(run, body)` with `jump_to_body(run, body)`

### repeat_check (repeat.rs:94-116)

**Same bug when looping**: `repeat_check` calls `jump_to(run, body_entry)` when attempts remain without resetting body.
- After body attempt completes, checking for retry would fail

**Fix**: Replace `jump_to(run, body_entry)` with `jump_to_body(run, body_entry)`

---

## Proof Obligations

See `proof-obligations.planned.jsonl` for exhaustive proof obligations and verification targets.

---

## TLA+ Specification

See `tla-spec.md` for formal TLA+ model of the state machine fix.
