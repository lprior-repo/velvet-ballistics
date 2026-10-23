# vb-8eocj.3 implementation-v11: Narrow owned-continuation rearm for ForEachNext/ReduceNext

## Status: DRAFT (not merged, not pushed)

## Problem

V10 approach (wrapping primitives to return `Ok(signal)` and calling `run.mark_pending(self_step)` inside the primitive after `jump_to_body`) was rejected because:

1. The primitives now return bare `EngineSignal` instead of `Result<EngineSignal, EngineError>`, so `Ok()` wrapping is a type error.
2. More fundamentally, calling `run.mark_pending(self_step)` while the controller is still `Running` (before `finish_drive_step` commits `Succeeded`) creates a reentry ordering problem. The step state transitions Running -> Pending -> Succeeded are interleaved incorrectly.

## Solution

Move the legitimate owner rearm to the correct post-success state commit stage in the drive transition layer (`transitions.rs`), not inside the primitive. The primitive simply calls `jump_to_body` and returns its signal as before. After the normal success/evidence commit, the transition layer checks if the just-completed node is an owning-loop control node (ForEachNext or ReduceNext) that successfully jumped back to its declared body, and only then re-arms the node's own step for the next iteration.

## Files changed

### crates/vb_runtime/src/primitives/for_each.rs

**Reverted V10 changes.** Restored `for_each_next` to simply call and return `jump_to_body(run, body)` directly, matching baseline behavior:

```rust
pub fn for_each_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    // ... iterator advancement logic ...
    jump_to_body(run, body)  // returns EngineSignal directly
}
```

No `self_step` capture, no `mark_pending` call inside the primitive.

### crates/vb_runtime/src/primitives/reduce.rs

**Reverted V10 changes.** Restored `reduce_next` to simply call and return `jump_to_body(run, body)` directly:

```rust
pub fn reduce_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    _accumulator: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    // ... iterator advancement logic ...
    jump_to_body(run, body)  // returns EngineSignal directly
}
```

No `self_step` capture, no `mark_pending` call inside the primitive.

### crates/vb_runtime/src/engine/drive/transitions.rs

**Added narrow owned-continuation rearm.** Two new helper functions and one call site in `finish_drive_step`:

**New: `loop_body_target(node)`** - Extracts the body step index from an owning-loop control node (ForEachNext or ReduceNext), or `None` for non-loop/join/finish nodes:

```rust
pub(super) fn loop_body_target(node: &CompiledNode) -> Option<StepIdx> {
    match node.kind {
        CompiledNodeKind::ForEachNext { body, .. }
        | CompiledNodeKind::ReduceNext { body, .. } => Some(body),
        _ => None,
    }
}
```

**New: `owned_continuation_rearm(run, step)`** - After normal success state and evidence commit, re-arms the ForEachNext/ReduceNext control node for the next iteration when the drive successfully jumped back to its declared body. Two owner checks confirm legitimate ownership:

```rust
pub(super) fn owned_continuation_rearm(
    run: &mut RunFrame,
    step: DriveStep<'_>,
) -> RuntimeEngineResult<()> {
    let Some(body) = loop_body_target(step.node) else {
        return Ok(());
    };
    if run.pc() != body {
        return Ok(());
    }
    if body >= step.pc {
        return Ok(());
    }
    run.mark_pending(step.pc).map_err(RuntimeEngineError::Core)?;
    Ok(())
}
```

Owner checks:
- (a) `run.pc() == body`: The drive successfully jumped to this node's declared body target. Confirms this specific control node owns the current execution position.
- (b) `body < step.pc`: The body step index is less than the captured control step index. Confirms legitimate loop-backward targeting rather than arbitrary step manipulation.

Both checks must pass for the rearm to occur. No generic Succeeded->Running transition; no tests resetting states; no reset of unrelated nodes or future steps.

**Modified: `finish_drive_step`** - Calls `owned_continuation_rearm` after the success path completes state and evidence commit:

```rust
if signal_is_success(signal) {
    mark_step_after_signal(run, step.pc, signal)?;
    evidence
        .push_step_succeeded(step.pc, step.node.output)
        .map_err(RuntimeEngineError::Core)?;
    owned_continuation_rearm(run, step)?;  // NEW: rearm owning-loop control nodes only
}
```

## What this preserves

- `push_loop_span`/`pop-before-compare` evidence semantics untouched.
- Generic transition table untouched.
- ForEachStart, ForEachJoin, ReduceStart, ReduceFinish, Repeat, Together, Collect semantics untouched.
- No suppression of actual errors or evidence gaps.

## State limitation (explicit)

This implementation covers the current one-body canonical subset where each ForEachNext/ReduceNext jumps to exactly one declared body target that is reset to the first body step by `jump_to_body`. Arbitrary multi-body loops (where a ForEachNext could target different bodies based on runtime state) remain a full F1 future enhancement; this is a state limitation, not a coverage claim. The owner check `run.pc() == body` ensures that only the control node whose declared body matches the current execution position is re-armed.

## Tests

NOT RUN. Source edits only. No test suite modifications. Existing integration tests in `crates/vb_runtime/src/primitives/for_each/tests.rs`, `crates/vb_runtime/src/primitives/reduce/tests.rs`, and `crates/workspace_tests/` should validate behavior once root tests are run.

## Verification scope

- Source compiles (pending root tests).
- Type system enforces `EngineSignal` return from primitives (no `Ok()` wrapping).
- State transitions: Running -> Succeeded (committed) -> Pending (rearmed) for owning-loop nodes only.
- Non-loop nodes and join/finish nodes remain Succeeded after success commit.
- Suspension signals (AwaitingWait, AwaitingAsk, AwaitingAction, StepBudgetExhausted) bypass success path entirely; no rearm occurs.
