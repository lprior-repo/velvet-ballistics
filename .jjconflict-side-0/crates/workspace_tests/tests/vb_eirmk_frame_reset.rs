//! RunFrame reset qualification: scalar getters, reinitialize invariants, clean reset.
//!
//! This test validates that RunFrame::reinitialize correctly resets all frame
//! state to initial values and that invalid reinitialization attempts return
//! the expected errors while leaving the frame unchanged.

use vb_core::CoreResult;
use vb_core::errors::CoreError;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};

fn build_fixture() -> CoreResult<RunFrame> {
    let mut frame = RunFrame::new(RunId::new(9), StepIdx::new(1), 3, 3)?;
    frame.mark_running(StepIdx::new(1))?;
    frame.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(7), Taint::DerivedFromSecret)?;
    frame.set_max_parallel_in_flight(5);
    frame.add_parallel_in_flight(2)?;
    frame.increment_executed()?;
    Ok(frame)
}

#[test]
fn getter_run_id() -> CoreResult<()> {
    let frame = build_fixture()?;
    assert_eq!(frame.run_id(), RunId::new(9));
    Ok(())
}

#[test]
fn getter_pc() -> CoreResult<()> {
    let frame = build_fixture()?;
    assert_eq!(frame.pc(), StepIdx::new(1));
    Ok(())
}

#[test]
fn getter_executed() -> CoreResult<()> {
    let frame = build_fixture()?;
    assert_eq!(frame.executed(), 1);
    Ok(())
}

#[test]
fn getter_step_count() -> CoreResult<()> {
    let frame = build_fixture()?;
    assert_eq!(frame.step_count(), 3);
    Ok(())
}

#[test]
fn getter_slot_count() -> CoreResult<()> {
    let frame = build_fixture()?;
    assert_eq!(frame.slot_count(), 3);
    Ok(())
}

#[test]
fn getter_max_parallel_in_flight() -> CoreResult<()> {
    let frame = build_fixture()?;
    assert_eq!(frame.max_parallel_in_flight(), 5);
    Ok(())
}

#[test]
fn getter_parallel_in_flight() -> CoreResult<()> {
    let frame = build_fixture()?;
    assert_eq!(frame.parallel_in_flight(), 2);
    Ok(())
}

#[test]
fn valid_reset_restores_all_defaults() -> CoreResult<()> {
    let mut frame = build_fixture()?;
    frame.reinitialize(RunId::new(99), StepIdx::new(0), 3, 3)?;
    let fresh = RunFrame::new(RunId::new(99), StepIdx::new(0), 3, 3)?;
    assert_eq!(frame, fresh);
    Ok(())
}

fn reinitialize_negative_case(
    entry: StepIdx,
    steps: u16,
    slots: u16,
    expected: CoreError,
) -> CoreResult<()> {
    let original = build_fixture()?;
    let mut frame = original.clone();
    assert_eq!(
        frame.reinitialize(RunId::new(99), entry, steps, slots),
        Err(expected)
    );
    assert_eq!(frame, original);
    Ok(())
}

#[test]
fn reinitialize_zero_steps_errors() -> CoreResult<()> {
    reinitialize_negative_case(
        StepIdx::new(0),
        0,
        3,
        CoreError::InvalidCompiledWorkflow {
            reason: "step_count_zero",
        },
    )
}

#[test]
fn reinitialize_first_step_at_count_errors() -> CoreResult<()> {
    reinitialize_negative_case(
        StepIdx::new(3),
        3,
        3,
        CoreError::InvalidProgramCounter {
            step: StepIdx::new(3),
        },
    )
}

#[test]
fn reinitialize_changed_step_count_errors() -> CoreResult<()> {
    reinitialize_negative_case(
        StepIdx::new(0),
        4,
        3,
        CoreError::InvalidCompiledWorkflow {
            reason: "frame_dimension_mismatch",
        },
    )
}

#[test]
fn reinitialize_changed_slot_count_errors() -> CoreResult<()> {
    reinitialize_negative_case(
        StepIdx::new(0),
        3,
        4,
        CoreError::InvalidCompiledWorkflow {
            reason: "frame_dimension_mismatch",
        },
    )
}

#[test]
fn reinitialize_step_count_zero_takes_precedence() -> CoreResult<()> {
    reinitialize_negative_case(
        StepIdx::new(3),
        0,
        4,
        CoreError::InvalidCompiledWorkflow {
            reason: "step_count_zero",
        },
    )
}

#[test]
fn reinitialize_first_step_at_count_takes_precedence_over_dimension() -> CoreResult<()> {
    reinitialize_negative_case(
        StepIdx::new(4),
        4,
        3,
        CoreError::InvalidProgramCounter {
            step: StepIdx::new(4),
        },
    )
}
