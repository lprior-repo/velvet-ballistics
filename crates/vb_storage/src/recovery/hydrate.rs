#![forbid(unsafe_code)]
//! RunFrame hydration from snapshot and journal events.
//!
//! Provides:
//! - `hydrate_run_frame`: Reconstruct live RunFrame from snapshot + tail events
//! - `hydrate_run_frame_from_events`: Reconstruct live RunFrame from events only

use crate::JournalEvent;
use crate::recovery::hydrate_support::{
    apply_tail_events, decode_snapshot_slots, derive_dimensions_from_snapshot_and_tail,
    verified_action_envelope_digest, verify_action_ticket_event,
};
use crate::recovery::types::ActionReplayEffect;
use crate::recovery::types::{
    ActionReplayTracker, RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState, RecoveryError,
    RecoveryFrameSeed, RecoveryResult, RunSnapshot,
};
use vb_core::RunId;

/// Production proof surface for snapshot-plus-tail run identity.
#[must_use]
pub fn hydrate_snapshot_tail_run_matches(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> bool {
    snapshot.run == run_id && tail_events.iter().all(|event| event.run_id() == run_id)
}

/// Production proof surface for snapshot-plus-tail sequence ordering.
#[must_use]
pub fn hydrate_snapshot_tail_seq_after_snapshot(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
) -> bool {
    tail_events.iter().all(|event| event.seq() > snapshot.seq)
}

/// Production proof surface for non-empty recovery evidence.
#[must_use]
pub fn hydrate_snapshot_tail_has_evidence(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
) -> bool {
    !tail_events.is_empty() || !snapshot.slots.is_empty() || !snapshot.taint.is_empty()
}

/// Production proof surface for hydrate_run_frame preconditions that do not decode bytes.
#[must_use]
pub fn hydrate_snapshot_tail_preconditions(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> bool {
    hydrate_snapshot_tail_run_matches(snapshot, tail_events, run_id)
        && hydrate_snapshot_tail_seq_after_snapshot(snapshot, tail_events)
        && hydrate_snapshot_tail_has_evidence(snapshot, tail_events)
}

/// Production proof surface for events-only hydrate preconditions.
#[must_use]
pub const fn hydrate_events_preconditions(events: &[JournalEvent]) -> bool {
    !events.is_empty()
}

/// Production proof surface for positive frame dimensions.
#[must_use]
pub const fn hydrate_dimensions_positive(step_count: u16, slot_count: u16) -> bool {
    step_count > 0 && slot_count > 0
}

/// Copy-only metadata needed to validate snapshot/tail hydration ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TailEventMetadata {
    pub(crate) run: RunId,
    pub(crate) seq: crate::EventSeq,
}

impl TailEventMetadata {
    #[must_use]
    pub(crate) const fn new(run: RunId, seq: crate::EventSeq) -> Self {
        Self { run, seq }
    }

    #[must_use]
    pub(crate) const fn from_event(event: &JournalEvent) -> Self {
        Self::new(event.run_id(), event.seq())
    }
}

/// Allocation-free classification for snapshot/tail hydration preconditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnapshotRecoveryInputViolation {
    SnapshotRunMismatch {
        snapshot_run: RunId,
        snapshot_seq: crate::EventSeq,
    },
    TailRunMismatch {
        expected: RunId,
        actual: RunId,
    },
    TailSeqNotAfterSnapshot {
        snapshot_seq: crate::EventSeq,
        actual_seq: crate::EventSeq,
    },
    NoRecoveryData {
        run: RunId,
    },
}

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
    let executed = apply_replay_accounting(&mut frame, events, run_id)?;
    increment_executed(&mut frame, run_id, executed)?;

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

fn apply_replay_accounting(
    frame: &mut vb_core::RunFrame,
    events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<u64> {
    let mut tracker = ActionReplayTracker::new();
    let mut count = 0u64;
    let mut peak = 0u16;
    for event in events {
        if apply_accounting_event(frame, event, &mut tracker)? {
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
) -> RecoveryResult<bool> {
    match event {
        JournalEvent::ActionScheduled { action, step, .. } => {
            reject_resolved_action(tracker, *action, *step)?;
            add_replay_parallel_in_flight(frame, *step)?;
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
            if effect == ActionReplayEffect::Apply {
                add_replay_parallel_in_flight(frame, ticket.step)?;
            }
            Ok(effect == ActionReplayEffect::Apply)
        }
        JournalEvent::ActionCompletedEvent { action, step, .. } => {
            reject_resolved_action(tracker, *action, *step)?;
            tracker.mark_completed(*action, *step);
            sub_replay_parallel_in_flight(frame, *step)?;
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
            if effect == ActionReplayEffect::Apply {
                sub_replay_parallel_in_flight(frame, ticket.step)?;
            }
            Ok(effect == ActionReplayEffect::Apply)
        }
        JournalEvent::ActionFailedEvent { action, step, .. } => {
            reject_resolved_action(tracker, *action, *step)?;
            tracker.mark_failed(*action, *step);
            sub_replay_parallel_in_flight(frame, *step)?;
            Ok(true)
        }
        JournalEvent::StepStarted { .. }
        | JournalEvent::StepSucceeded { .. }
        | JournalEvent::SlotWrittenEvent { .. }
        | JournalEvent::WaitScheduledEvent { .. }
        | JournalEvent::AskScheduledEvent { .. }
        | JournalEvent::AskTimedOutEvent { .. } => Ok(true),
        _ => Ok(false),
    }
}

fn add_replay_parallel_in_flight(
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

fn sub_replay_parallel_in_flight(
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
