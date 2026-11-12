use crate::errors::{CoreError, CoreResult};
use crate::frame::{RunFrame, StepState, is_valid_step_state_transition};
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};

const STATES: [StepState; 8] = [
    StepState::Pending,
    StepState::Running,
    StepState::Succeeded,
    StepState::Failed,
    StepState::Skipped,
    StepState::Waiting,
    StepState::Asking,
    StepState::Cancelled,
];

fn expected_v1_transition(source: StepState, target: StepState) -> bool {
    source == target
        || match source {
            StepState::Pending => matches!(
                target,
                StepState::Running
                    | StepState::Succeeded
                    | StepState::Failed
                    | StepState::Cancelled
                    | StepState::Skipped
            ),
            StepState::Running => matches!(
                target,
                StepState::Succeeded
                    | StepState::Failed
                    | StepState::Waiting
                    | StepState::Asking
                    | StepState::Cancelled
                    | StepState::Skipped
            ),
            StepState::Waiting | StepState::Asking => target == StepState::Running,
            StepState::Succeeded => target == StepState::Pending,
            StepState::Failed | StepState::Skipped | StepState::Cancelled => false,
        }
}

fn set_state(frame: &mut RunFrame, step: StepIdx, state: StepState) -> CoreResult<()> {
    *frame
        .states
        .get_mut(step.as_usize())
        .ok_or(CoreError::StepStateOutOfBounds { step })? = state;
    Ok(())
}

fn mark_state(frame: &mut RunFrame, step: StepIdx, target: StepState) -> CoreResult<()> {
    match target {
        StepState::Pending => frame.mark_pending(step),
        StepState::Running => frame.mark_running(step),
        StepState::Succeeded => frame.mark_succeeded(step),
        StepState::Failed => frame.mark_failed(step),
        StepState::Skipped => frame.mark_skipped(step),
        StepState::Waiting => frame.mark_waiting(step),
        StepState::Asking => frame.mark_asking(step),
        StepState::Cancelled => frame.mark_cancelled(step),
    }
}

fn populated_frame() -> CoreResult<RunFrame> {
    let mut frame = RunFrame::new(RunId::new(9), StepIdx::ZERO, 2, 2)?;
    frame.set_pc(StepIdx::new(1))?;
    frame.increment_executed()?;
    frame.set_max_parallel_in_flight(3);
    frame.add_parallel_in_flight(1)?;
    frame.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(11), Taint::Clean)?;
    frame.write_slot_with_taint(
        SlotIdx::new(1),
        SlotValue::Bool(true),
        Taint::DerivedFromSecret,
    )?;
    set_state(&mut frame, StepIdx::new(1), StepState::Running)?;
    Ok(frame)
}

fn exercise_pair(frame: &mut RunFrame, source: StepState, target: StepState) -> CoreResult<()> {
    set_state(frame, StepIdx::ZERO, source)?;
    let before = frame.clone();
    let expected = expected_v1_transition(source, target);
    assert_eq!(is_valid_step_state_transition(source, target), expected);
    let result = mark_state(frame, StepIdx::ZERO, target);
    if expected {
        assert_eq!(result, Ok(()));
        assert_eq!(frame.step_state(StepIdx::ZERO), Ok(target));
        let mut expected_frame = before;
        set_state(&mut expected_frame, StepIdx::ZERO, target)?;
        assert_eq!(*frame, expected_frame);
    } else {
        assert_eq!(
            result,
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition".into()
            })
        );
        assert_eq!(*frame, before);
    }
    Ok(())
}

#[test]
fn all_current_state_pairs_match_v1_contract_through_public_mark_methods() -> CoreResult<()> {
    let mut frame = populated_frame()?;
    for source in STATES {
        for target in STATES {
            exercise_pair(&mut frame, source, target)?;
        }
    }
    Ok(())
}

#[test]
fn every_public_mark_method_rejects_out_of_bounds_without_mutating_frame() -> CoreResult<()> {
    let mut frame = populated_frame()?;
    let invalid_step = StepIdx::new(frame.step_count());
    let before = frame.clone();
    for target in STATES {
        assert_eq!(
            mark_state(&mut frame, invalid_step, target),
            Err(CoreError::StepStateOutOfBounds { step: invalid_step })
        );
        assert_eq!(frame, before);
    }
    Ok(())
}
