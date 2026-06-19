#![forbid(unsafe_code)]
//! Parallel in-flight peak computation from action events.

use crate::JournalEvent;
use crate::recovery::{
    ActionReplayTracker, RecoveryError, RecoveryResult, types::ActionReplayEffect,
};
use vb_core::RunFrame;

/// Computes parallel in-flight counters from action events without mutating step states.
///
/// This is used by `hydrate_run_frame_from_events` after the seed has already
/// applied step states. It only tracks action scheduling/completion for parallel
/// counters.
///
/// Returns the peak parallel in-flight count observed.
pub(crate) fn compute_parallel_in_flight(
    frame: &mut RunFrame,
    events: &[JournalEvent],
) -> RecoveryResult<u16> {
    let mut tracker = ActionReplayTracker::new();
    let mut peak: u16 = 0;

    for event in events {
        match event {
            JournalEvent::ActionScheduled { action, step, .. } => {
                if tracker.is_resolved(*action, *step) {
                    return Err(RecoveryError::NonIdempotentActionBlocked {
                        action: *action,
                        step: *step,
                    });
                }
                frame
                    .add_parallel_in_flight(1)
                    .map_err(|_| RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: "parallel_in_flight overflow".to_owned(),
                    })?;
                if frame.parallel_in_flight() > peak {
                    peak = frame.parallel_in_flight();
                }
            }
            JournalEvent::ActionScheduledTicket {
                run,
                ticket,
                input,
                output,
                ..
            } => {
                crate::recovery::action_digest::verify_action_ticket_event(*run, *ticket)?;
                let effect = tracker.mark_scheduled_ticket_effect(*ticket, *input, *output)?;
                if effect == ActionReplayEffect::Duplicate {
                    continue;
                }
                frame
                    .add_parallel_in_flight(1)
                    .map_err(|_| RecoveryError::ReplayDivergence {
                        step: ticket.step,
                        detail: "parallel_in_flight overflow".to_owned(),
                    })?;
                if frame.parallel_in_flight() > peak {
                    peak = frame.parallel_in_flight();
                }
            }
            JournalEvent::ActionCompletedEvent { action, step, .. } => {
                if tracker.is_resolved(*action, *step) {
                    return Err(RecoveryError::NonIdempotentActionBlocked {
                        action: *action,
                        step: *step,
                    });
                }
                tracker.mark_completed(*action, *step);
                frame
                    .sub_parallel_in_flight(1)
                    .map_err(|_| RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: "parallel_in_flight underflow".to_owned(),
                    })?;
            }
            JournalEvent::ActionCompletedEnvelope {
                run,
                ticket,
                output,
                outcome,
                value,
                encoded_len,
                taint,
                value_digest,
                ..
            } => {
                let verified_digest =
                    crate::recovery::action_digest::verified_action_envelope_digest(
                        *run,
                        *ticket,
                        *outcome,
                        value,
                        *encoded_len,
                        *value_digest,
                    )?;
                tracker.require_scheduled_ticket(*ticket, *output)?;
                let effect = tracker.mark_completed_envelope_effect(
                    *ticket,
                    *output,
                    *encoded_len,
                    *taint,
                    verified_digest,
                )?;
                if effect == ActionReplayEffect::Apply {
                    frame.sub_parallel_in_flight(1).map_err(|_| {
                        RecoveryError::ReplayDivergence {
                            step: ticket.step,
                            detail: "parallel_in_flight underflow".to_owned(),
                        }
                    })?;
                }
            }
            JournalEvent::ActionFailedEvent { action, step, .. } => {
                if tracker.is_resolved(*action, *step) {
                    return Err(RecoveryError::NonIdempotentActionBlocked {
                        action: *action,
                        step: *step,
                    });
                }
                tracker.mark_failed(*action, *step);
                frame
                    .sub_parallel_in_flight(1)
                    .map_err(|_| RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: "parallel_in_flight underflow".to_owned(),
                    })?;
            }
            _ => {}
        }
    }

    Ok(peak)
}
