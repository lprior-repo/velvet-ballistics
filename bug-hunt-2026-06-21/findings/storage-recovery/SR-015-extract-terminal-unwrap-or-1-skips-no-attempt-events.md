# SR-015: `extract_terminal` quietly skips terminal events without an `attempt` field

- **Severity**: Low
- **Category**: bug
- **Location**: `crates/vb_storage/src/recovery/replay/terminal.rs:23`
- **Confidence**: confirmed

## Description

`extract_terminal` finds the latest terminal event whose
`event.attempt().unwrap_or(1)` equals the run's `max_attempt`. The
`unwrap_or(1)` fallback means any future terminal event variant that does
not carry an `attempt` field is silently skipped when `max_attempt > 1`,
making the run look non-terminal to callers like
`recover_all_incomplete_runs`.

## Evidence

```rust
pub fn extract_terminal(events: &[JournalEvent]) -> Option<&JournalEvent> {
    let max_attempt = compute_max_attempt(events);
    events
        .iter()
        .rev()
        .find(|event| is_terminal_event(event) && event.attempt().unwrap_or(1) == max_attempt)
}
```

Today all three terminal variants (`RunFinished`, `RunCancelled`,
`RunFailedEvent`) carry `attempt`, so the fallback never fires. But the
`JournalEvent` enum is `#[non_exhaustive]` and any new terminal variant
added without an `attempt` field will be silently dropped here whenever the
run has been retried (max_attempt > 1). That variant would then be missed
by `recover_all_incomplete_runs` (which uses `extract_terminal` to decide
incompleteness), so the run would be re-enqueued for recovery forever.

## Adversarial Check

A counter-argument: today the code is correct because all three terminal
variants have attempt. The finding is therefore purely defensive. That is
true, but the wildcard `unwrap_or(1)` is a silent failure mode for future
variants — exactly the kind of trap a non-exhaustive enum is supposed to
surface. Forcing `event.attempt().ok_or(...)` would let the compiler
participate in catching new variants.

## Suggested Fix

Either:

1. Make the fallback explicit and noisy: `event.attempt().unwrap_or(1)` →
   return `None` with a `tracing::warn!` when a terminal event has no
   attempt field.
2. Refactor `JournalEvent` to provide a `terminal_attempt()` method that
   returns `Option<u16>` for terminal events only, forcing callers to
   handle the missing case explicitly.

At minimum, change the `1` to a named constant `CURRENT_ATTEMPT_DEFAULT`
with a comment explaining the assumption, so a future variant author has a
chance to notice.
