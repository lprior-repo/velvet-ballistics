use vb_core::{
    CoreError, RunFrame,
    ids::{RunId, SlotIdx, StepIdx},
    value::{SlotValue, Taint},
};

fn populated_frame() -> Result<RunFrame, CoreError> {
    let mut frame = RunFrame::new(RunId::new(47), StepIdx::new(0), 3, 3)?;
    frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(19), Taint::Secret)?;
    frame.write_slot_with_taint(SlotIdx::new(2), SlotValue::Bool(true), Taint::Random)?;
    frame.set_pc(StepIdx::new(2))?;
    frame.mark_succeeded(StepIdx::new(0))?;
    frame.mark_running(StepIdx::new(1))?;
    frame.increment_executed()?;
    frame.set_max_parallel_in_flight(4);
    frame.add_parallel_in_flight(2)?;
    Ok(frame)
}

#[test]
fn out_of_range_write_slot_returns_error_without_mutating_frame() -> Result<(), CoreError> {
    let mut frame = populated_frame()?;
    let before = frame.clone();
    let result = frame.write_slot(SlotIdx::new(3), SlotValue::I64(99));
    assert_eq!(
        result,
        Err(CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(3)
        })
    );
    assert_eq!(frame, before);
    Ok(())
}

#[test]
fn out_of_range_write_slot_with_taint_is_atomic() -> Result<(), CoreError> {
    let mut frame = populated_frame()?;
    let before = frame.clone();
    let result = frame.write_slot_with_taint(SlotIdx::new(3), SlotValue::I64(99), Taint::Secret);
    assert_eq!(
        result,
        Err(CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(3)
        })
    );
    assert_eq!(frame, before);
    Ok(())
}

#[test]
fn out_of_range_write_taint_returns_error_without_mutating_frame() -> Result<(), CoreError> {
    let mut frame = populated_frame()?;
    let before = frame.clone();
    let result = frame.write_taint(SlotIdx::new(3), Taint::Secret);
    assert_eq!(
        result,
        Err(CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(3)
        })
    );
    assert_eq!(frame, before);
    Ok(())
}

#[test]
fn write_taint_on_uninitialized_slot_is_atomic() -> Result<(), CoreError> {
    let mut frame = populated_frame()?;
    let before = frame.clone();
    let result = frame.write_taint(SlotIdx::new(1), Taint::Secret);
    assert_eq!(
        result,
        Err(CoreError::SlotUninitialized {
            slot: SlotIdx::new(1)
        })
    );
    assert_eq!(frame, before);
    Ok(())
}

#[test]
fn reads_distinguish_uninitialized_from_out_of_range_slots() -> Result<(), CoreError> {
    let frame = populated_frame()?;
    let uninitialized = SlotIdx::new(1);
    let outside = SlotIdx::new(3);
    assert_eq!(
        frame.read_slot(uninitialized),
        Err(CoreError::SlotUninitialized {
            slot: uninitialized
        })
    );
    assert_eq!(
        frame.read_slot(outside),
        Err(CoreError::SlotOutOfBounds { slot: outside })
    );
    assert_eq!(
        frame.read_taint(uninitialized),
        Err(CoreError::SlotUninitialized {
            slot: uninitialized
        })
    );
    assert_eq!(
        frame.read_taint(outside),
        Err(CoreError::SlotOutOfBounds { slot: outside })
    );
    Ok(())
}
