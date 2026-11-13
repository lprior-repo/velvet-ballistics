use super::{first_success_can_continue, jump};
use crate::frame::RunFrame;
use crate::ids::{RunId, StepIdx};
use crate::replay::ReplayError;
use crate::replay::step::types::ReplayAction;

pub(super) fn eval_node() -> crate::workflow::CompiledNode {
    crate::workflow::node(
        StepIdx::new(3),
        crate::workflow::CompiledNodeKind::EvalExpr {
            expr: crate::ids::ExprIdx::new(0),
        },
        None,
        None,
    )
}

#[test]
fn first_success_requires_marker_to_match_the_failing_eval_node() {
    use crate::workflow::{CompiledNodeKind as K, node};
    let error = ReplayError::ExpressionEvalFailed {
        step: StepIdx::new(3),
    };
    let mismatched = node(
        StepIdx::new(4),
        K::EvalExpr {
            expr: crate::ids::ExprIdx::new(0),
        },
        None,
        None,
    );
    let choose = node(
        StepIdx::new(3),
        K::Choose {
            branches: Box::new([]),
            otherwise: None.into(),
        },
        None,
        None,
    );
    assert!(!first_success_can_continue(Some(&mismatched), &error));
    assert!(!first_success_can_continue(Some(&choose), &error));
    assert!(!first_success_can_continue(None, &error));
}

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
