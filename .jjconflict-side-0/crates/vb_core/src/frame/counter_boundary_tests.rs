use crate::errors::{CoreError, CoreResult};
use crate::frame::RunFrame;
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};

#[test]
fn runframe_executed_counter_exact_boundary() -> CoreResult<()> {
    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;
    frame.mark_running(StepIdx::ZERO)?;
    frame.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(7), Taint::DerivedFromSecret)?;
    frame.executed = u64::MAX
        .checked_sub(1)
        .ok_or(CoreError::StepCounterOverflow)?;
    frame.increment_executed()?;
    assert_eq!(frame.executed(), u64::MAX);
    let snapshot = frame.clone();
    assert_eq!(
        frame.increment_executed(),
        Err(CoreError::StepCounterOverflow)
    );
    assert_eq!(frame, snapshot);
    Ok(())
}
