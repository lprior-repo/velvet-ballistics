# SR-007: `pending_actions_from_events` does not remove failed actions from the pending set

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_storage/src/recovery/replay/summary/slots/pending.rs:36`
- **Confidence**: confirmed

## Description

`pending_actions_from_events` is documented as returning "the set of actions
that were scheduled but not completed". Its `match` handles
`ActionScheduled`/`ActionScheduledTicket` (insert) and
`ActionCompletedEvent`/`ActionCompletedEnvelope` (remove), but
`ActionFailedEvent` falls through to the `_ => {}` wildcard. A failed action
is therefore left in the pending set forever, contradicting the
accumulator's own behavior in `frame_seed/action_records/mod.rs:113-124`
(`record_action_failed` removes the action from `pending_actions`).

## Evidence

```rust
fn recover_pending_actions_from_events_inner(
    events: &[JournalEvent],
) -> HashSet<(ActionId, StepIdx)> {
    let mut pending: HashSet<(ActionId, StepIdx)> = HashSet::new();

    for event in events {
        match event {
            JournalEvent::ActionScheduled { step, action, .. } => {
                pending.insert((*action, *step));
            }
            JournalEvent::ActionScheduledTicket { ticket, .. } => {
                pending.insert((ticket.action, ticket.step));
            }
            JournalEvent::ActionCompletedEvent { step, action, .. } => {
                pending.remove(&(*action, *step));
            }
            JournalEvent::ActionCompletedEnvelope { ticket, .. } => {
                pending.remove(&(ticket.action, ticket.step));
            }
            // All other events are irrelevant for pending actions tracking
            _ => {}
        }
    }

    pending
}
```

Compare `frame_seed/action_records/mod.rs:113-124` (the accumulator's
equivalent):
```rust
pub(super) fn record_action_failed(
    mut self,
    action: ActionId,
    step: StepIdx,
) -> RecoveryResult<Self> {
    if self.action_tracker.is_resolved(action, step) {
        return Err(RecoveryError::NonIdempotentActionBlocked { action, step });
    }
    self.action_tracker.mark_failed(action, step);
    self.pending_actions.remove(&(action, step));   // <-- accumulator removes on failure
    Ok(self)
}
```

So the accumulator and the standalone helper disagree about whether a
failed action remains pending. Callers that use
`pending_actions_from_events` to drive a "resume list" (e.g. retry planning,
operator dashboards) will propose to resume actions that already failed,
which will then re-execute non-idempotent side effects.

## Adversarial Check

A reading of the docstring ("scheduled but not completed") could be read
literally as "any schedule event without a corresponding completed event"
— in which case failed actions stay pending. But that reading contradicts
the accumulator's behavior in the same module, and it produces a pending set
that no consumer actually wants: there is no recovery action that "resume a
failed action" can safely perform. The `ActionReplayTracker` itself
(`recovery/types/replay.rs:230`) defines `is_resolved` as
"completed OR failed", which is the correct semantics for "should not be
re-executed". The standalone function should match.

## Suggested Fix

Add an explicit arm:
```rust
JournalEvent::ActionFailedEvent { step, action, .. } => {
    pending.remove(&(*action, *step));
}
```
Or, better, refactor to share the pending-tracking logic with the
accumulator so the two paths cannot diverge again.
