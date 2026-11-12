use super::{RunFrame, StepState, read_taint_cell, write_taint_cell};
use crate::value::SlotCell;
use crate::{
    BlobId, CoreError, CoreResult, FiniteF64, ListId, ObjectId, RunId, SlotIdx, SlotValue, StepIdx,
    SymbolId, Taint,
};

const TAINTS: [Taint; 5] = [
    Taint::Clean,
    Taint::DerivedFromSecret,
    Taint::Secret,
    Taint::Random,
    Taint::TimeDependent,
];

fn hostile_frame(slots: u16, _markers: u16) -> CoreResult<RunFrame> {
    let mut frame = RunFrame::new(RunId::new(91), StepIdx::new(1), 3, slots)?;
    frame.mark_succeeded(StepIdx::ZERO)?;
    frame.mark_running(StepIdx::new(1))?;
    frame.increment_executed()?;
    frame.set_max_parallel_in_flight(37);
    frame.add_parallel_in_flight(3)?;
    Ok(frame)
}

fn expected_read(frame: &RunFrame, slot: SlotIdx) -> CoreResult<Taint> {
    match frame.cells.get(slot.as_usize()) {
        None => Err(CoreError::SlotOutOfBounds { slot }),
        Some(SlotCell::Empty) => Err(CoreError::SlotUninitialized { slot }),
        Some(SlotCell::Value { taint, .. }) => Ok(*taint),
    }
}

fn check_cell(frame: &mut RunFrame, slot: SlotIdx, requested: Taint) -> CoreResult<()> {
    let mut expected = frame.clone();
    let expected_read = expected_read(frame, slot);
    assert_eq!(frame.read_taint(slot), expected_read);
    assert_eq!(
        frame.initialized_taint(slot),
        expected_read.as_ref().ok().copied()
    );
    assert_eq!(*frame, expected);
    let initialized = frame
        .cells
        .get(slot.as_usize())
        .map(SlotCell::is_initialized);
    let mut marker = frame.cells.get(slot.as_usize()).and_then(SlotCell::taint);
    assert_eq!(read_taint_cell(slot, initialized, marker), expected_read);
    let expected_write = expected_read.map(|_| ());
    assert_eq!(
        write_taint_cell(slot, initialized, marker.as_mut(), requested),
        expected_write
    );
    assert_eq!(frame.write_taint(slot, requested), expected_write);
    if expected_write.is_ok() {
        let cell = expected
            .cells
            .get_mut(slot.as_usize())
            .ok_or(CoreError::SlotOutOfBounds { slot })?;
        if let SlotCell::Value { taint, .. } = cell {
            *taint = requested;
        }
        assert_eq!(marker, Some(requested));
    } else {
        assert_eq!(
            marker,
            expected
                .cells
                .get(slot.as_usize())
                .and_then(SlotCell::taint)
        );
    }
    assert_eq!(*frame, expected);
    Ok(())
}

// Retain the independent malformed parallel-array domain at the projection boundary.
fn check_legacy_projection(
    slots: u16,
    markers: u16,
    slot: SlotIdx,
    initialized: bool,
    old: Taint,
    requested: Taint,
) {
    let index = slot.as_usize();
    let present = (index < usize::from(slots)).then_some(initialized);
    let mut marker = (index < usize::from(markers)).then_some(old);
    let expected = if index >= usize::from(slots) {
        Err(CoreError::SlotOutOfBounds { slot })
    } else if !initialized {
        Err(CoreError::SlotUninitialized { slot })
    } else if index >= usize::from(markers) {
        Err(CoreError::SlotOutOfBounds { slot })
    } else {
        Ok(old)
    };
    assert_eq!(read_taint_cell(slot, present, marker), expected);
    assert_eq!(
        write_taint_cell(slot, present, marker.as_mut(), requested),
        expected.clone().map(|_| ())
    );
    assert_eq!(
        marker,
        if expected.is_ok() {
            Some(requested)
        } else {
            (index < usize::from(markers)).then_some(old)
        }
    );
}

#[test]
fn selected_cell_boundary_and_mismatched_length_matrix() -> CoreResult<()> {
    for slots in [0_u16, 1, u16::MAX] {
        for markers in [0_u16, 1, u16::MAX] {
            let mut frame = hostile_frame(slots, markers)?;
            let mut indices = vec![
                0,
                slots.saturating_sub(1),
                slots,
                markers.saturating_sub(1),
                markers,
                u16::MAX,
            ];
            indices.sort_unstable();
            indices.dedup();
            for raw in indices {
                let slot = SlotIdx::new(raw);
                for initialized in [false, true] {
                    for old in TAINTS {
                        for requested in TAINTS {
                            check_legacy_projection(
                                slots,
                                markers,
                                slot,
                                initialized,
                                old,
                                requested,
                            );
                            if let Some(cell) = frame.cells.get_mut(slot.as_usize()) {
                                *cell = if initialized {
                                    SlotCell::value(SlotValue::Null, old)
                                } else {
                                    SlotCell::empty()
                                };
                            }
                            check_cell(&mut frame, slot, requested)?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn selected_cell_payloads_and_other_state_do_not_affect_access() -> CoreResult<()> {
    let values = [
        SlotValue::Null,
        SlotValue::Bool(true),
        SlotValue::I64(i64::MIN),
        SlotValue::F64(FiniteF64::new(-1.25)?),
        SlotValue::Symbol(SymbolId::new(u32::MAX)),
        SlotValue::List(ListId::new(u32::MAX)),
        SlotValue::Object(ObjectId::new(u32::MAX)),
        SlotValue::Blob(BlobId::new(u64::MAX)),
    ];
    for value in values {
        for old in TAINTS {
            for requested in TAINTS {
                let mut frame = hostile_frame(2, 2)?;
                frame.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(19), Taint::Random)?;
                frame.write_slot_with_taint(SlotIdx::new(1), value, old)?;
                assert_eq!(frame.read_slot(SlotIdx::new(1)), Ok(&value));
                check_cell(&mut frame, SlotIdx::new(1), requested)?;
                frame.run_id = RunId::new(u64::MAX);
                frame.pc = StepIdx::ZERO;
                frame.executed = u64::MAX;
                frame.states =
                    Box::new([StepState::Cancelled, StepState::Asking, StepState::Failed]);
                frame.max_parallel_in_flight = u16::MAX;
                frame.parallel_in_flight = u16::MAX;
                frame.slot_count = 0;
                frame.step_count = u16::MAX;
                check_cell(&mut frame, SlotIdx::new(1), requested)?;
            }
        }
    }
    Ok(())
}

#[test]
fn selected_cell_original_positive_domain_native_maximum() -> CoreResult<()> {
    for count in [1_u16, u16::MAX] {
        for raw in [0, count.saturating_sub(1)] {
            let slot = SlotIdx::new(raw);
            for requested in TAINTS {
                let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, count)?;
                assert_eq!(frame.write_slot(slot, SlotValue::Null), Ok(()));
                assert_eq!(frame.read_taint(slot), Ok(Taint::Clean));
                let mut expected = frame.clone();
                assert_eq!(frame.write_taint(slot, requested), Ok(()));
                assert_eq!(frame.read_taint(slot), Ok(requested));
                *expected
                    .cells
                    .get_mut(slot.as_usize())
                    .ok_or(CoreError::SlotOutOfBounds { slot })? =
                    SlotCell::value(SlotValue::Null, requested);
                assert_eq!(frame, expected);
            }
        }
    }
    Ok(())
}

fn check_slot_writes(
    frame: &RunFrame,
    slot: SlotIdx,
    value: SlotValue,
    requested: Taint,
) -> CoreResult<()> {
    let index = slot.as_usize();
    for clean_write in [false, true] {
        let mut actual = frame.clone();
        let mut expected = frame.clone();
        let marker = if clean_write { Taint::Clean } else { requested };
        if let Some(target) = expected.cells.get_mut(index) {
            *target = SlotCell::value(value, marker);
        }
        let expected_result = if index < frame.cells.len() {
            Ok(())
        } else {
            Err(CoreError::SlotOutOfBounds { slot })
        };
        let result = if clean_write {
            actual.write_slot(slot, value)
        } else {
            actual.write_slot_with_taint(slot, value, requested)
        };
        assert_eq!(result, expected_result);
        assert_eq!(actual, expected);
    }
    Ok(())
}

#[test]
fn slot_access_public_errors_and_partial_writes() -> CoreResult<()> {
    for slots in [0_u16, 1, u16::MAX] {
        for markers in [0_u16, 1, u16::MAX] {
            let mut frame = hostile_frame(slots, markers)?;
            let mut indices = vec![
                0,
                slots.saturating_sub(1),
                slots,
                markers.saturating_sub(1),
                markers,
                u16::MAX,
            ];
            indices.sort_unstable();
            indices.dedup();
            for raw in indices {
                let slot = SlotIdx::new(raw);
                for initialized in [false, true] {
                    let value = SlotValue::I64(i64::MIN);
                    if let Some(target) = frame.cells.get_mut(slot.as_usize()) {
                        *target = if initialized {
                            SlotCell::value(value, Taint::Secret)
                        } else {
                            SlotCell::empty()
                        };
                    }
                    let before = frame.clone();
                    let expected = if slot.as_usize() >= usize::from(slots) {
                        Err(CoreError::SlotOutOfBounds { slot })
                    } else if !initialized {
                        Err(CoreError::SlotUninitialized { slot })
                    } else {
                        Ok(&value)
                    };
                    assert_eq!(frame.read_slot(slot), expected);
                    assert_eq!(frame, before);
                    for requested in TAINTS {
                        check_slot_writes(&frame, slot, SlotValue::Bool(false), requested)?;
                    }
                }
            }
        }
    }
    Ok(())
}
