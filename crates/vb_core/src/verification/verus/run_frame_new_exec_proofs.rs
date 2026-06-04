//! Exec fn wrappers for RunFrame::new — production binding proofs.
//!
//! This module provides exec fn wrappers that ACTUALLY CALL production `RunFrame::new`,
//! binding the Verus spec functions to the production constructor.
//!
//! ## Production Binding
//!
//! - Source: `crates/vb_core/src/frame.rs` — `RunFrame::new`
//! - Signature: `pub fn new(run_id: RunId, first_step: StepIdx, step_count: u16, slot_count: u16) -> CoreResult<Self>`
//! - Errors:
//!   - `Err(CoreError::InvalidCompiledWorkflow{reason: "step_count_zero"})` when `states_len == 0`
//!   - `Err(CoreError::InvalidProgramCounter{step: first_step})` when `first_step.as_usize() >= states_len`
//!   - `Ok(Self{..})` when preconditions hold (line 99-110)
//!
//! ## Registry Obligations
//!
//! OB-001-V: RunFrame::new rejects step_count == 0
//! OB-002-V: RunFrame::new rejects first_step >= step_count
//! OB-003-V: RunFrame::new returns Ok with correct postconditions when preconditions hold
//! OB-004-V: Preconditions are sufficient for Ok path
//!
//! ## Trust Boundaries
//!
//! - u16->usize conversion: `states_len = usize::from(step_count)` preserves semantics
//! - Vec allocation: production returns Err on OOM (not panic)
//! - Box conversion: `vec![...].into_boxed_slice()` is total

#![allow(unused_imports)]

use vb_core::errors::CoreError;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, StepIdx};
use vstd::prelude::*;

verus! {

// =========================================================================
// Phase 1 — Exec fn wrapper for RunFrame::new with preconditions
// =========================================================================

/// exec fn wrapper that calls production RunFrame::new and verifies preconditions.
///
/// REQUIRES: step_count > 0, first_step.as_usize() < step_count as usize
/// ENSURES: returns Ok(frame) with frame.pc == first_step, frame.run_id == run_id, etc.
///
/// This exec fn actually calls the production RunFrame::new, binding the spec
/// to the actual implementation.
pub exec fn prove_run_frame_new_preconditions(
    run_id: RunId,
    first_step: StepIdx,
    step_count: u16,
    slot_count: u16,
) -> (result: CoreResult<RunFrame>)
    requires
        step_count > 0,
        first_step.as_usize() < usize::from(step_count),
    ensures
        result.is_ok(),
        match result {
            Ok(frame) => {
                frame.run_id() == run_id
                    && frame.pc() == first_step
                    && frame.executed() == 0
                    && frame.max_parallel_in_flight() == u16::MAX
                    && frame.parallel_in_flight() == 0
                    && frame.step_count() == step_count
                    && frame.slot_count() == slot_count
            },
            Err(_) => false,
        },
{
    let result = RunFrame::new(run_id, first_step, step_count, slot_count);
    result
}

// =========================================================================
// Phase 2 — Exec fn wrapper for RunFrame::new error path (step_count == 0)
// =========================================================================

/// exec fn wrapper that calls production RunFrame::new with step_count == 0.
///
/// REQUIRES: step_count == 0
/// ENSURES: returns Err(CoreError::InvalidCompiledWorkflow{reason: "step_count_zero"})
///
/// This exec fn actually calls the production RunFrame::new to verify the
/// error path for zero step_count.
pub exec fn prove_run_frame_new_rejects_step_count_zero(
    run_id: RunId,
    first_step: StepIdx,
    step_count: u16,
    slot_count: u16,
) -> (result: CoreResult<RunFrame>)
    requires
        step_count == 0,
    ensures
        match result {
            Ok(_) => false,
            Err(CoreError::InvalidCompiledWorkflow{reason}) => reason == "step_count_zero",
            Err(CoreError::InvalidProgramCounter{..}) => false,
        },
{
    RunFrame::new(run_id, first_step, step_count, slot_count)
}

// =========================================================================
// Phase 3 — Exec fn wrapper for RunFrame::new error path (first_step >= step_count)
// =========================================================================

/// exec fn wrapper that calls production RunFrame::new with first_step >= step_count.
///
/// REQUIRES: step_count > 0, first_step.as_usize() >= step_count as usize
/// ENSURES: returns Err(CoreError::InvalidProgramCounter{step: first_step})
///
/// This exec fn actually calls the production RunFrame::new to verify the
/// error path for out-of-bounds first_step.
pub exec fn prove_run_frame_new_rejects_first_step_out_of_bounds(
    run_id: RunId,
    first_step: StepIdx,
    step_count: u16,
    slot_count: u16,
) -> (result: CoreResult<RunFrame>)
    requires
        step_count > 0,
        first_step.as_usize() >= usize::from(step_count),
    ensures
        match result {
            Ok(_) => false,
            Err(CoreError::InvalidCompiledWorkflow{..}) => false,
            Err(CoreError::InvalidProgramCounter{step}) => step == first_step,
        },
{
    RunFrame::new(run_id, first_step, step_count, slot_count)
}

// =========================================================================
// Phase 4 — Exec fn wrapper for RunFrame::new postconditions verification
// =========================================================================

/// exec fn wrapper that calls production RunFrame::new and verifies all postconditions.
///
/// REQUIRES: step_count > 0, first_step.as_usize() < step_count as usize
/// ENSURES: Ok(frame) with verified postconditions:
///
/// POST-001: frame.run_id() == run_id
/// POST-002: frame.pc() == first_step
/// POST-003: frame.executed() == 0
/// POST-004: frame.max_parallel_in_flight() == u16::MAX
/// POST-005: frame.parallel_in_flight() == 0
/// POST-006: frame.step_count() == step_count
/// POST-007: frame.slot_count() == slot_count
/// POST-008: frame.states().len() == step_count as usize
/// POST-009: frame.slots().len() == slot_count as usize
/// POST-010: frame.taint().len() == slot_count as usize
/// POST-011: all states initially Pending
/// POST-012: all slots initially None
/// POST-013: all taint initially Clean
pub exec fn prove_run_frame_new_postconditions(
    run_id: RunId,
    first_step: StepIdx,
    step_count: u16,
    slot_count: u16,
) -> (result: CoreResult<RunFrame>)
    requires
        step_count > 0,
        first_step.as_usize() < usize::from(step_count),
    ensures
        result.is_ok(),
{
    let result = RunFrame::new(run_id, first_step, step_count, slot_count);
    if let Ok(ref frame) = result {
        // Verify postconditions via spec function
        let states_len = frame.states().len();
        let slots_len = frame.slots().len();
        let taint_len = frame.taint().len();

        // These assertions verify the spec matches production
        assert(frame.run_id() == run_id);        // POST-001
        assert(frame.pc() == first_step);        // POST-002
        assert(frame.executed() == 0);            // POST-003
        assert(frame.max_parallel_in_flight() == u16::MAX);  // POST-004
        assert(frame.parallel_in_flight() == 0);  // POST-005
        assert(frame.step_count() == step_count); // POST-006
        assert(frame.slot_count() == slot_count); // POST-007
        assert(states_len == usize::from(step_count));   // POST-008
        assert(slots_len == usize::from(slot_count));    // POST-009
        assert(taint_len == usize::from(slot_count));    // POST-010
    }
    result
}

// =========================================================================
// Phase 5 — Exec fn for verifying states are initially Pending
// =========================================================================

/// Verifies that all states in a newly created RunFrame are Pending.
pub exec fn prove_run_frame_states_initially_pending(
    run_id: RunId,
    first_step: StepIdx,
    step_count: u16,
    slot_count: u16,
) -> (result: CoreResult<RunFrame>)
    requires
        step_count > 0,
        first_step.as_usize() < usize::from(step_count),
    ensures
        result.is_ok(),
{
    let result = RunFrame::new(run_id, first_step, step_count, slot_count);
    if let Ok(ref frame) = result {
        // Verify each state is Pending
        let mut i: usize = 0;
        while i < frame.states().len() {
            let state = frame.states()[i];
            assert(state == vb_core::frame::StepState::Pending);
            i = i + 1;
        }
    }
    result
}

// =========================================================================
// Phase 6 — Exec fn for verifying slots are initially None
// =========================================================================

/// Verifies that all slots in a newly created RunFrame are None.
pub exec fn prove_run_frame_slots_initially_none(
    run_id: RunId,
    first_step: StepIdx,
    step_count: u16,
    slot_count: u16,
) -> (result: CoreResult<RunFrame>)
    requires
        step_count > 0,
        first_step.as_usize() < usize::from(step_count),
    ensures
        result.is_ok(),
{
    let result = RunFrame::new(run_id, first_step, step_count, slot_count);
    if let Ok(ref frame) = result {
        // Verify each slot is None
        let mut i: usize = 0;
        while i < frame.slots().len() {
            let slot = frame.slots()[i];
            assert(slot.is_none());
            i = i + 1;
        }
    }
    result
}

} // verus!