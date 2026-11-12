use crate::frame::{RunFrame, SlotIdx, SlotValue, StepIdx, StepState, Taint};
use crate::ids::RunId;

fn write(frame: &mut RunFrame, index: u16, value: SlotValue, taint: Taint) -> Result<(), String> {
    frame
        .write_slot_with_taint(SlotIdx::new(index), value, taint)
        .map_err(|e| e.to_string())
}

fn sparse_frame() -> Result<RunFrame, String> {
    let mut frame = RunFrame::new(RunId::new(2), StepIdx::ZERO, 3, 5).map_err(|e| e.to_string())?;
    write(
        &mut frame,
        4,
        SlotValue::Bool(true),
        Taint::DerivedFromSecret,
    )?;
    write(&mut frame, 1, SlotValue::I64(17), Taint::Clean)?;
    write(&mut frame, 3, SlotValue::Null, Taint::DerivedFromSecret)?;
    Ok(frame)
}

fn snapshot_frame() -> Result<RunFrame, String> {
    let mut frame = RunFrame::new(RunId::new(3), StepIdx::ZERO, 3, 3).map_err(|e| e.to_string())?;
    write(
        &mut frame,
        2,
        SlotValue::Bool(true),
        Taint::DerivedFromSecret,
    )?;
    write(&mut frame, 0, SlotValue::I64(8), Taint::Clean)?;
    frame
        .mark_running(StepIdx::ZERO)
        .map_err(|e| e.to_string())?;
    Ok(frame)
}

fn expected_slots() -> Vec<Option<SlotValue>> {
    vec![Some(SlotValue::I64(8)), None, Some(SlotValue::Bool(true))]
}

fn expected_taints() -> Vec<Taint> {
    vec![Taint::Clean, Taint::Clean, Taint::DerivedFromSecret]
}

fn expected_states() -> Vec<StepState> {
    vec![StepState::Running, StepState::Pending, StepState::Pending]
}

fn assert_frame_state(frame: &RunFrame) -> Result<(), String> {
    assert_eq!(frame.slots_snapshot(), expected_slots());
    assert_eq!(frame.taint_snapshot(), expected_taints());
    assert_eq!(frame.states_snapshot(), expected_states());
    assert_eq!(
        frame.initialized_slots().map_err(|e| e.to_string())?,
        vec![
            (SlotIdx::ZERO, SlotValue::I64(8), Taint::Clean),
            (
                SlotIdx::new(2),
                SlotValue::Bool(true),
                Taint::DerivedFromSecret
            )
        ]
    );
    Ok(())
}

#[test]
fn empty_frame_has_no_initialized_slots_or_written_count() -> Result<(), String> {
    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 4).map_err(|e| e.to_string())?;
    assert_eq!(
        frame.initialized_slots().map_err(|e| e.to_string())?,
        Vec::new()
    );
    assert_eq!(frame.written_slot_count(), 0);
    assert_eq!(frame.slots_snapshot(), vec![None; 4]);
    assert_eq!(frame.taint_snapshot(), vec![Taint::Clean; 4]);
    assert_eq!(frame.states_snapshot(), vec![StepState::Pending; 2]);
    Ok(())
}

#[test]
fn sparse_writes_enumerate_in_order_and_overwrite_without_counting_twice() -> Result<(), String> {
    let mut frame = sparse_frame()?;
    assert_eq!(frame.written_slot_count(), 3);
    write(&mut frame, 1, SlotValue::I64(23), Taint::Clean)?;
    assert_eq!(
        frame.initialized_slots().map_err(|e| e.to_string())?,
        vec![
            (SlotIdx::new(1), SlotValue::I64(23), Taint::Clean),
            (SlotIdx::new(3), SlotValue::Null, Taint::DerivedFromSecret),
            (
                SlotIdx::new(4),
                SlotValue::Bool(true),
                Taint::DerivedFromSecret
            )
        ]
    );
    assert_eq!(frame.written_slot_count(), 3);
    Ok(())
}

#[test]
fn snapshots_are_exact_copies_independent_of_frame_state() -> Result<(), String> {
    let frame = snapshot_frame()?;
    let before = frame.clone();
    let mut slots = frame.slots_snapshot();
    let mut taints = frame.taint_snapshot();
    let mut states = frame.states_snapshot();
    assert_eq!(slots, expected_slots());
    assert_eq!(taints, expected_taints());
    assert_eq!(states, expected_states());
    assert_eq!((slots.len(), taints.len(), states.len()), (3, 3, 3));
    *slots.get_mut(2).ok_or("missing slot copy")? = Some(SlotValue::Null);
    *taints.get_mut(2).ok_or("missing taint copy")? = Taint::Clean;
    *states.get_mut(0).ok_or("missing state copy")? = StepState::Failed;
    assert_frame_state(&frame)?;
    assert_eq!(frame, before);
    Ok(())
}
