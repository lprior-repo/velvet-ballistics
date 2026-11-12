use vb_core::frame::{CheckpointStepState, RunFrameCheckpointV1, RunFrameCheckpointV1Error};
use vb_core::{CoreResult, RunFrame, RunId, SlotIdx, SlotValue, StepIdx, Taint};

fn valid_base_dto() -> CoreResult<RunFrameCheckpointV1> {
    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;
    Ok(frame.checkpoint_v1())
}

fn populated_frame() -> CoreResult<RunFrame> {
    let mut frame = RunFrame::new(RunId::new(9), StepIdx::new(1), 2, 2)?;
    frame.mark_running(StepIdx::new(1))?;
    frame.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(7), Taint::DerivedFromSecret)?;
    frame.increment_executed()?;
    frame.set_max_parallel_in_flight(5);
    frame.add_parallel_in_flight(2)?;
    Ok(frame)
}

#[test]
fn test_happy_path_restore() -> Result<(), String> {
    // Given: a populated frame with nondefault execution and slot state.
    let frame = populated_frame().map_err(|error| format!("frame setup: {error:?}"))?;
    // When: its checkpoint is restored through the public constructor.
    let restored = RunFrame::from_checkpoint_v1(frame.checkpoint_v1())
        .map_err(|error| format!("checkpoint restore: {error:?}"))?;
    // Then: program counter and every frame field are restored exactly.
    assert_eq!(restored.run_id(), frame.run_id());
    assert_eq!(restored.pc(), frame.pc());
    assert_eq!(restored.step_count(), frame.step_count());
    assert_eq!(restored.slot_count(), frame.slot_count());
    assert_eq!(restored, frame);
    Ok(())
}

#[test]
fn test_step_count_overflow() -> CoreResult<()> {
    // Given: a checkpoint with 65536 step states.
    let mut dto = valid_base_dto()?;
    dto.states = vec![CheckpointStepState::Pending; 65536];
    // When: the checkpoint is restored.
    let result = RunFrame::from_checkpoint_v1(dto);
    // Then: step-count conversion fails with its exact error.
    assert_eq!(result, Err(RunFrameCheckpointV1Error::StepCountOverflow));
    Ok(())
}

#[test]
fn test_slot_count_overflow() -> CoreResult<()> {
    // Given: a checkpoint with 65536 slots.
    let mut dto = valid_base_dto()?;
    dto.slots = vec![None; 65536];
    // When: the checkpoint is restored.
    let result = RunFrame::from_checkpoint_v1(dto);
    // Then: slot-count conversion fails with its exact error.
    assert_eq!(result, Err(RunFrameCheckpointV1Error::SlotCountOverflow));
    Ok(())
}

#[test]
fn test_empty_steps() -> CoreResult<()> {
    // Given: a checkpoint with no step states.
    let mut dto = valid_base_dto()?;
    dto.states.clear();
    // When: the checkpoint is restored.
    let result = RunFrame::from_checkpoint_v1(dto);
    // Then: restoration reports the exact empty-step error.
    assert_eq!(result, Err(RunFrameCheckpointV1Error::EmptySteps));
    Ok(())
}

#[test]
fn test_slot_taint_length_mismatch() -> CoreResult<()> {
    // Given: a checkpoint whose slot and taint arrays have different lengths.
    let mut dto = valid_base_dto()?;
    dto.taint.clear();
    // When: the checkpoint is restored.
    let result = RunFrame::from_checkpoint_v1(dto);
    // Then: restoration reports the exact length-mismatch error.
    assert_eq!(
        result,
        Err(RunFrameCheckpointV1Error::SlotTaintLengthMismatch)
    );
    Ok(())
}

#[test]
fn test_taint_without_value() -> CoreResult<()> {
    // Given: an uninitialized slot carrying secret-derived taint.
    let mut dto = valid_base_dto()?;
    dto.slots = vec![None; 2];
    dto.taint = vec![Taint::DerivedFromSecret; 2];
    // When: the checkpoint is restored.
    let result = RunFrame::from_checkpoint_v1(dto);
    // Then: restoration reports the exact taint error.
    assert_eq!(result, Err(RunFrameCheckpointV1Error::TaintWithoutValue));
    Ok(())
}

#[test]
fn test_parallel_count_out_of_bounds() -> CoreResult<()> {
    // Given: the in-flight count exceeds its configured maximum.
    let mut dto = valid_base_dto()?;
    dto.parallel_in_flight = 2;
    dto.max_parallel_in_flight = 1;
    // When: the checkpoint is restored.
    let result = RunFrame::from_checkpoint_v1(dto);
    // Then: restoration reports the exact parallel-count error.
    assert_eq!(
        result,
        Err(RunFrameCheckpointV1Error::ParallelCountOutOfBounds)
    );
    Ok(())
}

#[test]
fn test_invalid_frame_pc() -> CoreResult<()> {
    // Given: the program counter is outside the checkpoint's two-step frame.
    let mut dto = valid_base_dto()?;
    dto.pc = StepIdx::new(2);
    // When: the checkpoint is restored.
    let result = RunFrame::from_checkpoint_v1(dto);
    // Then: frame construction reports the exact invalid-frame error.
    assert_eq!(result, Err(RunFrameCheckpointV1Error::InvalidFrame));
    Ok(())
}

#[test]
fn test_precedence_slot_count_before_empty_steps() -> CoreResult<()> {
    // Given: both step states are empty and slot length is 65536.
    let mut dto = valid_base_dto()?;
    dto.states.clear();
    dto.slots = vec![None; 65536];
    // When: the checkpoint is restored.
    let result = RunFrame::from_checkpoint_v1(dto);
    // Then: slot-count overflow precedes empty-step validation.
    assert_eq!(result, Err(RunFrameCheckpointV1Error::SlotCountOverflow));
    Ok(())
}

#[test]
fn test_precedence_parallel_before_pc() -> CoreResult<()> {
    // Given: both the parallel count and program counter are invalid.
    let mut dto = valid_base_dto()?;
    dto.parallel_in_flight = 2;
    dto.max_parallel_in_flight = 1;
    dto.pc = StepIdx::new(2);
    // When: the checkpoint is restored.
    let result = RunFrame::from_checkpoint_v1(dto);
    // Then: parallel-count validation precedes frame and PC validation.
    assert_eq!(
        result,
        Err(RunFrameCheckpointV1Error::ParallelCountOutOfBounds)
    );
    Ok(())
}
