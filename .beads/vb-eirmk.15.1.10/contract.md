# vb-eirmk.15.1.10 — Frame Pool Reinitialize Error Must Not Drop Popped Frame

## Problem
`FramePool::take` pops a frame from the pool, then calls `reinitialize(...)`. If `reinitialize` errors, the `?` propagates the error but **drops the popped frame**. The pool loses one pre-allocated frame on every reinitialize failure.

## Fix
Replace the `?` after `reinitialize` with explicit `if let Err` that pushes the frame back before returning the error:

```rust
if let Err(error) = frame.reinitialize(run_id, first_step, self.step_count, self.slot_count) {
    self.frames.push(frame);
    return Err(error);
}
```

The pop + `AllocationFailed` check stays first (lines 59-62). An exhausted pool still returns `AllocationFailed` before any PC validation. No pre-validation guard is added.

## Invariants
- No allocation, clone, or new API in the fix.
- No changed release behavior.
- `Vec::push` restores the popped slot without allocation (capacity retained).
- Function `take` remains ≤ 25 logical lines.
- No unsafe, suppressions, or new error types.

## Verification
Run existing `vb_runtime` frame_pool regression suite. No new tests added here (another agent writes independent public API tests).
