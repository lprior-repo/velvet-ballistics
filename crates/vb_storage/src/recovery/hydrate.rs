#![forbid(unsafe_code)]
//! RunFrame hydration from snapshot and journal events.
//!
//! Provides:
//! - `hydrate_run_frame`: Reconstruct live RunFrame from snapshot + tail events
//! - `hydrate_run_frame_from_events`: Reconstruct live RunFrame from events only

use crate::JournalEvent;
use crate::recovery::hydrate_support::{
    apply_tail_events, compute_parallel_in_flight, decode_snapshot_slots,
    derive_dimensions_from_snapshot_and_tail,
};
use crate::recovery::types::{
    ActionReplayTracker, RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState, RecoveryError,
    RecoveryFrameSeed, RecoveryResult, RunSnapshot,
};
use vb_core::RunId;

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
    if snapshot.run != run_id {
        return Err(RecoveryError::CorruptSnapshot {
            run: snapshot.run,
            seq: snapshot.seq,
        });
    }
    validate_tail_events_match_run(tail_events, run_id)?;
    validate_tail_events_after_snapshot(tail_events, snapshot)?;
    if tail_events.is_empty() && snapshot.slots.is_empty() && snapshot.taint.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run: run_id });
    }
    Ok(())
}

fn validate_tail_events_match_run(
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<()> {
    for event in tail_events {
        if event.run_id() != run_id {
            return Err(RecoveryError::ReplayDivergence {
                step: vb_core::StepIdx::ZERO,
                detail: format!(
                    "tail event run_id mismatch: expected {:?}, found {:?}",
                    run_id,
                    event.run_id()
                ),
            });
        }
    }
    Ok(())
}

fn validate_tail_events_after_snapshot(
    tail_events: &[JournalEvent],
    snapshot: &RunSnapshot,
) -> RecoveryResult<()> {
    for event in tail_events {
        if event.seq() <= snapshot.seq {
            return Err(RecoveryError::ReplayDivergence {
                step: vb_core::StepIdx::ZERO,
                detail: format!(
                    "tail event seq {} is not after snapshot seq {}",
                    event.seq().get(),
                    snapshot.seq.get()
                ),
            });
        }
    }
    Ok(())
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
    let count = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                JournalEvent::StepStarted { .. }
                    | JournalEvent::StepSucceeded { .. }
                    | JournalEvent::SlotWrittenEvent { .. }
                    | JournalEvent::ActionScheduled { .. }
                    | JournalEvent::ActionCompletedEvent { .. }
                    | JournalEvent::ActionFailedEvent { .. }
                    | JournalEvent::WaitScheduledEvent { .. }
                    | JournalEvent::AskScheduledEvent { .. }
            )
        })
        .count();
    u64::try_from(count).map_err(|_| RecoveryError::FrameDimensionOverflow { run: run_id })
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
