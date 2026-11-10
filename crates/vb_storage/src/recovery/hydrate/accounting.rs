#![forbid(unsafe_code)]

use super::envelope::{
    apply_action_completed_envelope_event, apply_action_scheduled_ticket_event,
    classify_metadata_event,
};

use crate::JournalEvent;
use crate::recovery::types::{ActionReplayTracker, RecoveryError, RecoveryResult};
use vb_core::RunId;

pub(crate) fn apply_replay_accounting(
    frame: &mut vb_core::RunFrame,
    events: &[JournalEvent],
    run_id: RunId,
    expected_action_abi_digests: &[(vb_core::ActionId, vb_core::WorkflowDigest)],
) -> RecoveryResult<u64> {
    let mut tracker = ActionReplayTracker::new();
    let mut count = 0u64;
    let mut peak = 0u16;
    for event in events {
        if apply_accounting_event(frame, event, &mut tracker, expected_action_abi_digests)? {
            count = increment_replay_count(count, run_id)?;
        }
        if frame.parallel_in_flight() > peak {
            peak = frame.parallel_in_flight();
        }
    }
    frame.set_max_parallel_in_flight(peak);
    Ok(count)
}

fn increment_replay_count(current: u64, run_id: RunId) -> RecoveryResult<u64> {
    current
        .checked_add(1)
        .ok_or(RecoveryError::FrameDimensionOverflow { run: run_id })
}
fn apply_accounting_event(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
    tracker: &mut ActionReplayTracker,
    expected_action_abi_digests: &[(vb_core::ActionId, vb_core::WorkflowDigest)],
) -> RecoveryResult<bool> {
    match event {
        JournalEvent::ActionScheduled { .. }
        | JournalEvent::ActionCompletedEvent { .. }
        | JournalEvent::ActionFailedEvent { .. } => {
            apply_simple_action_event(frame, tracker, event)
        }
        JournalEvent::ActionScheduledTicket { .. } => {
            apply_action_scheduled_ticket_event(frame, tracker, event, expected_action_abi_digests)
        }
        JournalEvent::ActionCompletedEnvelope { .. } => apply_action_completed_envelope_event(
            frame,
            tracker,
            event,
            expected_action_abi_digests,
        ),
        _ => classify_metadata_event(event),
    }
}

fn apply_simple_action_event(
    frame: &mut vb_core::RunFrame,
    tracker: &mut ActionReplayTracker,
    event: &JournalEvent,
) -> RecoveryResult<bool> {
    match event {
        JournalEvent::ActionScheduled { action, step, .. } => {
            reject_resolved_action(tracker, *action, *step)?;
            add_replay_parallel_in_flight(frame, *step).map(|()| true)
        }
        JournalEvent::ActionCompletedEvent { action, step, .. } => {
            reject_resolved_action(tracker, *action, *step)?;
            tracker.mark_completed(*action, *step);
            sub_replay_parallel_in_flight(frame, *step).map(|()| true)
        }
        JournalEvent::ActionFailedEvent { action, step, .. } => {
            reject_resolved_action(tracker, *action, *step)?;
            tracker.mark_failed(*action, *step);
            sub_replay_parallel_in_flight(frame, *step).map(|()| true)
        }
        _ => Ok(false),
    }
}

pub(crate) fn add_replay_parallel_in_flight(
    frame: &mut vb_core::RunFrame,
    step: vb_core::StepIdx,
) -> RecoveryResult<()> {
    frame
        .add_parallel_in_flight(1)
        .map_err(|_| RecoveryError::ReplayDivergence {
            step,
            detail: "parallel_in_flight overflow".to_owned(),
        })
}

pub(crate) fn sub_replay_parallel_in_flight(
    frame: &mut vb_core::RunFrame,
    step: vb_core::StepIdx,
) -> RecoveryResult<()> {
    frame
        .sub_parallel_in_flight(1)
        .map_err(|_| RecoveryError::ReplayDivergence {
            step,
            detail: "parallel_in_flight underflow".to_owned(),
        })
}

pub(crate) fn reject_resolved_action(
    tracker: &ActionReplayTracker,
    action: vb_core::ActionId,
    step: vb_core::StepIdx,
) -> RecoveryResult<()> {
    if tracker.is_resolved(action, step) {
        return Err(RecoveryError::NonIdempotentActionBlocked { action, step });
    }
    Ok(())
}
