# SJ-005: `derive_lifecycle_state_from_events` wildcard silently maps new events to `Active`

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_storage/src/journal/incident/lifecycle.rs:61`
- **Confidence**: confirmed

## Description

`derive_lifecycle_state_from_events` reduces the run state to "the state
implied by the last event". Its `match` ends with a `_ => LifecycleState::Active`
wildcard arm, so any future `JournalEvent` variant that does not yet have an
explicit arm will be reported as `Active` to operators (`inspect`,
`recover_all_incomplete_runs`).

## Evidence

```rust
pub fn derive_lifecycle_state_from_events(
    events: &[crate::events::JournalEvent],
) -> LifecycleState {
    events
        .last()
        .map(|e| match e {
            JournalEvent::RunCancelled { .. } => LifecycleState::Cancelled,
            JournalEvent::RunResumed { .. } => LifecycleState::Active,
            JournalEvent::RunRetried { .. } => LifecycleState::Active,
            JournalEvent::RunAnswered { .. } => LifecycleState::Completed,
            JournalEvent::RunFinished { .. } => LifecycleState::Completed,
            JournalEvent::RunFailedEvent { .. } => LifecycleState::Failed,
            JournalEvent::RunAccepted { .. } => LifecycleState::Active,
            JournalEvent::RunAdmission { .. } => LifecycleState::Active,
            JournalEvent::StepStarted { .. } => LifecycleState::Active,
            JournalEvent::StepSucceeded { .. } => LifecycleState::Active,
            JournalEvent::ActionScheduled { .. } => LifecycleState::Active,
            JournalEvent::SlotWrittenEvent { .. } => LifecycleState::Active,
            JournalEvent::ActionCompletedEvent { .. } => LifecycleState::Active,
            JournalEvent::ActionFailedEvent { .. } => LifecycleState::Failed,
            JournalEvent::WaitScheduledEvent { .. } => LifecycleState::WaitingAnswer,
            JournalEvent::AskScheduledEvent { .. } => LifecycleState::WaitingAnswer,
            JournalEvent::AskAnsweredEvent { .. } => LifecycleState::WaitingAnswer,
            JournalEvent::RetryScheduledEvent { .. } => LifecycleState::Active,
            _ => LifecycleState::Active,                    // <-- silent default
        })
        .unwrap_or(LifecycleState::Pending)
}
```

The function is annotated `#[must_use]` only — there is no
`#[non_exhaustive]` awareness and no compiler-enforced exhaustiveness. The
event enum is declared `#[non_exhaustive]` (see `events.rs`), so adding a new
variant externally is permitted and will silently flow into the wildcard.

## Adversarial Check

A counter-argument is "the wildcard exists because `JournalEvent` is
non-exhaustive and we cannot enumerate future variants." But the correct
defense is `unimplemented!()` is forbidden, so the codebase must either (1)
return an explicit `Unknown` lifecycle state that fails closed, or (2) accept
that any new variant is by definition non-terminal and treat `Active` as the
explicit safe default. The current code does neither — it pretends to handle
the variant by silently mapping to `Active`, which would cause
`recover_all_incomplete_runs` to omit a run that is actually in a new
terminal state (because `Active` is treated as "not incomplete"). This is a
correctness hole that scales with every new event variant.

## Suggested Fix

Force exhaustive matching by removing the wildcard and letting the compiler
flag new variants, or introduce an explicit `LifecycleState::Unknown` and
have callers decide whether to fail closed. At minimum, add a comment that
names the variants the wildcard is supposed to absorb (today there are none —
the wildcard is purely defensive against future additions).
