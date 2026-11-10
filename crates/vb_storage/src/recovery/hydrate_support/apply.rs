use super::apply_action::{
    apply_action_completed_envelope, apply_action_completed_event, apply_action_failed,
    apply_action_scheduled, apply_action_scheduled_ticket,
};
use super::resolve_slot_taint_or_fail;

use crate::JournalEvent;
use crate::recovery::types::{ActionReplayTracker, RecoveryError, RecoveryResult};
use vb_core::{ActionId, RunId, WorkflowDigest};

/// Categorizes the result of dispatching an event to a category helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApplyOutcome {
    /// Helper handled the event and contributed to the executed count.
    Executed,
    /// Helper handled the event but the caller must not increment (Duplicate skip).
    Skipped,
    /// Helper did not recognize the event variant.
    NotApplicable,
}

/// Applies tail journal events to a mutable RunFrame, tracking action resolution.
///
/// Returns the count of state-affecting events applied.
pub(crate) fn apply_tail_events(
    frame: &mut vb_core::RunFrame,
    tail_events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RecoveryResult<u64> {
    frame.set_max_parallel_in_flight(0);
    let mut executed = 0u64;
    for event in tail_events {
        let outcome = match event {
            JournalEvent::StepStarted { .. }
            | JournalEvent::StepSucceeded { .. }
            | JournalEvent::StepFailed { .. } => apply_step_lifecycle(frame, event)?,
            JournalEvent::ActionScheduled { .. } => apply_action_scheduled(frame, event, tracker)?,
            JournalEvent::ActionScheduledTicket { .. } => {
                apply_action_scheduled_ticket(frame, event, tracker, expected_action_abi_digests)?
            }
            JournalEvent::ActionCompletedEvent { .. } => {
                apply_action_completed_event(frame, event, tracker)?
            }
            JournalEvent::ActionCompletedEnvelope { .. } => {
                apply_action_completed_envelope(frame, event, tracker, expected_action_abi_digests)?
            }
            JournalEvent::ActionFailedEvent { .. } => apply_action_failed(frame, event, tracker)?,
            JournalEvent::SlotWrittenEvent { .. } => apply_slot_written(frame, event)?,
            JournalEvent::WaitScheduledEvent { .. }
            | JournalEvent::AskScheduledEvent { .. }
            | JournalEvent::AskTimedOutEvent { .. } => apply_signal_event(frame, event)?,
            _ => ApplyOutcome::NotApplicable,
        };
        if outcome == ApplyOutcome::Executed {
            executed = executed.saturating_add(1);
        }
    }
    Ok(executed)
}

fn apply_step_lifecycle(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
) -> RecoveryResult<ApplyOutcome> {
    match event {
        JournalEvent::StepStarted { step, .. } => {
            frame
                .mark_running(*step)
                .map_err(|_e| RecoveryError::ReplayDivergence {
                    step: *step,
                    detail: "mark_running failed".to_owned(),
                })?;
            Ok(ApplyOutcome::Executed)
        }
        JournalEvent::StepSucceeded { step, .. } => {
            frame
                .mark_succeeded(*step)
                .map_err(|_e| RecoveryError::ReplayDivergence {
                    step: *step,
                    detail: "mark_succeeded failed".to_owned(),
                })?;
            Ok(ApplyOutcome::Executed)
        }
        JournalEvent::StepFailed { step, .. } => {
            frame
                .mark_failed(*step)
                .map_err(|_e| RecoveryError::ReplayDivergence {
                    step: *step,
                    detail: "mark_failed failed".to_owned(),
                })?;
            Ok(ApplyOutcome::Executed)
        }
        _ => Ok(ApplyOutcome::NotApplicable),
    }
}

fn reject_missing_slot_payload(run: RunId) -> RecoveryError {
    RecoveryError::UnsupportedFrameSeed {
        run,
        reason: String::from("slot_values"),
    }
}

fn decode_slot_value(
    bytes: &[u8],
    run: RunId,
    slot: vb_core::SlotIdx,
) -> RecoveryResult<vb_core::SlotValue> {
    postcard::from_bytes(bytes).map_err(|_| RecoveryError::ReplayDivergence {
        step: vb_core::StepIdx::ZERO,
        detail: format!("slot value decode failed for run {:?} slot {:?}", run, slot),
    })
}

fn apply_slot_written(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
) -> RecoveryResult<ApplyOutcome> {
    let JournalEvent::SlotWrittenEvent {
        run, slot, value, ..
    } = event
    else {
        return Ok(ApplyOutcome::NotApplicable);
    };
    let Some(bytes) = value else {
        return Err(reject_missing_slot_payload(*run));
    };
    let slot_value = decode_slot_value(bytes, *run, *slot)?;
    let taint = resolve_slot_taint_or_fail(frame, *slot)?;
    write_slot_with_replay_divergence(frame, *slot, slot_value, taint)?;
    Ok(ApplyOutcome::Executed)
}

fn write_slot_with_replay_divergence(
    frame: &mut vb_core::RunFrame,
    slot: vb_core::SlotIdx,
    slot_value: vb_core::SlotValue,
    taint: vb_core::Taint,
) -> RecoveryResult<()> {
    frame
        .write_slot_with_taint(slot, slot_value, taint)
        .map_err(|_| RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: "slot write out of bounds".to_owned(),
        })
}

fn apply_signal_event(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
) -> RecoveryResult<ApplyOutcome> {
    match event {
        JournalEvent::WaitScheduledEvent { step, .. } => apply_signal_wait(frame, *step),
        JournalEvent::AskScheduledEvent { step, .. } => apply_signal_ask(frame, *step),
        JournalEvent::AskTimedOutEvent { step, .. } => apply_signal_ask_timeout(frame, *step),
        _ => Ok(ApplyOutcome::NotApplicable),
    }
}

fn apply_signal_wait(
    frame: &mut vb_core::RunFrame,
    step: vb_core::StepIdx,
) -> RecoveryResult<ApplyOutcome> {
    ensure_step_running(frame, step, "waiting")?;
    frame
        .mark_waiting(step)
        .map_err(|_e| RecoveryError::ReplayDivergence {
            step,
            detail: "mark_waiting failed".to_owned(),
        })?;
    Ok(ApplyOutcome::Executed)
}

fn apply_signal_ask(
    frame: &mut vb_core::RunFrame,
    step: vb_core::StepIdx,
) -> RecoveryResult<ApplyOutcome> {
    ensure_step_running(frame, step, "asking")?;
    frame
        .mark_asking(step)
        .map_err(|_e| RecoveryError::ReplayDivergence {
            step,
            detail: "mark_asking failed".to_owned(),
        })?;
    Ok(ApplyOutcome::Executed)
}

fn apply_signal_ask_timeout(
    frame: &mut vb_core::RunFrame,
    step: vb_core::StepIdx,
) -> RecoveryResult<ApplyOutcome> {
    frame
        .mark_running(step)
        .and_then(|_| frame.mark_succeeded(step))
        .map_err(|_e| RecoveryError::ReplayDivergence {
            step,
            detail: "mark ask timeout resolved failed".to_owned(),
        })?;
    Ok(ApplyOutcome::Executed)
}

fn ensure_step_running(
    frame: &mut vb_core::RunFrame,
    step: vb_core::StepIdx,
    context: &str,
) -> RecoveryResult<()> {
    let current = frame
        .step_state(step)
        .map_err(|_| RecoveryError::ReplayDivergence {
            step,
            detail: "step_state read failed".to_owned(),
        })?;
    if current == vb_core::StepState::Pending {
        frame
            .mark_running(step)
            .map_err(|_e| RecoveryError::ReplayDivergence {
                step,
                detail: format!("mark_running before {context} failed"),
            })?;
    }
    Ok(())
}
