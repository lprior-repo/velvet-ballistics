#![forbid(unsafe_code)]
//! RunFrame hydration from snapshot+tail journal events or events-only.
//!
//! Organized into:
//! - `invariants`: Pure proof-surface predicates and metadata types
//! - `validation`: Input validation orchestration and error mapping
//! - `apply`: Frame mutation helpers (snapshot slots, seed states, parallel peak)
//!
//! Provides:
//! - `hydrate_run_frame`: Reconstruct live RunFrame from snapshot + tail events
//! - `hydrate_run_frame_from_events`: Reconstruct live RunFrame from events only

mod apply;
pub mod invariants;
pub mod validation;

use crate::JournalEvent;
use crate::recovery::action_digest::{verified_action_envelope_digest, verify_action_ticket_event};
use crate::recovery::event_replay::apply_tail_events;
use crate::recovery::snapshot_decode::{
    decode_snapshot_slots, derive_dimensions_from_snapshot_and_tail,
};
use crate::recovery::types::ActionReplayEffect;
use crate::recovery::{
    ActionReplayTracker, RecoveryError, RecoveryFrameSeed, RecoveryResult, RunSnapshot,
};
use vb_core::RunId;

// Re-export invariant predicates for downstream consumers.
pub use invariants as invariant;
pub use invariants::{
    hydrate_dimensions_positive, hydrate_events_preconditions, hydrate_snapshot_tail_has_evidence,
    hydrate_snapshot_tail_preconditions, hydrate_snapshot_tail_run_matches,
    hydrate_snapshot_tail_seq_after_snapshot,
};
pub use invariants::{SnapshotRecoveryInputViolation, TailEventMetadata};
pub use validation::{
    validate_recovery_data_present, validate_snapshot_metadata,
    validate_snapshot_recovery_inputs, validate_tail_events_after_snapshot,
    validate_tail_first_seq_contiguous_with_snapshot, validate_tail_run_metadata,
    validate_tail_seq_after_snapshot,
};

// ---------------------------------------------------------------------------
// Snapshot-plus-tail hydration
// ---------------------------------------------------------------------------

/// Hydrates a live RunFrame from a snapshot plus ordered tail journal events.
///
/// Reconstructs the full runtime frame by decoding the snapshot's compact
/// slot/taint data and applying tail events on top.
///
/// # Errors
///
/// Returns `RecoveryError` when:
/// - Snapshot run_id does not match requested run_id
/// - Tail events contain a different run_id
/// - Tail event seq is not strictly after snapshot seq
/// - Snapshot bytes are corrupt or undecodable
/// - No snapshot and no tail events are provided
/// - Derived dimensions are zero or overflow `u16`
pub fn hydrate_run_frame(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<vb_core::RunFrame> {
    validation::validate_snapshot_recovery_inputs(snapshot, tail_events, run_id)?;
    let snapshot_slots = decode_snapshot_slots(&snapshot.slots, &snapshot.taint, run_id)?;
    let (step_count, slot_count, first_step) =
        derive_dimensions_from_snapshot_and_tail(snapshot, tail_events, run_id, &snapshot_slots)?;
    ensure_nonzero_step_count(step_count)?;

    let mut frame = vb_core::RunFrame::new(run_id, first_step, step_count, slot_count)
        .map_err(|_| RecoveryError::FrameDimensionOverflow { run: run_id })?;
    apply::apply_snapshot_slots(&mut frame, &snapshot_slots)?;

    let mut tracker = ActionReplayTracker::new();
    let executed = apply_tail_events(&mut frame, tail_events, &mut tracker)?;
    apply::increment_executed(&mut frame, run_id, executed)?;
    Ok(frame)
}

fn ensure_nonzero_step_count(step_count: u16) -> RecoveryResult<()> {
    if step_count == 0 {
        return Err(RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: "derived step_count is zero".to_owned(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Events-only hydration
// ---------------------------------------------------------------------------

/// Hydrates a live RunFrame from full journal events (no snapshot).
///
/// # Errors
///
/// Returns `RecoveryError` when:
/// - Events are empty
/// - Derived dimensions are zero or overflow `u16`
pub fn hydrate_run_frame_from_events(
    events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<vb_core::RunFrame> {
    if !invariants::hydrate_events_preconditions(events) {
        return Err(RecoveryError::NoRecoveryData { run: run_id });
    }

    let seed = crate::recovery::replay::summary::recover_runtime_frame_seed_from_events(events)?;
    ensure_nonzero_step_count(seed.step_count)?;
    let mut frame = build_frame_from_seed(&seed, run_id)?;
    apply::apply_seed_step_states(&mut frame, &seed.steps)?;
    apply::apply_seed_slots(&mut frame, &seed.slots)?;
    apply::apply_seed_pc(&mut frame, seed.pc)?;
    apply::increment_executed(&mut frame, run_id, count_state_events(events, run_id)?)?;
    apply::apply_parallel_peak(&mut frame, events)?;

    Ok(frame)
}

fn build_frame_from_seed(
    seed: &RecoveryFrameSeed,
    run_id: RunId,
) -> RecoveryResult<vb_core::RunFrame> {
    vb_core::RunFrame::new(run_id, seed.first_step, seed.step_count, seed.slot_count)
        .map_err(|_| RecoveryError::FrameDimensionOverflow { run: run_id })
}

/// Count state-effect events across the journal, tracking non-idempotent actions.
fn count_state_events(events: &[JournalEvent], run_id: RunId) -> RecoveryResult<u64> {
    let mut tracker = ActionReplayTracker::new();
    let mut count = 0u64;
    for event in events {
        if count_state_event(event, &mut tracker)? {
            count = count.saturating_add(1);
        }
    }
    if count == u64::MAX {
        return Err(RecoveryError::FrameDimensionOverflow { run: run_id });
    }
    Ok(count)
}

fn count_state_event(
    event: &JournalEvent,
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<bool> {
    match event {
        JournalEvent::ActionScheduled { action, step, .. } => {
            reject_resolved_action(tracker, *action, *step)?;
            Ok(true)
        }
        JournalEvent::ActionScheduledTicket {
            run,
            ticket,
            input,
            output,
            ..
        } => {
            verify_action_ticket_event(*run, *ticket)?;
            let effect = tracker.mark_scheduled_ticket_effect(*ticket, *input, *output)?;
            Ok(effect == ActionReplayEffect::Apply)
        }
        JournalEvent::ActionCompletedEvent { action, step, .. } => {
            reject_resolved_action(tracker, *action, *step)?;
            tracker.mark_completed(*action, *step);
            Ok(true)
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
            let verified_digest = verified_action_envelope_digest(
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
            Ok(effect == ActionReplayEffect::Apply)
        }
        JournalEvent::ActionFailedEvent { action, step, .. } => {
            reject_resolved_action(tracker, *action, *step)?;
            tracker.mark_failed(*action, *step);
            Ok(true)
        }
        JournalEvent::StepStarted { .. }
        | JournalEvent::StepSucceeded { .. }
        | JournalEvent::SlotWrittenEvent { .. }
        | JournalEvent::WaitScheduledEvent { .. }
        | JournalEvent::AskScheduledEvent { .. } => Ok(true),
        _ => Ok(false),
    }
}

fn reject_resolved_action(
    tracker: &ActionReplayTracker,
    action: vb_core::ActionId,
    step: vb_core::StepIdx,
) -> RecoveryResult<()> {
    if tracker.is_resolved(action, step) {
        return Err(RecoveryError::NonIdempotentActionBlocked { action, step });
    }
    Ok(())
}
