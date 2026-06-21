#![forbid(unsafe_code)]
//! Pending action tracking from journal events.
//!
//! Provides:
//! - `pending_actions_from_events` — pending action set from journal events

use std::collections::HashSet;

use crate::JournalEvent;
use crate::recovery::RecoveredPendingAction;
use vb_core::{ActionId, StepIdx};

/// Production proof surface: converts the accumulator HashSet into the
/// public-facing `Vec<RecoveredPendingAction>` representation.
fn recovered_pending_actions(
    pending_actions: HashSet<(ActionId, StepIdx)>,
) -> Vec<RecoveredPendingAction> {
    pending_actions
        .into_iter()
        .map(|(action, step)| RecoveredPendingAction { step, action })
        .collect()
}

/// Public accessor for tests and observability.
/// Returns the set of actions that were scheduled but not completed
/// from a sequence of journal events.
///
/// This is a convenience wrapper around the private `recovered_pending_actions`
/// that accepts raw journal events instead of a pre-built accumulator.
#[must_use]
pub fn pending_actions_from_events(events: &[JournalEvent]) -> Vec<RecoveredPendingAction> {
    let accumulator = recover_pending_actions_from_events_inner(events);
    recovered_pending_actions(accumulator)
}

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
            JournalEvent::ActionFailedEvent { step, action, .. } => {
                pending.remove(&(*action, *step));
            }
            // All other events are irrelevant for pending actions tracking
            _ => {}
        }
    }

    pending
}
