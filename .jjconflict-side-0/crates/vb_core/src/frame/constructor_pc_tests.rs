//! RunFrame constructor and program counter tests.

use super::{RunFrame, StepState};
use crate::{CoreError, CoreResult, RunId, SlotIdx, SlotValue, StepIdx, Taint};

fn populated_frame() -> CoreResult<RunFrame> {
    let mut frame = RunFrame::new(RunId::new(47), StepIdx::new(1), 3, 3)?;
    frame.mark_running(StepIdx::new(1))?;
    frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(7), Taint::DerivedFromSecret)?;
    frame.increment_executed()?;
    frame.set_max_parallel_in_flight(5);
    frame.add_parallel_in_flight(2)?;
    Ok(frame)
}

#[test]
fn fresh_frame_exact_scalars() -> CoreResult<()> {
    let frame = RunFrame::new(RunId::new(47), StepIdx::new(1), 3, 3)?;
    assert_eq!(frame.run_id(), RunId::new(47));
    assert_eq!(frame.pc(), StepIdx::new(1));
    assert_eq!(frame.executed(), 0);
    assert_eq!(frame.step_count(), 3);
    assert_eq!(frame.slot_count(), 3);
    assert_eq!(frame.max_parallel_in_flight(), u16::MAX);
    assert_eq!(frame.parallel_in_flight(), 0);
    assert_eq!(frame.written_slot_count(), 0);
    Ok(())
}

#[test]
fn fresh_frame_exact_arrays() -> CoreResult<()> {
    let frame = RunFrame::new(RunId::new(47), StepIdx::new(1), 3, 3)?;
    let states = frame.states_snapshot();
    assert_eq!(states, vec![StepState::Pending; 3]);
    let slots = frame.slots_snapshot();
    let expected_slots: Vec<Option<SlotValue>> = vec![None; 3];
    assert_eq!(slots, expected_slots);
    let taints = frame.taint_snapshot();
    let expected_taints: Vec<Taint> = vec![Taint::Clean; 3];
    assert_eq!(taints, expected_taints);
    assert_eq!(frame.initialized_slots()?.len(), 0);
    Ok(())
}

#[test]
fn fresh_frame_zero_slots() -> CoreResult<()> {
    let frame = RunFrame::new(RunId::new(47), StepIdx::new(1), 2, 0)?;
    assert_eq!(frame.slot_count(), 0);
    let slots = frame.slots_snapshot();
    let expected_slots: Vec<Option<SlotValue>> = vec![];
    assert_eq!(slots, expected_slots);
    let taints = frame.taint_snapshot();
    let expected_taints: Vec<Taint> = vec![];
    assert_eq!(taints, expected_taints);
    let states = frame.states_snapshot();
    assert_eq!(states, vec![StepState::Pending; 2]);
    assert_eq!(frame.initialized_slots()?.len(), 0);
    Ok(())
}

#[test]
fn set_pc_updates_frame() -> CoreResult<()> {
    let frame = populated_frame()?;
    let mut expected = frame.clone();
    expected.pc = StepIdx::new(2);
    let mut actual = frame.clone();
    actual.set_pc(StepIdx::new(2))?;
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn set_pc_rejects_out_of_range() -> CoreResult<()> {
    let frame = populated_frame()?;
    let before = frame.clone();
    for bad_pc in [StepIdx::new(3), StepIdx::new(u16::MAX)] {
        let mut frame = frame.clone();
        let err = frame.set_pc(bad_pc);
        assert_eq!(err, Err(CoreError::InvalidProgramCounter { step: bad_pc }));
        assert_eq!(frame, before);
    }
    Ok(())
}
