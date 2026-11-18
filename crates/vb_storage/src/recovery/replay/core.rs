#![forbid(unsafe_code)]
//! Core replay logic for journal recovery.
//!
//! Provides:
//! - Event replay with divergence detection
//! - Non-idempotent action blocking
//! - Snapshot-plus-tail replay

use super::attempt::compute_max_attempt;
pub use super::attempt::{
    replay_attempt_is_current, replay_attempt_is_stale, replay_attempt_or_default,
    replay_event_has_state_effect, replay_event_is_stale_state_effect, replay_step_order_diverges,
};
use crate::recovery::hydrate_support::{
    verified_action_envelope_digest, verify_action_ticket_event,
};
use crate::recovery::types::{ActionReplayTracker, RecoveryError, RecoveryResult};
use crate::{EventSeq, JournalEvent};
use vb_core::{ActionId, StepIdx, WorkflowDigest};

mod full;

pub use full::{
    extract_terminal, is_terminal_event, load_snapshot, recover_full_journal,
    recover_snapshot_plus_tail,
};

/// Core replay logic for all journal event kinds.
/// Populates the action tracker and detects divergence.
///
/// ## Filtering (PRE-001)
/// Only events from the latest execution attempt affect live state.
/// Events from older attempts are excluded from state transition logic
/// but are still included in the returned output for diagnostics.
pub fn replay_events(
    events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RecoveryResult<Vec<JournalEvent>> {
    replay_events_with_schedule_requirement(events, tracker, true, expected_action_abi_digests)
}

fn replay_events_with_schedule_requirement(
    events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
    require_schedule: bool,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RecoveryResult<Vec<JournalEvent>> {
    validate_contiguous_sequences(events)?;
    let max_attempt = compute_max_attempt(events);
    let mut replayed = Vec::new();
    let mut last_step: Option<StepIdx> = None;
    for event in events {
        if replay_attempt_is_stale(event.attempt(), max_attempt) {
            replayed.push(event.clone());
            continue;
        }
        last_step = dispatch_replay_event(
            event,
            tracker,
            require_schedule,
            last_step,
            expected_action_abi_digests,
        )?;
        replayed.push(event.clone());
    }
    Ok(replayed)
}

/// Dispatch a single non-stale event to its per-variant helper. Returns the
/// updated `last_step` so `StepStarted` ordering state flows across
/// iterations; all other variants pass `last_step` through unchanged.
fn dispatch_replay_event(
    event: &JournalEvent,
    tracker: &mut ActionReplayTracker,
    require_schedule: bool,
    last_step: Option<StepIdx>,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RecoveryResult<Option<StepIdx>> {
    match event {
        JournalEvent::StepStarted { step, .. } => replay_step_started_event(*step, last_step),
        JournalEvent::ActionCompletedEnvelope { .. } => {
            replay_action_completed_envelope_event(
                event,
                tracker,
                require_schedule,
                expected_action_abi_digests,
            )?;
            Ok(last_step)
        }
        JournalEvent::ActionScheduled { .. }
        | JournalEvent::ActionScheduledTicket { .. }
        | JournalEvent::ActionCompletedEvent { .. }
        | JournalEvent::ActionFailedEvent { .. } => {
            replay_action_event(event, tracker, expected_action_abi_digests)?;
            Ok(last_step)
        }
        _ => Ok(last_step),
    }
}

/// Step ordering check for `StepStarted` events.
fn replay_step_started_event(
    step: StepIdx,
    last_step: Option<StepIdx>,
) -> RecoveryResult<Option<StepIdx>> {
    if replay_step_order_diverges(last_step, step) {
        let previous_step = match last_step {
            Some(value) => value,
            None => StepIdx::ZERO,
        };
        return Err(RecoveryError::ReplayDivergence {
            step,
            detail: format!(
                "step {} executed before previous step {}",
                step.get(),
                previous_step.get()
            ),
        });
    }
    Ok(Some(step))
}
/// Checks whether the found action ABI digest matches any of the expected digests.
///
/// Returns `Ok(())` only when the action is in the expected list and the
/// persisted digest is non-zero and matches.
///
/// Returns `ActionAbiMismatch` when the action is in the expected list
/// but the digests differ.
pub(crate) fn check_action_abi_digest_against_expected(
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
fn replay_action_event(
    event: &JournalEvent,
    tracker: &mut ActionReplayTracker,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RecoveryResult<()> {
    match event {
        JournalEvent::ActionScheduled { action, step, .. } => {
            reject_if_resolved(tracker, *action, *step)
        }
        JournalEvent::ActionScheduledTicket {
            run,
            ticket,
            input,
            output,
            action_abi_digest,
            ..
        } => {
            verify_action_ticket_event(*run, *ticket)?;
            check_action_abi_digest_against_expected(
                ticket.action,
                *action_abi_digest,
                expected_action_abi_digests,
            )?;
            tracker
                .mark_scheduled_ticket_effect(*ticket, *input, *output, *action_abi_digest)
                .map(|_| ())
        }
        JournalEvent::ActionCompletedEvent { action, step, .. } => {
            reject_if_resolved(tracker, *action, *step)?;
            tracker.mark_completed(*action, *step);
            Ok(())
        }
        JournalEvent::ActionFailedEvent { action, step, .. } => {
            reject_if_resolved(tracker, *action, *step)?;
            tracker.mark_failed(*action, *step);
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Envelope variant: verifies the action envelope digest, optionally
/// requires a prior scheduled ticket (driven by `require_schedule`), then
/// marks the envelope complete.
fn replay_action_completed_envelope_event(
    event: &JournalEvent,
    tracker: &mut ActionReplayTracker,
    require_schedule: bool,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RecoveryResult<()> {
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
        return Ok(());
    };
    let verified_digest = verified_action_envelope_digest(
        *run,
        *ticket,
        *outcome,
        value,
        *encoded_len,
        *value_digest,
    )?;
    if require_schedule {
        tracker.require_scheduled_ticket(*ticket, *output, *action_abi_digest)?;
    }
    check_action_abi_digest_against_expected(
        ticket.action,
        *action_abi_digest,
        expected_action_abi_digests,
    )?;
    tracker.mark_completed_envelope(
        *ticket,
        *output,
        *encoded_len,
        *taint,
        verified_digest,
        *action_abi_digest,
    )
}
/// Reject the event when the (action, step) pair has already been resolved
/// during this replay, preserving the non-idempotency invariant.
fn reject_if_resolved(
    tracker: &ActionReplayTracker,
    action: ActionId,
    step: StepIdx,
) -> RecoveryResult<()> {
    if tracker.is_resolved(action, step) {
        return Err(RecoveryError::NonIdempotentActionBlocked { action, step });
    }
    Ok(())
}

fn validate_contiguous_sequences(events: &[JournalEvent]) -> RecoveryResult<()> {
    let Some(first) = events.first() else {
        return Ok(());
    };
    let mut expected = first.seq();
    for event in events {
        let seq = event.seq();
        if seq != expected {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: format!(
                    "journal sequence violation: expected {}, found {}",
                    expected.get(),
                    seq.get()
                ),
            });
        }
        expected = EventSeq::new(expected.get().saturating_add(1));
    }
    Ok(())
}
