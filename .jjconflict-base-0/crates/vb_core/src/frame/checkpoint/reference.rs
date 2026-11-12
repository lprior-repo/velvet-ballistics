use super::{RunFrame, RunFrameCheckpointV1, RunFrameCheckpointV1Error, StepState, Taint};

// Native characterization oracle: the previous checked factory and its real constructor.
pub(super) fn via_new_and_seed(
    checkpoint: RunFrameCheckpointV1,
) -> Result<RunFrame, RunFrameCheckpointV1Error> {
    let step_count = u16::try_from(checkpoint.states.len())
        .map_err(|_| RunFrameCheckpointV1Error::StepCountOverflow)?;
    let slot_count = u16::try_from(checkpoint.slots.len())
        .map_err(|_| RunFrameCheckpointV1Error::SlotCountOverflow)?;
    if step_count == 0 {
        return Err(RunFrameCheckpointV1Error::EmptySteps);
    }
    if checkpoint.slots.len() != checkpoint.taint.len() {
        return Err(RunFrameCheckpointV1Error::SlotTaintLengthMismatch);
    }
    if checkpoint
        .slots
        .iter()
        .zip(&checkpoint.taint)
        .any(|(value, taint)| value.is_none() && *taint != Taint::Clean)
    {
        return Err(RunFrameCheckpointV1Error::TaintWithoutValue);
    }
    if checkpoint.parallel_in_flight > checkpoint.max_parallel_in_flight {
        return Err(RunFrameCheckpointV1Error::ParallelCountOutOfBounds);
    }

    let mut frame = RunFrame::new(checkpoint.run_id, checkpoint.pc, step_count, slot_count)
        .map_err(|_| RunFrameCheckpointV1Error::InvalidFrame)?;
    frame.executed = checkpoint.executed;
    frame.max_parallel_in_flight = checkpoint.max_parallel_in_flight;
    frame.parallel_in_flight = checkpoint.parallel_in_flight;
    for (target, state) in frame.states.iter_mut().zip(checkpoint.states) {
        *target = StepState::from(state);
    }
    for (cell, (value, taint)) in frame
        .cells
        .iter_mut()
        .zip(checkpoint.slots.into_iter().zip(checkpoint.taint))
    {
        *cell = match value {
            Some(value) => crate::value::SlotCell::value(value, taint),
            None => crate::value::SlotCell::empty(),
        };
    }
    Ok(frame)
}
