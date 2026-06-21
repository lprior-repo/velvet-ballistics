# RS-110-life: Ask timer timeout advances state without a resolution journal event

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:102`
- **Confidence**: confirmed

## Description

Timer firing advances the suspended state for both wait and ask timers, but only wait timers append a resolution event. Ask timeouts have no equivalent durable `AskTimedOut` or resolved event before the run is driven forward.

## Evidence

The state is advanced before the timer kind is matched:

```rust
crate::shard::helpers::advance_after_timer_fire(&mut state, timer)?;
match timer.kind {
    PendingTimerKind::Wait => {
        self.append_journal_event(RuntimeJournalEvent::WaitResolved {
            run,
            step: timer.step,
        })?;
    }
    PendingTimerKind::Ask => {}
}
```

For `PendingTimerKind::Ask`, the timeout path intentionally records nothing at this point.

## Adversarial Check

This is not covered by `AskScheduled`: scheduling records that the ask began, not that its timeout authority fired. Even if later evidence records subsequent progress, there is a crash window after `advance_after_timer_fire` and before later durable events, and replay cannot distinguish an unanswered pending ask from an ask whose timeout already fired.

## Suggested Fix

Add a durable `AskTimedOut` or generic timer-resolution journal event and append it before applying the timeout advancement, or make `advance_after_timer_fire` produce evidence that is flushed transactionally before state mutation becomes visible.
