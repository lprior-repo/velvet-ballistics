#![forbid(unsafe_code)]

use vb_core::frame::{RunFrame, StepState};
use vb_storage::recovery::{
    RecoveredPendingAction, RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState,
    RecoveryFrameSeed,
};

use crate::{RuntimeError, RuntimeResult};

pub(super) fn validate_recovery_seed_shape(seed: &RecoveryFrameSeed) -> RuntimeResult<()> {
    let step_count = usize::from(seed.step_count);
    let slot_count = usize::from(seed.slot_count);
    validate_seed_dimensions(seed, step_count)?;
    validate_seed_entry_caps(seed, step_count, slot_count)?;
    validate_recovered_step_entries(&seed.steps, step_count)?;
    validate_recovered_slot_entries(&seed.slots, slot_count)?;
    validate_pending_action_entries(&seed.pending_actions, step_count)?;
    Ok(())
}

pub(super) fn hydrate_shape_checked_run_frame(seed: &RecoveryFrameSeed) -> RuntimeResult<RunFrame> {
    let mut frame = empty_recovered_frame(seed)?;
    apply_recovered_steps(&mut frame, seed)?;
    apply_recovered_slots(&mut frame, seed)?;
    apply_recovered_pc(&mut frame, seed)?;
    Ok(frame)
}

fn validate_seed_dimensions(seed: &RecoveryFrameSeed, step_count: usize) -> RuntimeResult<()> {
    if step_count == 0
        || seed.first_step.as_usize() >= step_count
        || seed.pc.as_usize() >= step_count
    {
        return Err(RuntimeError::InvalidRecoveryHydration);
    }
    Ok(())
}

fn validate_seed_entry_caps(
    seed: &RecoveryFrameSeed,
    step_count: usize,
    slot_count: usize,
) -> RuntimeResult<()> {
    if seed.steps.len() > step_count
        || seed.slots.len() > slot_count
        || seed.pending_actions.len() > step_count
    {
        return Err(RuntimeError::InvalidRecoveryHydration);
    }
    Ok(())
}

fn validate_recovered_step_entries(
    entries: &[RecoveredStepEntry],
    step_count: usize,
) -> RuntimeResult<()> {
    if entries
        .iter()
        .any(|entry| entry.step.as_usize() >= step_count)
        || has_duplicate_by(entries, |left, right| left.step == right.step)
    {
        return Err(RuntimeError::InvalidRecoveryHydration);
    }
    Ok(())
}

fn validate_recovered_slot_entries(
    entries: &[RecoveredSlotEntry],
    slot_count: usize,
) -> RuntimeResult<()> {
    if entries
        .iter()
        .any(|entry| entry.slot.as_usize() >= slot_count)
        || has_duplicate_by(entries, |left, right| left.slot == right.slot)
    {
        return Err(RuntimeError::InvalidRecoveryHydration);
    }
    Ok(())
}

fn validate_pending_action_entries(
    entries: &[RecoveredPendingAction],
    step_count: usize,
) -> RuntimeResult<()> {
    if entries
        .iter()
        .any(|entry| entry.step.as_usize() >= step_count)
        || has_duplicate_by(entries, |left, right| left.step == right.step)
    {
        return Err(RuntimeError::InvalidRecoveryHydration);
    }
    Ok(())
}

fn has_duplicate_by<T, F>(entries: &[T], same_key: F) -> bool
where
    F: Fn(&T, &T) -> bool,
{
    let mut remaining = entries;
    while let Some((first, rest)) = remaining.split_first() {
        if rest.iter().any(|candidate| same_key(first, candidate)) {
            return true;
        }
        remaining = rest;
    }
    false
}

fn empty_recovered_frame(seed: &RecoveryFrameSeed) -> RuntimeResult<RunFrame> {
    RunFrame::new(
        seed.summary.run,
        seed.first_step,
        seed.step_count,
        seed.slot_count,
    )
    .map_err(|_| RuntimeError::InvalidRecoveryHydration)
}

fn apply_recovered_steps(frame: &mut RunFrame, seed: &RecoveryFrameSeed) -> RuntimeResult<()> {
    seed.steps
        .iter()
        .try_for_each(|entry| apply_recovered_step(frame, entry.step, entry.state))
}

fn apply_recovered_slots(frame: &mut RunFrame, seed: &RecoveryFrameSeed) -> RuntimeResult<()> {
    seed.slots.iter().try_for_each(|entry| {
        frame
            .write_slot_with_taint(entry.slot, entry.value, entry.taint)
            .map_err(|_| RuntimeError::InvalidRecoveryHydration)
    })
}

fn apply_recovered_pc(frame: &mut RunFrame, seed: &RecoveryFrameSeed) -> RuntimeResult<()> {
    if seed.pc.as_usize() >= usize::from(seed.step_count) {
        return Err(RuntimeError::InvalidRecoveryHydration);
    }
    frame
        .set_pc(seed.pc)
        .map_err(|_| RuntimeError::InvalidRecoveryHydration)
}

fn apply_recovered_step(
    frame: &mut RunFrame,
    step: vb_core::StepIdx,
    state: RecoveredStepState,
) -> RuntimeResult<()> {
    match state {
        RecoveredStepState::Running => frame.mark_running(step),
        RecoveredStepState::Succeeded => frame.mark_succeeded(step),
        RecoveredStepState::Failed => frame.mark_failed(step),
        RecoveredStepState::Waiting => mark_suspended(frame, step, StepState::Waiting),
        RecoveredStepState::Asking => mark_suspended(frame, step, StepState::Asking),
        _ => return Err(RuntimeError::InvalidRecoveryHydration),
    }
    .map_err(|_| RuntimeError::InvalidRecoveryHydration)
}

fn mark_suspended(
    frame: &mut RunFrame,
    step: vb_core::StepIdx,
    state: StepState,
) -> vb_core::CoreResult<()> {
    frame.mark_running(step)?;
    match state {
        StepState::Waiting => frame.mark_waiting(step),
        StepState::Asking => frame.mark_asking(step),
        _ => Err(vb_core::CoreError::InternalInvariantViolation {
            reason: "invalid_recovered_suspend_state",
        }),
    }
}
