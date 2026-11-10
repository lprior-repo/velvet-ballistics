use super::apply::ApplyOutcome;
use super::check_action_abi_digest_against_expected;
use super::verify::{
    decode_action_envelope_slot, sub_tail_parallel_in_flight, verified_action_envelope_digest,
    verify_action_ticket_event,
};

use crate::JournalEvent;
use crate::recovery::types::{
    ActionReplayEffect, ActionReplayTracker, RecoveryError, RecoveryResult,
};

pub(crate) fn apply_action_scheduled(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
    tracker: &ActionReplayTracker,
) -> RecoveryResult<ApplyOutcome> {
    let JournalEvent::ActionScheduled { action, step, .. } = event else {
        return Ok(ApplyOutcome::NotApplicable);
    };
    if tracker.is_resolved(*action, *step) {
        return Err(RecoveryError::NonIdempotentActionBlocked {
            action: *action,
            step: *step,
        });
    }
    frame
        .add_parallel_in_flight(1)
        .map_err(|_e| RecoveryError::ReplayDivergence {
            step: *step,
            detail: "parallel_in_flight overflow".to_owned(),
        })?;
    Ok(ApplyOutcome::Executed)
}

pub(crate) fn apply_action_scheduled_ticket(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
    tracker: &mut ActionReplayTracker,
    expected_action_abi_digests: &[(vb_core::ActionId, vb_core::WorkflowDigest)],
) -> RecoveryResult<ApplyOutcome> {
    let JournalEvent::ActionScheduledTicket {
        run,
        ticket,
        input,
        output,
        action_abi_digest,
        ..
    } = event
    else {
        return Ok(ApplyOutcome::NotApplicable);
    };
    verify_action_ticket_event(*run, *ticket)?;
    check_action_abi_digest_against_expected(
        ticket.action,
        *action_abi_digest,
        expected_action_abi_digests,
    )?;
    let effect =
        tracker.mark_scheduled_ticket_effect(*ticket, *input, *output, *action_abi_digest)?;
    if effect == ActionReplayEffect::Duplicate {
        return Ok(ApplyOutcome::Skipped);
    }
    frame
        .add_parallel_in_flight(1)
        .map_err(|_e| RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: "parallel_in_flight overflow".to_owned(),
        })?;
    Ok(ApplyOutcome::Executed)
}

pub(crate) fn apply_action_completed_event(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<ApplyOutcome> {
    let JournalEvent::ActionCompletedEvent { action, step, .. } = event else {
        return Ok(ApplyOutcome::NotApplicable);
    };
    if tracker.is_resolved(*action, *step) {
        return Err(RecoveryError::NonIdempotentActionBlocked {
            action: *action,
            step: *step,
        });
    }
    tracker.mark_completed(*action, *step);
    sub_tail_parallel_in_flight(frame, *step)?;
    Ok(ApplyOutcome::Executed)
}

pub(crate) fn apply_action_completed_envelope(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
    tracker: &mut ActionReplayTracker,
    expected_action_abi_digests: &[(vb_core::ActionId, vb_core::WorkflowDigest)],
) -> RecoveryResult<ApplyOutcome> {
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
        return Ok(ApplyOutcome::NotApplicable);
    };
    let verified = verified_action_envelope_digest(
        *run,
        *ticket,
        *outcome,
        value,
        *encoded_len,
        *value_digest,
    )?;
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
        verified,
        *action_abi_digest,
    )?;
    if effect == ActionReplayEffect::Duplicate {
        return Ok(ApplyOutcome::Skipped);
    }
    apply_envelope_slot_and_step(frame, *ticket, *output, value, *taint)?;
    Ok(ApplyOutcome::Executed)
}

pub(crate) fn apply_envelope_slot_and_step(
    frame: &mut vb_core::RunFrame,
    ticket: vb_core::ActionTicket,
    output: vb_core::SlotIdx,
    value: &[u8],
    taint: vb_core::Taint,
) -> RecoveryResult<()> {
    let slot_value = decode_action_envelope_slot(ticket, output, value)?;
    frame
        .write_slot_with_taint(output, slot_value, taint)
        .map_err(|_| RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: "slot write out of bounds".to_owned(),
        })?;
    frame
        .mark_succeeded(ticket.step)
        .map_err(|_| RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: "mark_succeeded failed".to_owned(),
        })?;
    sub_tail_parallel_in_flight(frame, ticket.step)
}

pub(crate) fn apply_action_failed(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<ApplyOutcome> {
    let JournalEvent::ActionFailedEvent { action, step, .. } = event else {
        return Ok(ApplyOutcome::NotApplicable);
    };
    if tracker.is_resolved(*action, *step) {
        return Err(RecoveryError::NonIdempotentActionBlocked {
            action: *action,
            step: *step,
        });
    }
    tracker.mark_failed(*action, *step);
    sub_tail_parallel_in_flight(frame, *step)?;
    Ok(ApplyOutcome::Executed)
}
