#![forbid(unsafe_code)]
//! Action ABI expectation checks for replay recovery.

use crate::JournalEvent;
use crate::recovery::types::{RecoveryError, RecoveryResult};
use vb_core::{ActionId, StepIdx, WorkflowDigest};

pub(super) fn validate_action_abi_expectations(
    events: &[JournalEvent],
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RecoveryResult<()> {
    if expected_action_abi_digests.is_empty() {
        return Ok(());
    }

    let Some(action) = events.iter().filter_map(event_action_id).find(|action| {
        !expected_action_abi_digests
            .iter()
            .any(|(expected_action, _)| expected_action == action)
    }) else {
        return Ok(());
    };

    Err(RecoveryError::ReplayDivergence {
        step: StepIdx::ZERO,
        detail: format!("action {action:?} missing action ABI digest evidence"),
    })
}

fn event_action_id(event: &JournalEvent) -> Option<ActionId> {
    match event {
        JournalEvent::ActionScheduled { action, .. }
        | JournalEvent::ActionCompletedEvent { action, .. }
        | JournalEvent::ActionFailedEvent { action, .. } => Some(*action),
        JournalEvent::ActionScheduledTicket { ticket, .. }
        | JournalEvent::ActionCompletedEnvelope { ticket, .. } => Some(ticket.action),
        _ => None,
    }
}
