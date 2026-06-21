# RA-011: `find_timer_event` and `build_timer_from_event` carry a redundant `StepIdx` round-trip and a re-check that is guaranteed to pass

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_runtime/src/runtime/runtime_recovery.rs:89-140`
- **Confidence**: confirmed

## Description

`find_timer_event` returns `Option<(&JournalEvent, StepIdx)>` where the `StepIdx` is the same `pc` that was passed in. The downstream consumer `build_timer_from_event` then re-matches the event variant and adds a `if pc == *s` guard that can never fail (because the upstream `event_matches_step` filter already enforced it). The plumbing adds no information and the redundant guard obscures the actual logic.

## Evidence

```rust
fn find_timer_event(
    events: &[vb_storage::JournalEvent],
    pc: StepIdx,
) -> Option<(&vb_storage::JournalEvent, StepIdx)> {
    events
        .iter()
        .rev()
        .find(|ev| Self::event_matches_step(ev, pc))
        .map(|ev| (ev, pc))           // <-- returns input pc unchanged
}
```

```rust
fn build_timer_from_event(
    event: Option<(&vb_storage::JournalEvent, StepIdx)>,
) -> Option<PendingTimer> {
    event.and_then(|(ev, pc)| match ev {
        vb_storage::JournalEvent::WaitScheduledEvent { step: s, deadline_ms, .. }
            if pc == *s => Some(Self::make_timer(*s, PendingTimerKind::Wait, *deadline_ms)),
        vb_storage::JournalEvent::AskScheduledEvent { step: s, deadline_ms, .. }
            if pc == *s => Some(Self::make_timer(*s, PendingTimerKind::Ask, *deadline_ms)),
        _ => None,
    })
}
```

`event_matches_step` (line 100-106) already returns `false` for any event whose `step != pc` or that is not `WaitScheduledEvent` / `AskScheduledEvent`. So inside `build_timer_from_event`, the `if pc == *s` guard is structurally unreachable in the failing direction and the `_ => None` arm is unreachable in any direction (the `find` already filtered non-matching events).

## Adversarial Check

One could argue the guards are defense-in-depth: if a future maintainer changes `event_matches_step` to return true for additional variants, the guard prevents misclassification. But the same maintainer would also see the dead `_ => None` arm and assume the function is exhaustive. A cleaner defense is a single match that returns `Option<PendingTimer>` directly from the slice, with no intermediate tuple. Functional-rust prefers one pass.

## Suggested Fix

Collapse both helpers into:

```rust
fn timer_from_events(events: &[vb_storage::JournalEvent], pc: StepIdx) -> Option<PendingTimer> {
    events.iter().rev().find_map(|ev| match ev {
        vb_storage::JournalEvent::WaitScheduledEvent { step, deadline_ms, .. } if *step == pc => {
            Some(Self::make_timer(*step, PendingTimerKind::Wait, *deadline_ms))
        }
        vb_storage::JournalEvent::AskScheduledEvent { step, deadline_ms, .. } if *step == pc => {
            Some(Self::make_timer(*step, PendingTimerKind::Ask, *deadline_ms))
        }
        _ => None,
    })
}
```

One function, one pass, no round-tripped tuple, no unreachable arms.
