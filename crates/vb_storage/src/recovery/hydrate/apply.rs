#![forbid(unsafe_code)]
//! Frame mutation helpers for snapshot and event replay hydration.
//!
//! Provides:
//! - `apply_snapshot_slots`: Write pre-decoded slot entries (with taint) into a RunFrame
//! - `apply_seed_step_states`: Exhaustive step-state transition from recovery summary
//! - `apply_seed_slots`, `apply_seed_pc`: Apply seed data for events-only hydration
//! - `increment_executed`: Bump the executed counter for a run
//! - `apply_parallel_peak`: Derive and store max parallel in-flight from events

use crate::recovery::{RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState, RecoveryError, RecoveryResult};
use crate::JournalEvent;
use crate::recovery::event_replay::compute_parallel_in_flight;
use vb_core::{RunFrame, RunId, StepIdx};

/// Write pre-decoded snapshot slots into the frame with their taint.
pub(crate) fn apply_snapshot_slots(
    frame: &mut RunFrame,
    snapshot_slots: &[RecoveredSlotEntry],
) -> RecoveryResult<()> {
    for entry in snapshot_slots {
        frame
            .write_slot_with_taint(entry.slot, entry.value, entry.taint)
            .map_err(|_| RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: "snapshot slot write out of bounds".to_owned(),
            })?;
    }
    Ok(())
}

/// Exhaustively apply recovery summary step states to a frame.
pub(crate) fn apply_seed_step_states(
    frame: &mut RunFrame,
    steps: &[RecoveredStepEntry],
) -> RecoveryResult<()> {
    for entry in steps {
        let result = match entry.state {
            RecoveredStepState::Running => frame.mark_running(entry.step),
            RecoveredStepState::Succeeded => frame.mark_succeeded(entry.step),
            RecoveredStepState::Failed => frame.mark_failed(entry.step),
            RecoveredStepState::Waiting => {
                frame.mark_running(entry.step).and_then(|_| frame.mark_waiting(entry.step))
            }
            RecoveredStepState::Asking => {
                frame.mark_running(entry.step).and_then(|_| frame.mark_asking(entry.step))
            }
        };
        result.map_err(|_| RecoveryError::ReplayDivergence {
            step: entry.step,
            detail: "seed step state transition failed".to_owned(),
        })?;
    }
    Ok(())
}

/// Write pre-decoded seed slots into the frame with their taint.
pub(crate) fn apply_seed_slots(
    frame: &mut RunFrame,
    slots: &[RecoveredSlotEntry],
) -> RecoveryResult<()> {
    for entry in slots {
        frame
            .write_slot_with_taint(entry.slot, entry.value, entry.taint)
            .map_err(|_| RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: "seed slot write out of bounds".to_owned(),
            })?;
    }
    Ok(())
}

/// Set the program counter from seed data.
pub(crate) fn apply_seed_pc(frame: &mut RunFrame, pc: StepIdx) -> RecoveryResult<()> {
    frame
        .set_pc(pc)
        .map_err(|_| RecoveryError::ReplayDivergence {
            step: pc,
            detail: "seed pc out of bounds".to_owned(),
        })
}

/// Increment the executed counter `executed` times, guarding on overflow.
pub(crate) fn increment_executed(
    frame: &mut RunFrame,
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

/// Derive and store the max parallel in-flight from journal events.
pub(crate) fn apply_parallel_peak(
    frame: &mut RunFrame,
    events: &[JournalEvent],
) -> RecoveryResult<()> {
    let peak = compute_parallel_in_flight(frame, events).map_err(|_| RecoveryError::ReplayDivergence {
        step: StepIdx::ZERO,
        detail: "parallel in-flight computation failed".to_owned(),
    })?;
    frame.set_max_parallel_in_flight(peak);
    Ok(())
}
