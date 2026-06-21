# RA-006: `make_timer` deadline overflow silently fires the timer immediately

- **Severity**: Low
- **Category**: correctness (silent fallback)
- **Location**: `crates/vb_runtime/src/runtime/runtime_recovery.rs:142-151`
- **Confidence**: confirmed

## Description

`make_timer` computes the recovered timer deadline as `Instant::now() + Duration::from_millis(deadline_ms)`. If the addition overflows (a far-future `deadline_ms` whose duration exceeds `Instant::MAX - now`), the fallback is `Instant::now()`, causing the timer to fire immediately on the next tick advance.

## Evidence

```rust
fn make_timer(step: StepIdx, kind: PendingTimerKind, deadline_ms: u64) -> PendingTimer {
    PendingTimer {
        step,
        kind,
        generation: 0,
        deadline: std::time::Instant::now()
            .checked_add(std::time::Duration::from_millis(deadline_ms))
            .unwrap_or_else(std::time::Instant::now),
    }
}
```

`deadline_ms` comes from a serialized `WaitScheduledEvent` / `AskScheduledEvent` and is a `u64` millisecond count, so it can be up to `u64::MAX ms ≈ 584 million years`. Any value above `(~Instant::MAX - now)` (typically a few hundred years on Linux `CLOCK_MONOTONIC`) silently becomes "fire now."

## Adversarial Check

One could argue this is a reasonable "fail safe" because there is no sensible fallback deadline. But "fire immediately" is the *opposite* of safe for a recovered wait/ask timer: a run that was suspended for hours will be resumed at the next tick, possibly violating an external SLA contract that the wait was honoring. The recovery code has the journal context to log the overflow, but it silently swallows the case. At minimum the timer should carry a "deadline overflowed" flag so the tick handler can fail closed with `WaitTimeout` rather than resuming silently.

Also note that `Instant::now()` itself violates determinism: two recover() calls for the same seed produce different absolute deadlines. This is OK for relative scheduling but means `make_timer` is not pure — a functional-rust refactor would take a `now: Instant` parameter.

## Suggested Fix

Return a `Result<PendingTimer, RuntimeError>` from `make_timer` and propagate `InvalidRecoveryHydration` on overflow, or cap `deadline_ms` to a sane upper bound (e.g. `u32::MAX ms ≈ 50 days`) and reject recovery for runs with longer deadlines rather than silently firing them.
