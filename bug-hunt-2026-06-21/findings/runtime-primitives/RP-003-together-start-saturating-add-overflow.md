# RP-003: `together_start` saturating-add check lets through `parallel_in_flight` overflow

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_runtime/src/primitives/together.rs:32-37`
- **Confidence**: confirmed

## Description

`together_start` guards the parallel-in-flight increment with `current.saturating_add(count) > max`. If `current + count` overflows `u16` while `max == u16::MAX`, the saturating add yields `u16::MAX`, the comparison `u16::MAX > u16::MAX` is false, and execution falls through to `run.add_parallel_in_flight(count)` — which then fails internally with `checked_add` and surfaces as `EngineError::InternalInvariantViolation` rather than the intended `ParallelLimitExceeded`.

## Evidence

`crates/vb_runtime/src/primitives/together.rs:32-37`:

```rust
let current = run.parallel_in_flight();
let max     = run.max_parallel_in_flight();
if current.saturating_add(count) > max {
    return Err(EngineError::ParallelLimitExceeded { limit: max });
}
run.add_parallel_in_flight(count)?;
```

`crates/vb_core/src/frame/parallel.rs:19-29`:

```rust
pub fn add_parallel_in_flight(&mut self, count: u16) -> CoreResult<()> {
    self.parallel_in_flight = self.parallel_in_flight
        .checked_add(count)
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "parallel_in_flight overflow",
        })?;
    ...
}
```

If a workflow sets `max_parallel_in_flight = u16::MAX` (which `compute_max_parallel_in_flight` will do for a single TogetherStart with `u16::MAX` branches), the saturating guard refuses nothing, `add_parallel_in_flight` then errors with `InternalInvariantViolation`. The error type lies about what happened: the workflow requested a legal-but-large parallelism, the runtime reports an "internal invariant" rather than a parallel-limit rejection.

The same pattern repeats a row later: `add_parallel_in_flight` *also* mutates `max_parallel_in_flight` whenever the running counter exceeds it (parallel.rs:25-27). So `max_parallel_in_flight` is not actually a hard limit — it is a high-water mark that the comparison in `together_start` happens to read before the bump. Any caller that reads `max` then bumps via `add_parallel_in_flight` will see a value that is silently ratcheted upward.

## Adversarial Check

1. *"u16::MAX branches is unrealistic."* — `compute_max_parallel_in_flight` (drive.rs:25-30) explicitly accepts up to `u16::MAX` branches and propagates that as the limit; nothing rejects it. A compiled workflow with 65 535 branches is permitted by the type system.
2. *"The error is still caught."* — Yes, but as `InternalInvariantViolation`, which is a misdiagnosis that surfaces in operator runbooks as a runtime bug rather than a workflow-rejection. Worse, `max_parallel_in_flight` has already been mutated by `set_max_parallel_in_flight` in `initialize_drive`, so the invariant violation happens after the limit was advertised as legal.
3. *"This is the same pattern as the bounded queue."* — The bounded queue uses `checked_mul`/`checked_div` for its threshold (queue.rs:150-158); it does not mix saturating and checked arithmetic on the same value.

Severity is Medium: no panic, but the error classification is wrong and operators will chase fictional internal corruption instead of a workflow-acceptance failure.

## Suggested Fix

Use checked arithmetic everywhere and surface the right error:

```rust
let next = current.checked_add(count).ok_or(EngineError::ParallelLimitExceeded { limit: max })?;
if next > max {
    return Err(EngineError::ParallelLimitExceeded { limit: max });
}
run.add_parallel_in_flight(count)?;
```

Separately, decide whether `max_parallel_in_flight` is a *cap* or a *high-water mark*. Today it is both, which is contradictory: `together_start` enforces it as a cap, while `add_parallel_in_flight` treats it as a high-water mark. Pick one — if it is a cap, `add_parallel_in_flight` must not raise it.
