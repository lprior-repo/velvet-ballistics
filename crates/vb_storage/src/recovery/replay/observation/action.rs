#![forbid(unsafe_code)]
//! Action event observation helpers.
//!
//! Converts action-related `JournalEvent` variants into `ActionObservation`
//! entries, preserving `action_abi_digest` for ticket/envelope events and
//! `capacity` for abandoned actions.

use super::types::{ActionObservation, JournalObservation};
use crate::JournalEvent;

/// Observe an action event and push the resulting observation(s) into the
/// provided vector.
pub fn observe_action_event(event: &JournalEvent, observations: &mut Vec<JournalObservation>) {
    match event {
        JournalEvent::ActionScheduled {
            step,
            action,
            attempt,
            ..
        } => {
            observations.push(JournalObservation::Action(ActionObservation::Scheduled {
                action: *action,
                step: *step,
                attempt: *attempt,
                action_abi_digest: None,
            }));
        }
        JournalEvent::ActionCompletedEvent {
            step,
            action,
            attempt,
            ..
        } => {
            observations.push(JournalObservation::Action(ActionObservation::Completed {
                action: *action,
                step: *step,
                attempt: *attempt,
            }));
        }
        JournalEvent::ActionScheduledTicket {
            ticket,
            action_abi_digest,
            ..
        } => {
            observations.push(JournalObservation::Action(ActionObservation::Scheduled {
                action: ticket.action,
                step: ticket.step,
                attempt: ticket.attempt,
                action_abi_digest: Some(*action_abi_digest),
            }));
        }
        JournalEvent::ActionCompletedEnvelope { ticket, .. } => {
            observations.push(JournalObservation::Action(ActionObservation::Completed {
                action: ticket.action,
                step: ticket.step,
                attempt: ticket.attempt,
            }));
        }
        JournalEvent::ActionFailedEvent {
            step,
            action,
            attempt,
            ..
        } => {
            observations.push(JournalObservation::Action(ActionObservation::Failed {
                action: *action,
                step: *step,
                attempt: *attempt,
            }));
        }
        JournalEvent::ActionAbandoned { ticket, .. } => {
            observations.push(JournalObservation::Action(ActionObservation::Abandoned {
                action: ticket.action,
                step: ticket.step,
                attempt: ticket.attempt,
                capacity: ticket.capacity,
            }));
        }
        _ => {}
    }
}
