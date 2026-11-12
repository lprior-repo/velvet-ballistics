use super::frame_seed_core::{
    RunFrame, RunId, SlotValue, StepIdx, Taint,
    frame::{CheckpointStepState, RunFrameCheckpointV1, RunFrameCheckpointV1Error},
};

pub(super) fn checked_frame_from_seed(
    run_id: RunId,
    pc: StepIdx,
    step_count: u16,
    entries: impl IntoIterator<Item = (SlotValue, Taint)>,
) -> Result<RunFrame, RunFrameCheckpointV1Error> {
    let (slots, taint) = entries
        .into_iter()
        .map(|(value, taint)| (Some(value), taint))
        .unzip();
    RunFrame::from_checkpoint_v1(RunFrameCheckpointV1 {
        run_id,
        pc,
        executed: 0,
        max_parallel_in_flight: u16::MAX,
        parallel_in_flight: 0,
        states: vec![CheckpointStepState::Pending; usize::from(step_count)],
        slots,
        taint,
    })
}
