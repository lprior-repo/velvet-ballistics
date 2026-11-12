#![forbid(unsafe_code)]

use super::accounting::apply_replay_accounting;
use super::{ensure_nonzero_step_count, increment_executed, reject_owned_continuation_events};

use crate::JournalEvent;
use crate::recovery::types::{
    RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState, RecoveryError, RecoveryFrameSeed,
    RecoveryResult,
};
use vb_core::RunId;

/// Hydrates a live RunFrame from full journal events (no snapshot).
///
/// # Errors
///
/// Returns `RecoveryError` when:
/// - Events are empty
/// - Derived dimensions are zero or overflow `u16`
/// - The recovered frame seed is cannot-resume (e.g. pending
///   actions, pending timers, pending asks, unsupported slot
///   values/taint/payloads, or missing full-RunState components
///   like workflow, store, action attempts, admission, collect
///   states, action contracts, or action ABI digests).
pub fn hydrate_run_frame_from_events(
    events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<vb_core::RunFrame> {
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run: run_id });
    }

    reject_owned_continuation_events(events)?;
    let seed = crate::recovery::replay::summary::recover_runtime_frame_seed_from_events(events)?;
    ensure_nonzero_step_count(seed.step_count)?;
    let cannot_resume = seed.cannot_resume_state();
    if !cannot_resume.is_resumable() {
        return Err(RecoveryError::UnsupportedFrameSeed {
            run: run_id,
            reason: String::from(cannot_resume.unsupported_reason()),
        });
    }
    let mut frame = build_frame_from_seed(&seed, run_id)?;
    apply_seed_step_states(&mut frame, &seed.steps)?;
    apply_seed_slots(&mut frame, &seed.slots)?;
    apply_seed_pc(&mut frame, seed.pc)?;
    let executed = apply_replay_accounting(&mut frame, events, run_id, &[])?;
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
