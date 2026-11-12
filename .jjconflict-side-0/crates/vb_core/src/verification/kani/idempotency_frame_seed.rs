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

#[cfg(kani)]
fn symbolic_entries<const N: usize>() -> [(SlotValue, Taint); N] {
    core::array::from_fn(|_| (kani::any::<SlotValue>(), kani::any::<Taint>()))
}

#[cfg(kani)]
pub(super) fn partitioned_frame(
    run_id: RunId,
    pc: StepIdx,
    step_count: u16,
    slot_count: u16,
) -> Result<RunFrame, RunFrameCheckpointV1Error> {
    kani::cover!(slot_count == 1, "one-slot frame generated");
    kani::cover!(slot_count == 2, "two-slot frame generated");
    kani::cover!(slot_count == 3, "three-slot frame generated");
    kani::cover!(slot_count == 4, "four-slot frame generated");
    let result = match slot_count {
        1 => checked_frame_from_seed(run_id, pc, step_count, symbolic_entries::<1>()),
        2 => checked_frame_from_seed(run_id, pc, step_count, symbolic_entries::<2>()),
        3 => checked_frame_from_seed(run_id, pc, step_count, symbolic_entries::<3>()),
        4 => checked_frame_from_seed(run_id, pc, step_count, symbolic_entries::<4>()),
        _ => {
            kani::assert(false, "slot-count partition covers the original bound");
            return Err(RunFrameCheckpointV1Error::InvalidFrame);
        }
    };
    if let Ok(frame) = &result {
        kani::assert(frame.run_id() == run_id, "generated run ID is preserved");
        kani::assert(frame.pc() == pc, "generated PC is preserved");
        kani::assert(
            frame.step_count() == step_count,
            "step dimension is preserved",
        );
        kani::assert(
            frame.slot_count() == slot_count,
            "slot dimension is preserved",
        );
    }
    result
}
