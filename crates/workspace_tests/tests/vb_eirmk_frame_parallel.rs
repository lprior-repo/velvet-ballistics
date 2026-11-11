use vb_core::{CoreError, CoreResult, RunFrame, RunId, SlotIdx, SlotValue, StepIdx, Taint};

fn populated_frame() -> CoreResult<RunFrame> {
    let mut frame = RunFrame::new(RunId::new(17), StepIdx::ZERO, 3, 3)?;
    frame.set_pc(StepIdx::new(2))?;
    frame.mark_running(StepIdx::ZERO)?;
    frame.mark_succeeded(StepIdx::new(1))?;
    frame.increment_executed()?;
    frame.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(41), Taint::Clean)?;
    frame.write_slot_with_taint(
        SlotIdx::new(2),
        SlotValue::Bool(true),
        Taint::DerivedFromSecret,
    )?;
    frame.set_max_parallel_in_flight(5);
    frame.add_parallel_in_flight(2)?;
    Ok(frame)
}

#[test]
fn zero_limit_allows_zero_and_rejects_one_atomically() -> CoreResult<()> {
    let mut frame = populated_frame()?;
    frame.set_max_parallel_in_flight(0);
    frame.sub_parallel_in_flight(2)?;
    let before = frame.clone();
    assert_eq!(frame.add_parallel_in_flight(0), Ok(()));
    assert_eq!(frame, before);
    assert_eq!(
        frame.add_parallel_in_flight(1),
        Err(CoreError::ParallelLimitExceeded { limit: 0 })
    );
    assert_eq!(frame, before);
    Ok(())
}

#[test]
fn lowering_limit_preserves_live_count_and_zero_add_interaction() -> CoreResult<()> {
    let mut frame = populated_frame()?;
    frame.set_max_parallel_in_flight(1);
    assert_eq!(frame.parallel_in_flight(), 2);
    let before_rejection = frame.clone();
    assert_eq!(
        frame.add_parallel_in_flight(0),
        Err(CoreError::ParallelLimitExceeded { limit: 1 })
    );
    assert_eq!(frame, before_rejection);
    frame.sub_parallel_in_flight(1)?;
    assert_eq!(frame.add_parallel_in_flight(0), Ok(()));
    assert_eq!(frame.parallel_in_flight(), 1);
    Ok(())
}

#[test]
fn overflow_error_preserves_complete_populated_frame() -> CoreResult<()> {
    let mut frame = populated_frame()?;
    frame.set_max_parallel_in_flight(u16::MAX);
    frame.sub_parallel_in_flight(2)?;
    frame.add_parallel_in_flight(u16::MAX)?;
    let before = frame.clone();
    assert_eq!(
        frame.add_parallel_in_flight(1),
        Err(CoreError::InternalInvariantViolation {
            reason: "parallel_in_flight overflow"
        })
    );
    assert_eq!(frame, before);
    Ok(())
}

#[test]
fn underflow_error_preserves_complete_populated_frame() -> CoreResult<()> {
    let mut frame = populated_frame()?;
    let before = frame.clone();
    assert_eq!(
        frame.sub_parallel_in_flight(3),
        Err(CoreError::InternalInvariantViolation {
            reason: "parallel_in_flight underflow"
        })
    );
    assert_eq!(frame, before);
    Ok(())
}
