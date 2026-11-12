#![forbid(unsafe_code)]
//! Bounded replay claims use the complete production ChooseSlot helper; native tests bridge public dispatch.
use super::{ReplayAction, ReplayError};
use crate::frame::RunFrame;
#[cfg(test)]
use crate::ids::RunId;
use crate::ids::StepIdx;
use crate::value::SlotValue;
use crate::workflow::SlotBranch;
#[path = "replay_kani_shapes.rs"]
mod shapes;
use shapes::Shape;
#[cfg(kani)]
use shapes::materialize;
#[cfg(test)]
fn frame(shape: Shape, a: bool, b: bool) -> Result<RunFrame, crate::CoreError> {
    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::ZERO,
        shape.node_count(),
        shape.slot_count(),
    )?;
    run.write_slot(shape.root(false), SlotValue::Bool(a))?;
    run.write_slot(shape.root(true), SlotValue::Bool(b))?;
    Ok(run)
}
fn outcome(result: Result<ReplayAction, ReplayError>) -> Result<StepIdx, ReplayError> {
    match result {
        Ok(ReplayAction::Continue(target)) => Ok(target),
        Err(error) => Err(error),
        _ => Err(ReplayError::Internal {
            reason: "unexpected action variant",
        }),
    }
}
#[cfg(test)]
fn replay_frame(
    shape: Shape,
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
    a: bool,
    b: bool,
) -> Result<(Result<StepIdx, ReplayError>, RunFrame), crate::CoreError> {
    let mut run = frame(shape, a, b)?;
    let mut expected = run.clone();
    let expected_result = shape.expected(a, b);
    if let Ok(target) = expected_result {
        expected.set_pc(target)?;
        expected.increment_executed()?;
    }
    let result = outcome(super::choose::replay_choose_slot(
        &mut run, branches, otherwise,
    ));
    assert_eq!(run.executed(), u64::from(expected_result.is_ok()));
    assert_eq!(run, expected);
    Ok((result, run))
}
#[cfg(kani)]
fn with_replay(
    shape: Shape,
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
    a: bool,
    b: bool,
    mut check: impl FnMut(Result<StepIdx, ReplayError>, &RunFrame),
) {
    let checkpoint = shape.checkpoint(a, b);
    assert_eq!(
        checkpoint
            .slots
            .get(shape.root(false).as_usize())
            .map(|value| matches!(value, Some(SlotValue::Bool(value)) if *value == a)),
        Some(true)
    );
    assert_eq!(
        checkpoint
            .slots
            .get(shape.root(true).as_usize())
            .map(|value| matches!(value, Some(SlotValue::Bool(value)) if *value == b)),
        Some(true)
    );
    let mut construction = RunFrame::from_checkpoint_v1(checkpoint);
    assert_eq!(construction.as_ref().map(|_| ()), Ok(()));
    let retained = if let Ok(run) = &mut construction {
        let result = outcome(super::choose::replay_choose_slot(run, branches, otherwise));
        check(result, run);
        Some(())
    } else {
        None
    };
    assert_eq!(retained.as_ref().map(|_| ()), Some(()));
    // Owned setup-result destruction is outside the replay-only claim boundary.
    std::mem::forget((construction, retained));
}
#[cfg(kani)]
mod verification {
    use super::*;
    #[kani::proof]
    #[kani::unwind(5)]
    fn verify_replay_choose_slot_two_branches_no_panic() {
        let shape: Shape = kani::any();
        let a: bool = kani::any();
        let b: bool = kani::any();
        materialize(shape, |shape, branches, otherwise| {
            with_replay(shape, branches, otherwise, a, b, |result, run| {
                assert_eq!(result, shape.expected(a, b));
                match &result {
                    Ok(target) => assert_eq!(run.pc(), *target),
                    Err(ReplayError::Internal { reason }) => {
                        assert_eq!(*reason, "choose_slot no branch matched and no otherwise")
                    }
                    _ => assert_eq!(result, shape.expected(a, b)),
                }
                kani::cover!(
                    shape.two_branches && result.is_ok(),
                    "two-branch replay reached"
                );
                kani::cover!(
                    !shape.two_branches && result.is_ok(),
                    "one-branch replay reached"
                );
                kani::cover!(
                    shape.otherwise && !a && !b && result.is_ok(),
                    "otherwise replay reached"
                );
                kani::cover!(
                    !shape.otherwise && !a && !b && result.is_err(),
                    "typed no-match error reached"
                );
            });
        });
    }
    #[kani::proof]
    #[kani::unwind(5)]
    fn verify_choose_slot_output_in_input_set() {
        let shape: Shape = kani::any();
        let a: bool = kani::any();
        let b: bool = kani::any();
        materialize(shape, |shape, branches, otherwise| {
            with_replay(shape, branches, otherwise, a, b, |result, run| {
                assert_eq!(result, shape.expected(a, b));
                match &result {
                    Ok(target) => {
                        let valid = *target == StepIdx::new(1)
                            || *target == StepIdx::new(2)
                            || *target == StepIdx::new(3);
                        assert!(valid);
                        assert!(target.get() <= shape.branch_count() + 1);
                        assert_eq!(run.pc(), *target);
                    }
                    Err(_) => assert_eq!(result, shape.expected(a, b)),
                }
                kani::cover!(a, "first branch replay reached");
                kani::cover!(
                    !a && b && shape.two_branches,
                    "second branch replay reached"
                );
                kani::cover!(
                    shape.otherwise && !a && !b && result.is_ok(),
                    "fallback replay reached"
                );
                kani::cover!(
                    shape.shifted_roots && shape.reverse_roots,
                    "shifted reversed roots reached"
                );
            });
        });
    }
    #[kani::proof]
    #[kani::unwind(5)]
    fn verify_replay_deterministic_for_same_input() {
        let shape: Shape = kani::any();
        let a: bool = kani::any();
        let b: bool = kani::any();
        materialize(shape, |shape, branches, otherwise| {
            with_replay(shape, branches, otherwise, a, b, |result_a, run_a| {
                with_replay(shape, branches, otherwise, a, b, |result_b, run_b| {
                    assert_eq!(result_a, shape.expected(a, b));
                    assert_eq!(result_a, result_b);
                    assert_eq!(run_a.pc(), run_b.pc());
                    match (&result_a, &result_b) {
                        (Ok(a), Ok(b)) => assert_eq!(a, b),
                        _ => assert_eq!(result_a, shape.expected(a, b)),
                    }
                    kani::cover!(
                        a && shape.two_branches,
                        "deterministic two-branch success reached"
                    );
                    kani::cover!(
                        shape.otherwise && !a && !b && !shape.two_branches && result_a.is_ok(),
                        "deterministic one-branch fallback reached"
                    );
                });
            });
        });
    }
}
#[cfg(test)]
#[path = "replay_kani_bridge_tests.rs"]
mod fixture_tests;
