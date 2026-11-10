#![forbid(unsafe_code)]

use super::accounting::{add_replay_parallel_in_flight, sub_replay_parallel_in_flight};

use crate::JournalEvent;
use crate::recovery::hydrate_support::{
    verified_action_envelope_digest, verify_action_ticket_event,
};
use crate::recovery::types::{
    ActionReplayEffect, ActionReplayTracker, RecoveryError, RecoveryResult,
};
use vb_core::{ActionId, WorkflowDigest};

pub(crate) fn apply_action_scheduled_ticket_event(
    frame: &mut vb_core::RunFrame,
    tracker: &mut ActionReplayTracker,
    event: &JournalEvent,
    expected_action_abi_digests: &[(vb_core::ActionId, vb_core::WorkflowDigest)],
) -> RecoveryResult<bool> {
    let JournalEvent::ActionScheduledTicket {
        run,
        ticket,
        input,
        output,
        action_abi_digest,
        ..
    } = event
    else {
        return Ok(false);
    };
    verify_action_ticket_event(*run, *ticket)?;
    check_action_abi_digest_against_expected(
        ticket.action,
        *action_abi_digest,
        expected_action_abi_digests,
    )?;
    let effect =
        tracker.mark_scheduled_ticket_effect(*ticket, *input, *output, *action_abi_digest)?;
    if effect == ActionReplayEffect::Apply {
        add_replay_parallel_in_flight(frame, ticket.step)?;
    }
    Ok(effect == ActionReplayEffect::Apply)
}

pub(crate) fn apply_action_completed_envelope_event(
    frame: &mut vb_core::RunFrame,
    tracker: &mut ActionReplayTracker,
    event: &JournalEvent,
    expected_action_abi_digests: &[(vb_core::ActionId, vb_core::WorkflowDigest)],
) -> RecoveryResult<bool> {
    let JournalEvent::ActionCompletedEnvelope {
        run,
        ticket,
        output,
        outcome,
        value,
        encoded_len,
        taint,
        value_digest,
        action_abi_digest,
        ..
    } = event
    else {
        return Ok(false);
    };
    let verified_digest = verified_action_envelope_digest(
        *run,
        *ticket,
        *outcome,
        value,
        *encoded_len,
        *value_digest,
    )?;
    tracker.require_scheduled_ticket(*ticket, *output, *action_abi_digest)?;
    check_action_abi_digest_against_expected(
        ticket.action,
        *action_abi_digest,
        expected_action_abi_digests,
    )?;
    let effect = tracker.mark_completed_envelope_effect(
        *ticket,
        *output,
        *encoded_len,
        *taint,
        verified_digest,
        *action_abi_digest,
    )?;
    if effect == ActionReplayEffect::Apply {
        sub_replay_parallel_in_flight(frame, ticket.step)?;
    }
    Ok(effect == ActionReplayEffect::Apply)
}
// VB-NOORE (wildcard elimination): explicit arms for every
// JournalEvent variant so adding a new variant forces a
// compile-time failure. The dispatcher routes action events
// to dedicated helpers, so this classifier returns Ok(false)
// for any action variant that ever reaches it (a defensive
// belt-and-braces for the dispatcher's `_ =>` arm).
pub(crate) fn classify_metadata_event(event: &JournalEvent) -> RecoveryResult<bool> {
    match event {
        JournalEvent::RunContinuation { .. } => Err(RecoveryError::UnsupportedFrameSeed {
            run: event.run_id(),
            reason: String::from("owned-continuation-needs-authoritative-recovery"),
        }),
        JournalEvent::StepStarted { .. }
        | JournalEvent::StepSucceeded { .. }
        | JournalEvent::StepFailed { .. }
        | JournalEvent::SlotWrittenEvent { .. }
        | JournalEvent::WaitScheduledEvent { .. }
        | JournalEvent::AskScheduledEvent { .. }
        | JournalEvent::AskTimedOutEvent { .. }
        | JournalEvent::WaitResolvedEvent { .. }
        | JournalEvent::AskAnsweredEvent { .. }
        | JournalEvent::RetryScheduledEvent { .. }
        | JournalEvent::PolicyDecisionEvent { .. }
        | JournalEvent::GateDecisionEvent { .. } => Ok(true),
        JournalEvent::GateOpenedEvent { .. } => Ok(true),
        JournalEvent::CompatibilityDecisionEvent { .. }
        | JournalEvent::TenantAdmissionEvent { .. } => Ok(true),
        JournalEvent::ActionScheduled { .. }
        | JournalEvent::ActionScheduledTicket { .. }
        | JournalEvent::ActionCompletedEvent { .. }
        | JournalEvent::ActionCompletedEnvelope { .. }
        | JournalEvent::ActionFailedEvent { .. }
        | JournalEvent::ActionAbandoned { .. }
        | JournalEvent::RunAccepted { .. }
        | JournalEvent::RunAdmission { .. }
        | JournalEvent::RunAdmissionBudget { .. }
        | JournalEvent::RetryCheckpoint { .. }
        | JournalEvent::RunCancelled { .. }
        | JournalEvent::RunKilled { .. }
        | JournalEvent::RunFinished { .. }
        | JournalEvent::RunFailedEvent { .. }
        | JournalEvent::RunResumed { .. }
        | JournalEvent::RunRetried { .. }
        | JournalEvent::RunAnswered { .. } => Ok(false),
    }
}

/// Checks whether the found action ABI digest matches any of the expected digests.
fn check_action_abi_digest_against_expected(
    action_id: ActionId,
    found: WorkflowDigest,
    expected: &[(ActionId, WorkflowDigest)],
) -> RecoveryResult<()> {
    if found == zero_workflow_digest() {
        return Err(RecoveryError::ActionAbiMismatch { action_id });
    }
    for (exp_action_id, exp_digest) in expected {
        if *exp_action_id == action_id {
            if *exp_digest == zero_workflow_digest() || *exp_digest != found {
                return Err(RecoveryError::ActionAbiMismatch {
                    action_id: *exp_action_id,
                });
            }
            return Ok(());
        }
    }
    Err(RecoveryError::ActionAbiMismatch { action_id })
}

const fn zero_workflow_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0; 32])
}
