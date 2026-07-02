#![forbid(unsafe_code)]
//! Attempt-filter proof helpers for journal replay.

use crate::JournalEvent;
use vb_core::StepIdx;

#[must_use]
pub(crate) fn compute_max_attempt(events: &[JournalEvent]) -> u16 {
    let mut max_attempt = 1u16;
    for event in events {
        if let Some(attempt) = event.attempt().filter(|&a| a > max_attempt) {
            max_attempt = attempt;
        }
    }
    max_attempt
}

#[must_use]
pub const fn replay_attempt_or_default(attempt: Option<u16>) -> u16 {
    match attempt {
        Some(value) => value,
        None => 1,
    }
}

#[must_use]
pub const fn replay_attempt_is_current(attempt: Option<u16>, max_attempt: u16) -> bool {
    replay_attempt_or_default(attempt) >= max_attempt
}

#[must_use]
pub const fn replay_attempt_is_stale(attempt: Option<u16>, max_attempt: u16) -> bool {
    replay_attempt_or_default(attempt) < max_attempt
}

#[must_use]
pub const fn replay_event_has_state_effect(event: &JournalEvent) -> bool {
    matches!(
        event,
        JournalEvent::StepStarted { .. }
            | JournalEvent::ActionScheduled { .. }
            | JournalEvent::ActionCompletedEvent { .. }
            | JournalEvent::ActionFailedEvent { .. }
            | JournalEvent::SlotWrittenEvent { .. }
            | JournalEvent::AskTimedOutEvent { .. }
    )
}

#[must_use]
pub fn replay_event_is_stale_state_effect(event: &JournalEvent, max_attempt: u16) -> bool {
    replay_event_has_state_effect(event) && replay_attempt_is_stale(event.attempt(), max_attempt)
}

#[must_use]
pub const fn replay_step_order_diverges(previous: Option<StepIdx>, current: StepIdx) -> bool {
    match previous {
        Some(step) => current.get() < step.get(),
        None => false,
    }
}
