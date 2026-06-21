# RS-204-help: Timer fire advances step state before validating the successor PC

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/helpers/timer.rs:24`
- **Confidence**: confirmed

## Description

`advance_after_timer_fire` marks the timer step running and succeeded before it checks that the workflow node has a successor. If the node has no `next`, or if `set_pc` fails, the function returns `InvalidTimerFire` after partially mutating the run frame.

## Evidence

```rust
// timer.rs:24-51
pub fn advance_after_timer_fire(state: &mut RunState, timer: PendingTimer) -> RuntimeResult<()> {
    let Some(node) = state.workflow.node(timer.step) else {
        return Err(RuntimeError::InvalidTimerFire);
    };
    match (timer.kind, &node.kind) { ... }
    state.frame.mark_running(timer.step).map_err(|_| RuntimeError::InvalidTimerFire)?;
    state.frame.mark_succeeded(timer.step).map_err(|_| RuntimeError::InvalidTimerFire)?;
    let Some(next) = node.next else {
        return Err(RuntimeError::InvalidTimerFire);
    };
    state.frame.set_pc(next).map_err(|_| RuntimeError::InvalidTimerFire)?;
    Ok(())
}
```

The fallible `node.next` validation happens after two state mutations. The error return does not undo either mutation.

## Adversarial Check

A valid compiled workflow may normally give waits and asks a successor, but this helper explicitly returns `RuntimeError::InvalidTimerFire` for malformed timer fires. Error paths should fail closed. As written, the caller sees an error while the frame now records the step as succeeded, which can poison later inspection, recovery evidence, or retry logic.

## Suggested Fix

Move all validation before mutation. Resolve and validate `next` before calling `mark_running`, `mark_succeeded`, or `set_pc`. If `set_pc` can fail independently, either validate that PC target first or use a frame API that performs the timer transition atomically.
