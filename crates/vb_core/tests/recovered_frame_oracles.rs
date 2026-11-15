#![forbid(unsafe_code)]
use vb_core::frame::{CheckpointStepState as C, StepState as S};
use vb_core::{CoreError, RunFrame, RunId, SlotIdx, SlotValue, StepIdx, Taint};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Marker = fn(&mut RunFrame, StepIdx) -> Result<(), CoreError>;

fn populated() -> Result<RunFrame, CoreError> {
    let mut frame = RunFrame::new(RunId::new(19), StepIdx::new(1), 2, 4)?;
    frame.increment_executed()?;
    frame.set_max_parallel_in_flight(5);
    frame.add_parallel_in_flight(2)?;
    frame.write_slot_with_taint(SlotIdx::new(3), SlotValue::Bool(true), Taint::Secret)?;
    frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::Null, Taint::DerivedFromSecret)?;
    frame.mark_running(StepIdx::new(1))?;
    Ok(frame)
}

#[test]
fn public_marks_obey_all_master_transition_pairs_and_preserve_other_fields() -> TestResult {
    // MASTER Section45; columns follow Pending, Running, Succeeded, Failed,
    // Skipped, Waiting, Asking, Cancelled. Loop reentry remains explicit.
    let rows: [(C, [bool; 8]); 8] = [
        (
            C::Pending,
            [true, true, true, true, true, false, false, true],
        ),
        (
            C::Running,
            [false, true, true, true, true, true, true, true],
        ),
        (
            C::Succeeded,
            [true, false, true, false, false, false, false, false],
        ),
        (
            C::Failed,
            [false, false, false, true, false, false, false, false],
        ),
        (
            C::Skipped,
            [false, false, false, false, true, false, false, false],
        ),
        (
            C::Waiting,
            [false, true, false, false, false, true, false, false],
        ),
        (
            C::Asking,
            [false, true, false, false, false, false, true, false],
        ),
        (
            C::Cancelled,
            [false, false, false, false, false, false, false, true],
        ),
    ];
    let marks: [(C, Marker); 8] = [
        (C::Pending, RunFrame::mark_pending),
        (C::Running, RunFrame::mark_running),
        (C::Succeeded, RunFrame::mark_succeeded),
        (C::Failed, RunFrame::mark_failed),
        (C::Skipped, RunFrame::mark_skipped),
        (C::Waiting, RunFrame::mark_waiting),
        (C::Asking, RunFrame::mark_asking),
        (C::Cancelled, RunFrame::mark_cancelled),
    ];
    let base = populated()?.checkpoint_v1();
    for (source, allowed) in rows {
        for ((target, mark), allowed) in marks.into_iter().zip(allowed) {
            let mut before = base.clone();
            *before.states.first_mut().ok_or("missing state")? = source;
            let mut frame = RunFrame::from_checkpoint_v1(before.clone())
                .map_err(|error| format!("{error:?}"))?;
            let result = mark(&mut frame, StepIdx::ZERO);
            if allowed {
                assert_eq!(result, Ok(()), "{source:?} to {target:?}");
                *before.states.first_mut().ok_or("missing expected state")? = target;
            } else {
                assert_eq!(
                    result,
                    Err(CoreError::InternalInvariantViolation {
                        reason: "invalid_state_transition".into()
                    }),
                    "{source:?} to {target:?}"
                );
            }
            assert_eq!(frame.checkpoint_v1(), before);
            let before = frame.checkpoint_v1();
            assert_eq!(
                mark(&mut frame, StepIdx::new(2)),
                Err(CoreError::StepStateOutOfBounds {
                    step: StepIdx::new(2)
                })
            );
            assert_eq!(frame.checkpoint_v1(), before);
        }
    }
    Ok(())
}

#[test]
fn slot_and_parallel_refusals_preserve_populated_frame_atomically() -> TestResult {
    let mut frame = populated()?;
    let before = frame.clone();
    for slot in [SlotIdx::new(4), SlotIdx::MAX] {
        assert_eq!(
            frame.write_slot(slot, SlotValue::I64(31)),
            Err(CoreError::SlotOutOfBounds { slot })
        );
        assert_eq!(frame, before);
        assert_eq!(
            frame.write_slot_with_taint(slot, SlotValue::I64(37), Taint::Secret),
            Err(CoreError::SlotOutOfBounds { slot })
        );
        assert_eq!(frame, before);
        assert_eq!(
            frame.write_taint(slot, Taint::Random),
            Err(CoreError::SlotOutOfBounds { slot })
        );
        assert_eq!(frame, before);
    }
    assert_eq!(
        frame.write_taint(SlotIdx::ZERO, Taint::Secret),
        Err(CoreError::SlotUninitialized {
            slot: SlotIdx::ZERO
        })
    );
    assert_eq!(frame, before);
    assert_eq!(
        frame.add_parallel_in_flight(4),
        Err(CoreError::ParallelLimitExceeded { limit: 5 })
    );
    assert_eq!(frame, before);
    assert_eq!(
        frame.add_parallel_in_flight(u16::MAX),
        Err(CoreError::InternalInvariantViolation {
            reason: "parallel_in_flight overflow".into()
        })
    );
    assert_eq!(frame, before);
    assert_eq!(
        frame.sub_parallel_in_flight(3),
        Err(CoreError::InternalInvariantViolation {
            reason: "parallel_in_flight underflow".into()
        })
    );
    assert_eq!(frame, before);
    Ok(())
}

#[test]
fn sparse_snapshots_order_slots_count_overwrites_and_own_their_copies() -> TestResult {
    let empty = RunFrame::new(RunId::new(17), StepIdx::ZERO, 2, 4)?;
    assert_eq!(empty.written_slot_count(), 0);
    assert!(empty.initialized_slots()?.is_empty());
    assert_eq!(empty.slots_snapshot(), [None; 4]);
    assert_eq!(empty.taint_snapshot(), [Taint::Clean; 4]);
    assert_eq!(empty.states_snapshot(), [S::Pending; 2]);
    let mut frame = populated()?;
    assert_eq!(frame.written_slot_count(), 2);
    frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(41), Taint::Random)?;
    assert_eq!(frame.written_slot_count(), 2);
    assert_eq!(
        frame.initialized_slots()?,
        [
            (SlotIdx::new(1), SlotValue::I64(41), Taint::Random),
            (SlotIdx::new(3), SlotValue::Bool(true), Taint::Secret)
        ]
    );
    let before = frame.clone();
    let mut slots = frame.slots_snapshot();
    let mut taints = frame.taint_snapshot();
    let mut states = frame.states_snapshot();
    assert_eq!(
        slots,
        [
            None,
            Some(SlotValue::I64(41)),
            None,
            Some(SlotValue::Bool(true))
        ]
    );
    assert_eq!(
        taints,
        [Taint::Clean, Taint::Random, Taint::Clean, Taint::Secret]
    );
    assert_eq!(states, [S::Pending, S::Running]);
    *slots.get_mut(1).ok_or("missing snapshot slot")? = None;
    *taints.get_mut(3).ok_or("missing snapshot taint")? = Taint::Clean;
    *states.get_mut(1).ok_or("missing snapshot state")? = S::Cancelled;
    assert_eq!(frame, before);
    Ok(())
}
