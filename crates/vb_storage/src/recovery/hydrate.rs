#![forbid(unsafe_code)]
//! RunFrame hydration from snapshot and journal events.
//!
//! Provides:
//! - `hydrate_run_frame`: Reconstruct live RunFrame from snapshot + tail events
//! - `hydrate_run_frame_from_events`: Reconstruct live RunFrame from events only

use crate::recovery::types::{
    ActionReplayTracker, RecoveredStepState, RecoveryError, RecoveryResult, RunSnapshot,
};
use crate::recovery::hydrate_support::{
    apply_tail_events, compute_parallel_in_flight, decode_snapshot_slots,
    derive_dimensions_from_snapshot_and_tail,
};
use crate::JournalEvent;
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
    // PRE-1: snapshot.run must match requested run_id
    if snapshot.run != run_id {
        return Err(RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: format!(
                "snapshot run_id mismatch: expected {:?}, found {:?}",
                run_id, snapshot.run
            ),
        });
    }

    // PRE-2: tail events must all belong to run_id
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

    // PRE-3: tail events must be strictly after snapshot seq
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

    // If no tail events and snapshot is empty, fail
    if tail_events.is_empty() && snapshot.slots.is_empty() && snapshot.taint.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run: run_id });
    }

    // Decode snapshot slot/taint bytes
    let snapshot_slots = decode_snapshot_slots(&snapshot.slots, &snapshot.taint, run_id)?;

    // Derive dimensions from snapshot + tail events
    let (step_count, slot_count, first_step) =
        derive_dimensions_from_snapshot_and_tail(snapshot, tail_events, run_id)?;

    if step_count == 0 {
        return Err(RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: "derived step_count is zero".to_owned(),
        });
    }

    // Build base frame
    let mut frame = vb_core::RunFrame::new(run_id, first_step, step_count, slot_count)
        .map_err(|_| RecoveryError::FrameDimensionOverflow { run: run_id })?;

    // Apply snapshot slots/taint to frame
    for entry in &snapshot_slots {
        frame
            .write_slot_with_taint(entry.slot, entry.value, entry.taint)
            .map_err(|_| RecoveryError::ReplayDivergence {
                step: vb_core::StepIdx::ZERO,
                detail: "snapshot slot write out of bounds".to_owned(),
            })?;
    }

    // Apply tail events
    let mut tracker = ActionReplayTracker::new();
    let executed = apply_tail_events(&mut frame, tail_events, &mut tracker)
        .map_err(|_| RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: "tail event application failed".to_owned(),
        })?;

    // Set executed counter (tail events applied)
    for _ in 0..executed {
        frame
            .increment_executed()
            .map_err(|_| RecoveryError::FrameDimensionOverflow { run: run_id })?;
    }

    Ok(frame)
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

    // Use existing seed recovery to derive dimensions and state
    let seed = crate::recovery::replay::summary::recover_runtime_frame_seed_from_events(events)?;

    if seed.step_count == 0 {
        return Err(RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: "derived step_count is zero".to_owned(),
        });
    }

    // Build frame from seed
    let mut frame =
        vb_core::RunFrame::new(run_id, seed.first_step, seed.step_count, seed.slot_count)
            .map_err(|_| RecoveryError::FrameDimensionOverflow { run: run_id })?;

    // Apply step states from seed
    for entry in &seed.steps {
        match entry.state {
            RecoveredStepState::Running => frame.mark_running(entry.step),
            RecoveredStepState::Succeeded => frame.mark_succeeded(entry.step),
            RecoveredStepState::Failed => frame.mark_failed(entry.step),
            RecoveredStepState::Waiting => frame.mark_waiting(entry.step),
            RecoveredStepState::Asking => frame.mark_asking(entry.step),
        }
        .map_err(|_| RecoveryError::ReplayDivergence {
            step: entry.step,
            detail: "seed step state transition failed".to_owned(),
        })?;
    }

    // Apply slots from seed
    for entry in &seed.slots {
        frame
            .write_slot_with_taint(entry.slot, entry.value, entry.taint)
            .map_err(|_| RecoveryError::ReplayDivergence {
                step: vb_core::StepIdx::ZERO,
                detail: "seed slot write out of bounds".to_owned(),
            })?;
    }

    // Set PC from seed
    frame
        .set_pc(seed.pc)
        .map_err(|_| RecoveryError::ReplayDivergence {
            step: seed.pc,
            detail: "seed pc out of bounds".to_owned(),
        })?;

    // Set executed counter from event count (approximate)
    let state_events = events
        .iter()
        .filter(|e| {
            matches!(
                e,
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
    let state_events = u64::try_from(state_events).unwrap_or(u64::MAX);

    for _ in 0..state_events {
        frame
            .increment_executed()
            .map_err(|_| RecoveryError::FrameDimensionOverflow { run: run_id })?;
    }

    // Compute parallel in-flight from action events
    let peak = compute_parallel_in_flight(&mut frame, events)
        .map_err(|_| RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: "parallel in-flight computation failed".to_owned(),
        })?;

    // Set max_parallel_in_flight to the observed peak
    // (RunFrame::new initializes it to u16::MAX, which would never update)
    frame.set_max_parallel_in_flight(peak);

    Ok(frame)
}
