use super::jump;
use crate::frame::RunFrame;
use crate::ids::{RunId, StepIdx};
use crate::replay::ReplayError;
use crate::replay::step::types::ReplayAction;

#[test]
fn concurrent_jump_rejects_target_outside_bounded_frame() -> Result<(), crate::errors::CoreError> {
    let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, 0)?;
    let target = StepIdx::new(1);
    assert!(matches!(
        jump(&mut run, target),
        Err(ReplayError::StepNotFound { step }) if step == target
    ));
    assert_eq!(run.pc(), StepIdx::ZERO);
    Ok(())
}

#[test]
fn concurrent_jump_updates_pc_for_target_inside_bounded_frame()
-> Result<(), crate::errors::CoreError> {
    let mut run = RunFrame::new(RunId::new(2), StepIdx::ZERO, 2, 0)?;
    let target = StepIdx::new(1);
    assert!(matches!(
        jump(&mut run, target),
        Ok(ReplayAction::Continue(step)) if step == target
    ));
    assert_eq!(run.pc(), target);
    Ok(())
}
