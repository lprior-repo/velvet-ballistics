#![forbid(unsafe_code)]
#![allow(dead_code)]
//! Action-event observation construction.
//!
//! Splits the action observation logic from `normalize.rs` so the
//! `action_abi_digest` and `capacity` preservation contracts are
//! independently testable and so the helper can be invoked from other
//! recovery code paths (e.g. cross-run diff tooling).
//!
//! [`observe_action_event`] is the top-level dispatcher; each
//! per-variant helper below it is responsible for at most one event
//! variant and stays within the Farley 25-line limit.

use crate::JournalEvent;
use vb_core::{ActionId, StepIdx, Taint, WorkflowDigest};

use super::helpers::taint_tag_value;
use super::types::{
    ActionObservation, ActionOutcomeObservation, ActionStateObservation, DigestObservation,
    DigestSubject, LEGACY_OUTCOME_PLACEHOLDER_DIGEST,
};

/// Construct an action observation from a journal event, when applicable.
///
/// Returns `None` for events that are not action events.
///
/// Preservation contracts:
///
/// - `action_abi_digest` is populated for `ActionScheduledTicket` and
///   `ActionCompletedEnvelope` (the only events that carry the field).
/// - `outcome` is populated for `ActionCompletedEvent` (legacy path)
///   and `ActionCompletedEnvelope` (modern path).
/// - `capacity` is populated for `ActionAbandoned` from the preserved
///   ticket capacity, since the abandonment carries the full ticket.
#[must_use]
pub(crate) fn observe_action_event(event: &JournalEvent) -> Option<ActionObservation> {
    observe_action_event_inner(event)
}

/// Inner match dispatcher. Kept private and separate from the public
/// wrapper so the [`observe_action_event`] signature stays compact and
/// this match can be edited without disturbing the documented
/// preservation contract on the public function.
fn observe_action_event_inner(event: &JournalEvent) -> Option<ActionObservation> {
    use JournalEvent::*;
    match event {
        ActionScheduled {
            step,
            action,
            attempt,
            ..
        } => Some(observe_action_scheduled(*step, *action, *attempt)),
        ActionCompletedEvent {
            step,
            action,
            attempt,
            ..
        } => Some(observe_action_completed_event(*step, *action, *attempt)),
        ActionScheduledTicket {
            ticket,
            action_abi_digest,
            ..
        } => Some(observe_action_scheduled_ticket(
            ticket.step,
            ticket.action,
            ticket.attempt,
            *action_abi_digest,
        )),
        ActionCompletedEnvelope {
            ticket,
            outcome,
            taint,
            value_digest,
            action_abi_digest,
            ..
        } => Some(observe_action_completed_envelope(
            ticket.step,
            ticket.action,
            ticket.attempt,
            *outcome,
            taint_tag_value(*taint),
            *value_digest,
            *action_abi_digest,
        )),
        ActionFailedEvent {
            step,
            action,
            attempt,
            ..
        } => Some(observe_action_failed_event(*step, *action, *attempt)),
        ActionAbandoned { ticket, .. } => Some(observe_action_abandoned(
            ticket.step,
            ticket.action,
            ticket.attempt,
            ticket.capacity,
        )),
        _ => None,
    }
}

/// Build the observation for a legacy scheduled action event.
fn observe_action_scheduled(step: StepIdx, action: ActionId, attempt: u16) -> ActionObservation {
    ActionObservation {
        step,
        action,
        attempt,
        state: ActionStateObservation::Scheduled,
        action_abi_digest: None,
        capacity: None,
        outcome: None,
    }
}

/// Build the observation for a legacy completed action event.
///
/// Legacy completion carries no `value_digest`, so the outcome
/// canonical encoding uses [`LEGACY_OUTCOME_PLACEHOLDER_DIGEST`] as
/// the fixed placeholder byte pattern.
fn observe_action_completed_event(
    step: StepIdx,
    action: ActionId,
    attempt: u16,
) -> ActionObservation {
    ActionObservation {
        step,
        action,
        attempt,
        state: ActionStateObservation::Completed,
        action_abi_digest: None,
        capacity: None,
        outcome: Some(ActionOutcomeObservation::Ready {
            taint_tag: taint_tag_value(Taint::Clean),
            value_digest: LEGACY_OUTCOME_PLACEHOLDER_DIGEST,
        }),
    }
}

/// Build the observation for a ticket-style scheduled action event.
fn observe_action_scheduled_ticket(
    step: StepIdx,
    action: ActionId,
    attempt: u16,
    action_abi_digest: WorkflowDigest,
) -> ActionObservation {
    ActionObservation {
        step,
        action,
        attempt,
        state: ActionStateObservation::Scheduled,
        action_abi_digest: Some(action_abi_digest_to_observation(action_abi_digest)),
        capacity: None,
        outcome: None,
    }
}

/// Build the observation for a completed-envelope action event.
fn observe_action_completed_envelope(
    step: StepIdx,
    action: ActionId,
    attempt: u16,
    outcome: crate::DurableActionOutcome,
    taint_tag: u8,
    value_digest: [u8; 32],
    action_abi_digest: WorkflowDigest,
) -> ActionObservation {
    let outcome_observation = match outcome {
        crate::DurableActionOutcome::Ready => ActionOutcomeObservation::Ready {
            taint_tag,
            value_digest,
        },
    };
    ActionObservation {
        step,
        action,
        attempt,
        state: ActionStateObservation::Completed,
        action_abi_digest: Some(action_abi_digest_to_observation(action_abi_digest)),
        capacity: None,
        outcome: Some(outcome_observation),
    }
}

/// Build the observation for a failed action event.
fn observe_action_failed_event(step: StepIdx, action: ActionId, attempt: u16) -> ActionObservation {
    ActionObservation {
        step,
        action,
        attempt,
        state: ActionStateObservation::Failed,
        action_abi_digest: None,
        capacity: None,
        outcome: None,
    }
}

/// Build the observation for an abandoned action event.
fn observe_action_abandoned(
    step: StepIdx,
    action: ActionId,
    attempt: u16,
    capacity: u16,
) -> ActionObservation {
    ActionObservation {
        step,
        action,
        attempt,
        state: ActionStateObservation::Abandoned,
        action_abi_digest: None,
        capacity: Some(capacity),
        outcome: None,
    }
}

/// Wrap a `WorkflowDigest` into a `DigestObservation` keyed to the `Action` subject.
fn action_abi_digest_to_observation(action_abi_digest: WorkflowDigest) -> DigestObservation {
    DigestObservation {
        subject: DigestSubject::Action,
        bytes: action_abi_digest.as_bytes(),
    }
}
