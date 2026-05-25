#![forbid(unsafe_code)]
//! RunFrame hydration from snapshot and journal events.
//!
//! Provides:
//! - `hydrate_run_frame`: Reconstruct live RunFrame from snapshot + tail events
//! - `hydrate_run_frame_from_events`: Reconstruct live RunFrame from events only

use crate::JournalEvent;
use crate::recovery::hydrate_support::{
    apply_tail_events, compute_parallel_in_flight, decode_snapshot_slots,
    derive_dimensions_from_snapshot_and_tail, verified_action_envelope_digest,
};
use crate::recovery::types::{ActionReplayEffect, ActionReplayTracker};
use crate::recovery::types::{
    ActionReplayTracker, RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState, RecoveryError,
    RecoveryFrameSeed, RecoveryResult, RunSnapshot,
};
use vb_core::RunId;

/// Copy-only metadata needed to validate snapshot/tail hydration ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TailEventMetadata {
    /// Run carried by the tail event.
    pub(crate) run: RunId,
    /// Sequence carried by the tail event.
    pub(crate) seq: crate::EventSeq,
}

impl TailEventMetadata {
    /// Creates metadata from explicit event fields.
    #[must_use]
    pub(crate) const fn new(run: RunId, seq: crate::EventSeq) -> Self {
        Self { run, seq }
    }

    /// Projects copy metadata from a journal event.
    #[must_use]
    pub(crate) const fn from_event(event: &JournalEvent) -> Self {
        Self::new(event.run_id(), event.seq())
    }
}

/// Allocation-free classification for snapshot/tail hydration preconditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnapshotRecoveryInputViolation {
    /// Snapshot belongs to a different run.
    SnapshotRunMismatch {
        /// Run carried by the snapshot.
        snapshot_run: RunId,
        /// Snapshot sequence.
        snapshot_seq: crate::EventSeq,
    },
    /// Tail event belongs to a different run.
    TailRunMismatch {
        /// Requested run.
        expected: RunId,
        /// Event run.
        actual: RunId,
    },
    /// Tail event is not strictly after the snapshot sequence.
    TailSeqNotAfterSnapshot {
        /// Snapshot sequence.
        snapshot_seq: crate::EventSeq,
        /// Event sequence.
        actual_seq: crate::EventSeq,
    },
    /// Snapshot and tail are both empty.
    NoRecoveryData {
        /// Requested run.
        run: RunId,
    },
}

/// Validates snapshot identity without allocating an error string.
pub(crate) const fn validate_snapshot_metadata(
    snapshot_run: RunId,
    snapshot_seq: crate::EventSeq,
    run_id: RunId,
) -> Result<(), SnapshotRecoveryInputViolation> {
    if snapshot_run.get() == run_id.get() {
        Ok(())
    } else {
        Err(SnapshotRecoveryInputViolation::SnapshotRunMismatch {
            snapshot_run,
            snapshot_seq,
        })
    }
}

/// Validates one tail event's run identity without allocating an error string.
pub(crate) const fn validate_tail_run_metadata(
    event: TailEventMetadata,
    run_id: RunId,
) -> Result<(), SnapshotRecoveryInputViolation> {
    if event.run.get() == run_id.get() {
        Ok(())
    } else {
        Err(SnapshotRecoveryInputViolation::TailRunMismatch {
            expected: run_id,
            actual: event.run,
        })
    }
}

/// Validates one tail event's sequence lower bound without allocating an error string.
pub(crate) const fn validate_tail_seq_after_snapshot(
    event: TailEventMetadata,
    snapshot_seq: crate::EventSeq,
) -> Result<(), SnapshotRecoveryInputViolation> {
    if event.seq.get() > snapshot_seq.get() {
        Ok(())
    } else {
        Err(SnapshotRecoveryInputViolation::TailSeqNotAfterSnapshot {
            snapshot_seq,
            actual_seq: event.seq,
        })
    }
}

/// Validates that recovery has at least one snapshot byte or tail event.
pub(crate) const fn validate_recovery_data_present(
    tail_events_empty: bool,
    snapshot_slots_empty: bool,
    snapshot_taint_empty: bool,
    run_id: RunId,
) -> Result<(), SnapshotRecoveryInputViolation> {
    if tail_events_empty && snapshot_slots_empty && snapshot_taint_empty {
        Err(SnapshotRecoveryInputViolation::NoRecoveryData { run: run_id })
    } else {
        Ok(())
    }
}

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
    validate_snapshot_recovery_inputs(snapshot, tail_events, run_id)?;
    let snapshot_slots = decode_snapshot_slots(&snapshot.slots, &snapshot.taint, run_id)?;
    let (step_count, slot_count, first_step) =
        derive_dimensions_from_snapshot_and_tail(snapshot, tail_events, run_id, &snapshot_slots)?;
    ensure_nonzero_step_count(step_count)?;

    let mut frame = vb_core::RunFrame::new(run_id, first_step, step_count, slot_count)
        .map_err(|_| RecoveryError::FrameDimensionOverflow { run: run_id })?;
    apply_snapshot_slots(&mut frame, &snapshot_slots)?;

    let mut tracker = ActionReplayTracker::new();
    let executed = apply_tail_events(&mut frame, tail_events, &mut tracker)?;
    increment_executed(&mut frame, run_id, executed)?;
    Ok(frame)
}

fn validate_snapshot_recovery_inputs(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<()> {
    validate_snapshot_metadata(snapshot.run, snapshot.seq, run_id)
        .map_err(snapshot_input_violation_to_error)?;
    validate_tail_events_match_run(tail_events, run_id)?;
    validate_tail_events_after_snapshot(tail_events, snapshot)?;
    validate_recovery_data_present(
        tail_events.is_empty(),
        snapshot.slots.is_empty(),
        snapshot.taint.is_empty(),
        run_id,
    )
    .map_err(snapshot_input_violation_to_error)
}

fn validate_tail_events_match_run(
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<()> {
    for event in tail_events {
        validate_tail_run_metadata(TailEventMetadata::from_event(event), run_id)
            .map_err(snapshot_input_violation_to_error)?;
    }
    Ok(())
}

fn validate_tail_events_after_snapshot(
    tail_events: &[JournalEvent],
    snapshot: &RunSnapshot,
) -> RecoveryResult<()> {
    for event in tail_events {
        validate_tail_seq_after_snapshot(TailEventMetadata::from_event(event), snapshot.seq)
            .map_err(snapshot_input_violation_to_error)?;
    }
    Ok(())
}

fn snapshot_input_violation_to_error(violation: SnapshotRecoveryInputViolation) -> RecoveryError {
    match violation {
        SnapshotRecoveryInputViolation::SnapshotRunMismatch {
            snapshot_run,
            snapshot_seq,
        } => RecoveryError::CorruptSnapshot {
            run: snapshot_run,
            seq: snapshot_seq,
        },
        SnapshotRecoveryInputViolation::TailRunMismatch { expected, actual } => {
            RecoveryError::ReplayDivergence {
                step: vb_core::StepIdx::ZERO,
                detail: format!(
                    "tail event run_id mismatch: expected {expected:?}, found {actual:?}"
                ),
            }
        }
        SnapshotRecoveryInputViolation::TailSeqNotAfterSnapshot {
            snapshot_seq,
            actual_seq,
        } => RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: format!(
                "tail event seq {} is not after snapshot seq {}",
                actual_seq.get(),
                snapshot_seq.get()
            ),
        },
        SnapshotRecoveryInputViolation::NoRecoveryData { run } => {
            RecoveryError::NoRecoveryData { run }
        }
    }
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

fn apply_snapshot_slots(
    frame: &mut vb_core::RunFrame,
    snapshot_slots: &[RecoveredSlotEntry],
) -> RecoveryResult<()> {
    for entry in snapshot_slots {
        frame
            .write_slot_with_taint(entry.slot, entry.value, entry.taint)
            .map_err(|_| RecoveryError::ReplayDivergence {
                step: vb_core::StepIdx::ZERO,
                detail: "snapshot slot write out of bounds".to_owned(),
            })?;
    }
    Ok(())
}

fn increment_executed(
    frame: &mut vb_core::RunFrame,
    run_id: RunId,
    executed: u64,
) -> RecoveryResult<()> {
    for _ in 0..executed {
        frame
            .increment_executed()
            .map_err(|_| RecoveryError::FrameDimensionOverflow { run: run_id })?;
    }
    Ok(())
}

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
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run: run_id });
    }

    let seed = crate::recovery::replay::summary::recover_runtime_frame_seed_from_events(events)?;
    ensure_nonzero_step_count(seed.step_count)?;
    let mut frame = build_frame_from_seed(&seed, run_id)?;
    apply_seed_step_states(&mut frame, &seed.steps)?;
    apply_seed_slots(&mut frame, &seed.slots)?;
    apply_seed_pc(&mut frame, seed.pc)?;
    increment_executed(&mut frame, run_id, count_state_events(events, run_id)?)?;
    apply_parallel_peak(&mut frame, events)?;

    Ok(frame)
}

fn build_frame_from_seed(
    seed: &RecoveryFrameSeed,
    run_id: RunId,
) -> RecoveryResult<vb_core::RunFrame> {
    vb_core::RunFrame::new(run_id, seed.first_step, seed.step_count, seed.slot_count)
        .map_err(|_| RecoveryError::FrameDimensionOverflow { run: run_id })
}

fn apply_seed_step_states(
    frame: &mut vb_core::RunFrame,
    steps: &[RecoveredStepEntry],
) -> RecoveryResult<()> {
    for entry in steps {
        let result = match entry.state {
            RecoveredStepState::Running => frame.mark_running(entry.step),
            RecoveredStepState::Succeeded => frame.mark_succeeded(entry.step),
            RecoveredStepState::Failed => frame.mark_failed(entry.step),
            RecoveredStepState::Waiting => frame
                .mark_running(entry.step)
                .and_then(|_| frame.mark_waiting(entry.step)),
            RecoveredStepState::Asking => frame
                .mark_running(entry.step)
                .and_then(|_| frame.mark_asking(entry.step)),
        };
        result.map_err(|_| RecoveryError::ReplayDivergence {
            step: entry.step,
            detail: "seed step state transition failed".to_owned(),
        })?;
    }
    Ok(())
}

fn apply_seed_slots(
    frame: &mut vb_core::RunFrame,
    slots: &[RecoveredSlotEntry],
) -> RecoveryResult<()> {
    for entry in slots {
        frame
            .write_slot_with_taint(entry.slot, entry.value, entry.taint)
            .map_err(|_| RecoveryError::ReplayDivergence {
                step: vb_core::StepIdx::ZERO,
                detail: "seed slot write out of bounds".to_owned(),
            })?;
    }
    Ok(())
}

fn apply_seed_pc(frame: &mut vb_core::RunFrame, pc: vb_core::StepIdx) -> RecoveryResult<()> {
    frame
        .set_pc(pc)
        .map_err(|_| RecoveryError::ReplayDivergence {
            step: pc,
            detail: "seed pc out of bounds".to_owned(),
        })
}

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
        JournalEvent::ActionScheduledTicket { ticket, .. } => {
            reject_resolved_action(tracker, ticket.action, ticket.step)?;
            Ok(true)
        }
        JournalEvent::ActionCompletedEvent { action, step, .. } => {
            reject_resolved_action(tracker, *action, *step)?;
            tracker.mark_completed(*action, *step);
            Ok(true)
        }
        JournalEvent::ActionCompletedEnvelope {
            ticket,
            output,
            value,
            encoded_len,
            taint,
            value_digest,
            ..
        } => {
            let verified_digest =
                verified_action_envelope_digest(*ticket, value, *encoded_len, *value_digest)?;
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

fn apply_parallel_peak(
    frame: &mut vb_core::RunFrame,
    events: &[JournalEvent],
) -> RecoveryResult<()> {
    let peak =
        compute_parallel_in_flight(frame, events).map_err(|_| RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: "parallel in-flight computation failed".to_owned(),
        })?;
    frame.set_max_parallel_in_flight(peak);
    Ok(())
}
