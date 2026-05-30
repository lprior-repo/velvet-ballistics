#![forbid(unsafe_code)]
//! Integration behavior tests for RunFrame slot, state, and lifecycle.
//!
//! Covers: creation, slot R/W, overwrite, OOB, uninitialized read,
//! step-state transitions, snapshots, reinitialize, PC, executed counter,
//! extreme values, and proptest write-then-read.

use crate::errors::CoreError;
use crate::frame::{RunFrame, StepState, is_valid_step_state_transition};
use crate::ids::{ListId, ObjectId, RunId, SlotIdx, StepIdx, SymbolId};
use crate::value::{SlotValue, Taint};

use proptest::prelude::*;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn make_frame() -> crate::errors::CoreResult<RunFrame> {
    RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 4)
}

fn make_frame_with(slots: u16) -> crate::errors::CoreResult<RunFrame> {
    RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, slots)
}

// =========================================================================
// 1. Frame creation
// =========================================================================

#[test]
fn frame_creation_valid_config_returns_ok() {
    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 2);
    assert!(frame.is_ok());
}

#[test]
fn frame_creation_valid_config_has_correct_dimensions() {
    let frame = RunFrame::new(RunId::new(42), StepIdx::new(1), 5, 3);
    let frame = frame.unwrap_or_else(|e| panic!("frame creation failed: {e:?}"));
    assert_eq!(frame.run_id(), RunId::new(42));
    assert_eq!(frame.pc(), StepIdx::new(1));
    assert_eq!(frame.step_count(), 5);
    assert_eq!(frame.slot_count(), 3);
}

#[test]
fn frame_creation_zero_step_count_returns_invalid_compiled_workflow() {
    let result = RunFrame::new(RunId::new(1), StepIdx::ZERO, 0, 2);
    assert_eq!(
        result,
        Err(CoreError::InvalidCompiledWorkflow {
            reason: "step_count_zero"
        })
    );
}

#[test]
fn frame_creation_first_step_oob_returns_invalid_program_counter() {
    let result = RunFrame::new(RunId::new(1), StepIdx::new(5), 3, 2);
    assert_eq!(
        result,
        Err(CoreError::InvalidProgramCounter {
            step: StepIdx::new(5)
        })
    );
}

#[test]
fn frame_creation_first_step_at_exact_step_count_returns_invalid_program_counter() {
    let result = RunFrame::new(RunId::new(1), StepIdx::new(4), 4, 2);
    assert_eq!(
        result,
        Err(CoreError::InvalidProgramCounter {
            step: StepIdx::new(4)
        })
    );
}

#[test]
fn frame_creation_slot_count_zero_is_valid() {
    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 0);
    assert!(frame.is_ok());
    let frame = frame.unwrap_or_else(|e| panic!("expected ok: {e:?}"));
    assert_eq!(frame.slot_count(), 0);
}

// =========================================================================
// 2. Slot write/read -- all value types
// =========================================================================

#[test]
fn slot_write_read_i64_roundtrip() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .write_slot(SlotIdx::ZERO, SlotValue::I64(42))
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    let value = frame
        .read_slot(SlotIdx::ZERO)
        .unwrap_or_else(|e| panic!("read: {e:?}"));
    assert_eq!(value, &SlotValue::I64(42));
}

#[test]
fn slot_write_read_i64_negative() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .write_slot(SlotIdx::new(1), SlotValue::I64(-999))
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    let value = frame
        .read_slot(SlotIdx::new(1))
        .unwrap_or_else(|e| panic!("read: {e:?}"));
    assert_eq!(value, &SlotValue::I64(-999));
}

#[test]
fn slot_write_read_bool_roundtrip() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .write_slot(SlotIdx::new(2), SlotValue::Bool(true))
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    let value = frame
        .read_slot(SlotIdx::new(2))
        .unwrap_or_else(|e| panic!("read: {e:?}"));
    assert_eq!(value, &SlotValue::Bool(true));
}

#[test]
fn slot_write_read_bool_false() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .write_slot(SlotIdx::ZERO, SlotValue::Bool(false))
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    let value = frame
        .read_slot(SlotIdx::ZERO)
        .unwrap_or_else(|e| panic!("read: {e:?}"));
    assert_eq!(value, &SlotValue::Bool(false));
}

#[test]
fn slot_write_read_null_roundtrip() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .write_slot(SlotIdx::new(3), SlotValue::Null)
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    let value = frame
        .read_slot(SlotIdx::new(3))
        .unwrap_or_else(|e| panic!("read: {e:?}"));
    assert_eq!(value, &SlotValue::Null);
}

#[test]
fn slot_write_read_symbol_roundtrip() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    let sym = SymbolId::new(7);
    frame
        .write_slot(SlotIdx::ZERO, SlotValue::Symbol(sym))
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    let value = frame
        .read_slot(SlotIdx::ZERO)
        .unwrap_or_else(|e| panic!("read: {e:?}"));
    assert_eq!(value, &SlotValue::Symbol(SymbolId::new(7)));
}

#[test]
fn slot_write_read_list_roundtrip() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    let list = ListId::new(99);
    frame
        .write_slot(SlotIdx::new(2), SlotValue::List(list))
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    let value = frame
        .read_slot(SlotIdx::new(2))
        .unwrap_or_else(|e| panic!("read: {e:?}"));
    assert_eq!(value, &SlotValue::List(ListId::new(99)));
}

#[test]
fn slot_write_read_object_roundtrip() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    let obj = ObjectId::new(33);
    frame
        .write_slot(SlotIdx::new(1), SlotValue::Object(obj))
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    let value = frame
        .read_slot(SlotIdx::new(1))
        .unwrap_or_else(|e| panic!("read: {e:?}"));
    assert_eq!(value, &SlotValue::Object(ObjectId::new(33)));
}

// =========================================================================
// 3. Slot overwrite -- last-write-wins
// =========================================================================

#[test]
fn slot_overwrite_last_write_wins_i64() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .write_slot(SlotIdx::ZERO, SlotValue::I64(10))
        .unwrap_or_else(|e| panic!("write1: {e:?}"));
    frame
        .write_slot(SlotIdx::ZERO, SlotValue::I64(20))
        .unwrap_or_else(|e| panic!("write2: {e:?}"));
    assert_eq!(
        frame.read_slot(SlotIdx::ZERO),
        Ok(&SlotValue::I64(20))
    );
}

#[test]
fn slot_overwrite_last_write_wins_type_change() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .write_slot(SlotIdx::ZERO, SlotValue::I64(1))
        .unwrap_or_else(|e| panic!("write1: {e:?}"));
    frame
        .write_slot(SlotIdx::ZERO, SlotValue::Bool(true))
        .unwrap_or_else(|e| panic!("write2: {e:?}"));
    assert_eq!(
        frame.read_slot(SlotIdx::ZERO),
        Ok(&SlotValue::Bool(true))
    );
}

// =========================================================================
// 4. Out-of-bounds
// =========================================================================

#[test]
fn write_slot_oob_returns_slot_out_of_bounds() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    let result = frame.write_slot(SlotIdx::new(99), SlotValue::I64(1));
    assert_eq!(
        result,
        Err(CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(99)
        })
    );
}

#[test]
fn read_slot_oob_returns_slot_out_of_bounds() {
    let frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    let result = frame.read_slot(SlotIdx::new(999));
    assert_eq!(
        result,
        Err(CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(999)
        })
    );
}

#[test]
fn step_state_oob_returns_step_state_out_of_bounds() {
    let frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    let result = frame.step_state(StepIdx::new(99));
    assert_eq!(
        result,
        Err(CoreError::StepStateOutOfBounds {
            step: StepIdx::new(99)
        })
    );
}

#[test]
fn mark_running_oob_returns_step_state_out_of_bounds() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    let result = frame.mark_running(StepIdx::new(50));
    assert_eq!(
        result,
        Err(CoreError::StepStateOutOfBounds {
            step: StepIdx::new(50)
        })
    );
}

// =========================================================================
// 5. Uninitialized read
// =========================================================================

#[test]
fn read_uninitialized_slot_returns_slot_uninitialized() {
    let frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    let result = frame.read_slot(SlotIdx::ZERO);
    assert_eq!(
        result,
        Err(CoreError::SlotUninitialized {
            slot: SlotIdx::ZERO
        })
    );
}

#[test]
fn read_taint_uninitialized_slot_returns_slot_uninitialized() {
    let frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    let result = frame.read_taint(SlotIdx::ZERO);
    assert_eq!(
        result,
        Err(CoreError::SlotUninitialized {
            slot: SlotIdx::ZERO
        })
    );
}

#[test]
fn write_taint_uninitialized_slot_returns_slot_uninitialized() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    let result = frame.write_taint(SlotIdx::ZERO, Taint::Secret);
    assert_eq!(
        result,
        Err(CoreError::SlotUninitialized {
            slot: SlotIdx::ZERO
        })
    );
}

// =========================================================================
// 6. State peek/transition
// =========================================================================

#[test]
fn state_transition_pending_to_running_to_succeeded() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::new(1)),
        Ok(StepState::Pending)
    );
    frame
        .mark_running(StepIdx::new(1))
        .unwrap_or_else(|e| panic!("mark_running: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::new(1)),
        Ok(StepState::Running)
    );
    frame
        .mark_succeeded(StepIdx::new(1))
        .unwrap_or_else(|e| panic!("mark_succeeded: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::new(1)),
        Ok(StepState::Succeeded)
    );
}

#[test]
fn state_transition_succeeded_terminal_rejects_running() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .mark_running(StepIdx::new(0))
        .unwrap_or_else(|e| panic!("mark_running: {e:?}"));
    frame
        .mark_succeeded(StepIdx::new(0))
        .unwrap_or_else(|e| panic!("mark_succeeded: {e:?}"));
    let result = frame.mark_running(StepIdx::new(0));
    assert_eq!(
        result,
        Err(CoreError::InternalInvariantViolation {
            reason: "invalid_state_transition"
        })
    );
}

#[test]
fn state_transition_failed_terminal_rejects_succeeded() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .mark_running(StepIdx::new(1))
        .unwrap_or_else(|e| panic!("mark_running: {e:?}"));
    frame
        .mark_failed(StepIdx::new(1))
        .unwrap_or_else(|e| panic!("mark_failed: {e:?}"));
    let result = frame.mark_succeeded(StepIdx::new(1));
    assert_eq!(
        result,
        Err(CoreError::InternalInvariantViolation {
            reason: "invalid_state_transition"
        })
    );
}

#[test]
fn state_transition_cancelled_terminal_rejects_pending() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .mark_running(StepIdx::new(2))
        .unwrap_or_else(|e| panic!("mark_running: {e:?}"));
    frame
        .mark_cancelled(StepIdx::new(2))
        .unwrap_or_else(|e| panic!("mark_cancelled: {e:?}"));
    let result = frame.mark_pending(StepIdx::new(2));
    assert_eq!(
        result,
        Err(CoreError::InternalInvariantViolation {
            reason: "invalid_state_transition"
        })
    );
}

#[test]
fn state_transition_idempotent_same_state_is_valid() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .mark_running(StepIdx::new(3))
        .unwrap_or_else(|e| panic!("mark_running: {e:?}"));
    frame
        .mark_running(StepIdx::new(3))
        .unwrap_or_else(|e| panic!("remark running: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::new(3)),
        Ok(StepState::Running)
    );
}

#[test]
fn state_transition_pending_to_running_via_mark_running() {
    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1)
        .unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .mark_running(StepIdx::new(0))
        .unwrap_or_else(|e| panic!("mark_running: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::new(0)),
        Ok(StepState::Running)
    );
}

#[test]
fn state_transition_pending_to_waiting_is_valid() {
    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1)
        .unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .mark_running(StepIdx::new(0))
        .unwrap_or_else(|e| panic!("mark_running: {e:?}"));
    frame
        .mark_waiting(StepIdx::new(0))
        .unwrap_or_else(|e| panic!("mark_waiting: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::new(0)),
        Ok(StepState::Waiting)
    );
}

#[test]
fn state_transition_pending_to_asking_is_valid() {
    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1)
        .unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .mark_running(StepIdx::new(1))
        .unwrap_or_else(|e| panic!("mark_running: {e:?}"));
    frame
        .mark_asking(StepIdx::new(1))
        .unwrap_or_else(|e| panic!("mark_asking: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::new(1)),
        Ok(StepState::Asking)
    );
}

// =========================================================================
// 7. Snapshots
// =========================================================================

#[test]
fn states_snapshot_reflects_mutations() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    assert_eq!(
        frame.states_snapshot(),
        vec![
            StepState::Pending,
            StepState::Pending,
            StepState::Pending,
            StepState::Pending,
        ]
    );
    frame
        .mark_running(StepIdx::new(0))
        .unwrap_or_else(|e| panic!("mark_running: {e:?}"));
    frame
        .mark_succeeded(StepIdx::new(0))
        .unwrap_or_else(|e| panic!("mark_succeeded: {e:?}"));
    assert_eq!(
        frame.states_snapshot(),
        vec![
            StepState::Succeeded,
            StepState::Pending,
            StepState::Pending,
            StepState::Pending,
        ]
    );
}

#[test]
fn slots_snapshot_includes_none_for_uninitialized() {
    let frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    let snap = frame.slots_snapshot();
    assert_eq!(snap.len(), 4);
    assert!(snap.iter().all(|s| s.is_none()), "all slots must be None initially");
}

#[test]
fn slots_snapshot_reflects_write() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .write_slot(SlotIdx::ZERO, SlotValue::I64(7))
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    frame
        .write_slot(SlotIdx::new(2), SlotValue::Bool(true))
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    let snap = frame.slots_snapshot();
    assert_eq!(snap[0], Some(SlotValue::I64(7)));
    assert_eq!(snap[1], None);
    assert_eq!(snap[2], Some(SlotValue::Bool(true)));
    assert_eq!(snap[3], None);
}

#[test]
fn taint_snapshot_reflects_taint_write() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(1), Taint::Secret)
        .unwrap_or_else(|e| panic!("write_taint: {e:?}"));
    frame
        .write_slot(SlotIdx::new(1), SlotValue::Bool(false))
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    let taints = frame.taint_snapshot();
    assert_eq!(taints[0], Taint::Secret);
    assert_eq!(taints[1], Taint::Clean);
}

// =========================================================================
// 8. Reinitialize
// =========================================================================

#[test]
fn reinitialize_resets_all_state_preserves_dimensions() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .write_slot(SlotIdx::ZERO, SlotValue::I64(10))
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    frame
        .mark_running(StepIdx::new(2))
        .unwrap_or_else(|e| panic!("mark_running: {e:?}"));
    frame
        .mark_succeeded(StepIdx::new(2))
        .unwrap_or_else(|e| panic!("mark_succeeded: {e:?}"));
    frame
        .increment_executed()
        .unwrap_or_else(|e| panic!("increment: {e:?}"));

    frame
        .reinitialize(RunId::new(99), StepIdx::ZERO, 4, 4)
        .unwrap_or_else(|e| panic!("reinit: {e:?}"));

    assert_eq!(frame.run_id(), RunId::new(99));
    assert_eq!(frame.pc(), StepIdx::ZERO);
    assert_eq!(frame.executed(), 0);
    assert_eq!(frame.step_count(), 4);
    assert_eq!(frame.slot_count(), 4);

    assert_eq!(
        frame.step_state(StepIdx::new(0)),
        Ok(StepState::Pending)
    );
    assert_eq!(
        frame.step_state(StepIdx::new(2)),
        Ok(StepState::Pending)
    );

    assert_eq!(
        frame.read_slot(SlotIdx::ZERO),
        Err(CoreError::SlotUninitialized {
            slot: SlotIdx::ZERO
        })
    );
}

#[test]
fn reinitialize_rejects_dimension_mismatch() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    let result = frame.reinitialize(RunId::new(2), StepIdx::ZERO, 5, 4);
    assert_eq!(
        result,
        Err(CoreError::InvalidCompiledWorkflow {
            reason: "frame_dimension_mismatch"
        })
    );
}

#[test]
fn reinitialize_rejects_zero_step_count() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    let result = frame.reinitialize(RunId::new(2), StepIdx::ZERO, 0, 4);
    assert_eq!(
        result,
        Err(CoreError::InvalidCompiledWorkflow {
            reason: "step_count_zero"
        })
    );
}

// =========================================================================
// 9. PC operations
// =========================================================================

#[test]
fn set_pc_oob_returns_invalid_program_counter() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    let result = frame.set_pc(StepIdx::new(999));
    assert_eq!(
        result,
        Err(CoreError::InvalidProgramCounter {
            step: StepIdx::new(999)
        })
    );
    assert_eq!(frame.pc(), StepIdx::ZERO);
}

#[test]
fn set_pc_valid_advances_pc() {
    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 6, 1)
        .unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .set_pc(StepIdx::new(3))
        .unwrap_or_else(|e| panic!("set_pc: {e:?}"));
    assert_eq!(frame.pc(), StepIdx::new(3));
}

#[test]
fn set_pc_at_max_valid_index_succeeds() {
    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 5, 1)
        .unwrap_or_else(|e| panic!("frame: {e:?}"));
    frame
        .set_pc(StepIdx::new(4))
        .unwrap_or_else(|e| panic!("set_pc: {e:?}"));
    assert_eq!(frame.pc(), StepIdx::new(4));
}

#[test]
fn set_pc_at_exact_step_count_is_oob() {
    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 5, 1)
        .unwrap_or_else(|e| panic!("frame: {e:?}"));
    let result = frame.set_pc(StepIdx::new(5));
    assert_eq!(
        result,
        Err(CoreError::InvalidProgramCounter {
            step: StepIdx::new(5)
        })
    );
}

// =========================================================================
// 10. Executed counter
// =========================================================================

#[test]
fn increment_executed_advances_counter() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    assert_eq!(frame.executed(), 0);
    frame
        .increment_executed()
        .unwrap_or_else(|e| panic!("inc1: {e:?}"));
    assert_eq!(frame.executed(), 1);
    frame
        .increment_executed()
        .unwrap_or_else(|e| panic!("inc2: {e:?}"));
    assert_eq!(frame.executed(), 2);
}

#[test]
fn increment_executed_many_times_is_monotonic() {
    let mut frame = make_frame().unwrap_or_else(|e| panic!("frame: {e:?}"));
    for expected in 1..=100u64 {
        frame
            .increment_executed()
            .unwrap_or_else(|e| panic!("inc: {e:?}"));
        assert_eq!(frame.executed(), expected);
    }
}

// =========================================================================
// 11. Extreme values
// =========================================================================

#[test]
fn extreme_values_max_slots() {
    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, u16::MAX);
    assert!(frame.is_ok());
    let frame = frame.unwrap_or_else(|e| panic!("frame: {e:?}"));
    assert_eq!(frame.slot_count(), u16::MAX);
}

#[test]
fn extreme_values_many_steps() {
    let steps: u16 = 10_000;
    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, steps, 2);
    assert!(frame.is_ok());
    let frame = frame.unwrap_or_else(|e| panic!("frame: {e:?}"));
    assert_eq!(frame.step_count(), steps);
}

#[test]
fn extreme_values_write_read_last_slot() {
    let slot_count: u16 = u16::MAX;
    let frame =
        RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, slot_count);
    assert!(frame.is_ok());
    let mut frame = frame.unwrap_or_else(|e| panic!("frame: {e:?}"));
    let last_slot = SlotIdx::new(slot_count - 1);
    frame
        .write_slot(last_slot, SlotValue::I64(42))
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    assert_eq!(
        frame.read_slot(last_slot),
        Ok(&SlotValue::I64(42))
    );
}

#[test]
fn extreme_values_write_read_first_step() {
    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, 1)
        .unwrap_or_else(|e| panic!("frame: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::ZERO),
        Ok(StepState::Pending)
    );
    frame
        .mark_running(StepIdx::ZERO)
        .unwrap_or_else(|e| panic!("mark_running: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::ZERO),
        Ok(StepState::Running)
    );
}

// =========================================================================
// 12. is_valid_step_state_transition function
// =========================================================================

#[test]
fn is_valid_transition_pending_to_running_is_true() {
    assert!(is_valid_step_state_transition(
        StepState::Pending,
        StepState::Running
    ));
}

#[test]
fn is_valid_transition_running_to_succeeded_is_true() {
    assert!(is_valid_step_state_transition(
        StepState::Running,
        StepState::Succeeded
    ));
}

#[test]
fn is_valid_transition_same_state_is_true() {
    let states = [
        StepState::Pending,
        StepState::Running,
        StepState::Succeeded,
        StepState::Failed,
        StepState::Skipped,
        StepState::Waiting,
        StepState::Asking,
        StepState::Cancelled,
    ];
    for &s in &states {
        assert!(
            is_valid_step_state_transition(s, s),
            "same-state transition for {s:?} must be true"
        );
    }
}

#[test]
fn is_valid_transition_terminal_rejects_non_same() {
    let terminals = [
        StepState::Succeeded,
        StepState::Failed,
        StepState::Cancelled,
    ];
    for &t in &terminals {
        assert!(
            !is_valid_step_state_transition(t, StepState::Running),
            "{t:?}->Running must be false"
        );
        assert!(
            !is_valid_step_state_transition(t, StepState::Pending),
            "{t:?}->Pending must be false"
        );
    }
}

// =========================================================================
// 13. Proptest -- write-then-read matches
// =========================================================================

fn arb_slot_value() -> impl Strategy<Value = SlotValue> {
    prop_oneof![
        Just(SlotValue::Null),
        Just(SlotValue::Bool(true)),
        Just(SlotValue::Bool(false)),
        any::<i64>().prop_map(SlotValue::I64),
        (0u32..=u32::MAX).prop_map(|v| SlotValue::Symbol(SymbolId::new(v))),
        (0u32..=u32::MAX).prop_map(|v| SlotValue::List(ListId::new(v))),
        (0u32..=u32::MAX).prop_map(|v| SlotValue::Object(ObjectId::new(v))),
    ]
}

proptest! {
    #[test]
    fn proptest_write_then_read_matches(value in arb_slot_value()) {
        let mut frame = make_frame_with(1).expect("frame");
        frame.write_slot(SlotIdx::ZERO, value).expect("write");
        let read = frame.read_slot(SlotIdx::ZERO).expect("read");
        prop_assert_eq!(read, &value);
    }

    #[test]
    fn proptest_overwrite_then_read_returns_last(
        first in arb_slot_value(),
        second in arb_slot_value(),
    ) {
        let mut frame = make_frame_with(1).expect("frame");
        frame.write_slot(SlotIdx::ZERO, first).expect("write1");
        frame.write_slot(SlotIdx::ZERO, second).expect("write2");
        let read = frame.read_slot(SlotIdx::ZERO).expect("read");
        prop_assert_eq!(read, &second);
    }

    #[test]
    fn proptest_two_slots_independent(
        a in arb_slot_value(),
        b in arb_slot_value(),
    ) {
        let mut frame = make_frame_with(2).expect("frame");
        frame.write_slot(SlotIdx::ZERO, a).expect("write0");
        frame.write_slot(SlotIdx::new(1), b).expect("write1");
        let read_a = frame.read_slot(SlotIdx::ZERO).expect("read0");
        let read_b = frame.read_slot(SlotIdx::new(1)).expect("read1");
        prop_assert_eq!(read_a, &a);
        prop_assert_eq!(read_b, &b);
    }

    #[test]
    fn proptest_independent_frames_do_not_interfere(
        val_a in arb_slot_value(),
        val_b in arb_slot_value(),
    ) {
        let mut frame_a = make_frame_with(1).expect("frame_a");
        let mut frame_b = make_frame_with(1).expect("frame_b");

        frame_a.write_slot(SlotIdx::ZERO, val_a).expect("write_a");
        frame_b.write_slot(SlotIdx::ZERO, val_b).expect("write_b");

        let read_a = frame_a.read_slot(SlotIdx::ZERO).expect("read_a");
        let read_b = frame_b.read_slot(SlotIdx::ZERO).expect("read_b");

        prop_assert_eq!(read_a, &val_a);
        prop_assert_eq!(read_b, &val_b);
    }

    #[test]
    fn proptest_taint_write_then_read_matches(
        value in arb_slot_value(),
    ) {
        let mut frame = make_frame_with(1).expect("frame");
        frame.write_slot_with_taint(SlotIdx::ZERO, value, Taint::Secret).expect("write");
        let taint = frame.read_taint(SlotIdx::ZERO).expect("read_taint");
        prop_assert_eq!(taint, Taint::Secret);
    }
}

// =========================================================================
// 14. Additional adversarial: write all step state transitions
// =========================================================================

#[test]
fn all_step_state_variants_are_reachable() {
    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 8, 1)
        .unwrap_or_else(|e| panic!("frame: {e:?}"));

    frame
        .mark_running(StepIdx::new(0))
        .unwrap_or_else(|e| panic!("r: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::new(0)),
        Ok(StepState::Running)
    );

    frame
        .mark_running(StepIdx::new(1))
        .unwrap_or_else(|e| panic!("r: {e:?}"));
    frame
        .mark_succeeded(StepIdx::new(1))
        .unwrap_or_else(|e| panic!("s: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::new(1)),
        Ok(StepState::Succeeded)
    );

    frame
        .mark_running(StepIdx::new(2))
        .unwrap_or_else(|e| panic!("r: {e:?}"));
    frame
        .mark_failed(StepIdx::new(2))
        .unwrap_or_else(|e| panic!("f: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::new(2)),
        Ok(StepState::Failed)
    );

    frame
        .mark_running(StepIdx::new(3))
        .unwrap_or_else(|e| panic!("r: {e:?}"));
    frame
        .mark_skipped(StepIdx::new(3))
        .unwrap_or_else(|e| panic!("k: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::new(3)),
        Ok(StepState::Skipped)
    );

    frame
        .mark_running(StepIdx::new(4))
        .unwrap_or_else(|e| panic!("r: {e:?}"));
    frame
        .mark_waiting(StepIdx::new(4))
        .unwrap_or_else(|e| panic!("w: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::new(4)),
        Ok(StepState::Waiting)
    );

    frame
        .mark_running(StepIdx::new(5))
        .unwrap_or_else(|e| panic!("r: {e:?}"));
    frame
        .mark_asking(StepIdx::new(5))
        .unwrap_or_else(|e| panic!("a: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::new(5)),
        Ok(StepState::Asking)
    );

    frame
        .mark_running(StepIdx::new(6))
        .unwrap_or_else(|e| panic!("r: {e:?}"));
    frame
        .mark_cancelled(StepIdx::new(6))
        .unwrap_or_else(|e| panic!("c: {e:?}"));
    assert_eq!(
        frame.step_state(StepIdx::new(6)),
        Ok(StepState::Cancelled)
    );

    assert_eq!(
        frame.step_state(StepIdx::new(7)),
        Ok(StepState::Pending)
    );
}
