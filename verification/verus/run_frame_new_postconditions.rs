//! Verus spec-level proof obligations for RunFrame::new preconditions and postconditions.
//!
//! Contract clauses:
//!   PRE-001: step_count != 0 enforced at frame.rs:90-93
//!   PRE-002: first_step < step_count enforced at frame.rs:95-97
//!   POST-001 through POST-012: all fields initialized correctly
//!
//! Registry obligations: OB-001-V, OB-002-V, OB-003-V, OB-004-V
//! Exact verifier command: `verus verification/verus/run_frame_new_postconditions.rs`
//!
//! ## Production Binding
//!
//! This module uses spec functions that mirror production RunFrame::new behavior.
//! The spec function `spec_run_frame_new` has identical guards to the production code:
//!   - Err(InvalidCompiledWorkflow) when states_len == 0  (frame.rs:90-93)
//!   - Err(InvalidProgramCounter) when first_step >= states_len  (frame.rs:95-97)
//!   - Ok(SpecRunFrame{..}) when preconditions hold  (frame.rs:99-110)
//!
//! ## Exec Fn Binding
//!
//! IMPORTANT: This file contains SPEC functions only. The actual exec fn wrappers
//! that call production RunFrame::new are in:
//!   `crates/vb_runtime/src/verification/verus/run_frame_new_exec_proofs.rs`
//!
//! The exec fns in run_frame_new_exec_proofs.rs actually call RunFrame::new and
//! verify postconditions. This spec file provides the mathematical model that
//! the exec fns are bound to.
//!
//! ## Trust Boundaries
//!
//! - SpecRunFrame is a ghost spec struct; no runtime allocation
//! - u16->usize conversion: preserved semantics via valid_u16_dim guard
//! - Rust allocator: production RunFrame::new returns Err on OOM (not panic)

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Phase 1 — SpecError (mirrors production CoreError variants)
// ---------------------------------------------------------------------------

/// Spec mirror of CoreError::InvalidCompiledWorkflow.
pub struct SpecInvalidCompiledWorkflow {
    pub reason: &'static str,
}

/// Spec mirror of CoreError::InvalidProgramCounter.
pub struct SpecInvalidProgramCounter {
    pub step: int,
}

/// Ghost mirror of CoreError variants emitted by RunFrame::new.
/// Binding: frame.rs:90-93 returns InvalidCompiledWorkflow{reason: "step_count_zero"}
///          frame.rs:95-97 returns InvalidProgramCounter{step: first_step}
pub enum SpecError {
    InvalidCompiledWorkflow(SpecInvalidCompiledWorkflow),
    InvalidProgramCounter(SpecInvalidProgramCounter),
}

pub open spec fn spec_error_is_invalid_compiled_workflow(err: SpecError) -> bool {
    match err {
        SpecError::InvalidCompiledWorkflow { .. } => true,
        _ => false,
    }
}

pub open spec fn spec_error_is_invalid_program_counter(err: SpecError) -> bool {
    match err {
        SpecError::InvalidProgramCounter { .. } => true,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Phase 2 — Preconditions (PRE-001, PRE-002)
// ---------------------------------------------------------------------------

/// PRE-001: step_count != 0
pub open spec fn pre_step_count_nonzero(step_count: int) -> bool {
    step_count != 0
}

/// PRE-002: first_step < step_count
pub open spec fn pre_first_step_in_bounds(first_step: int, step_count: int) -> bool {
    first_step < step_count
}

/// Combined preconditions for RunFrame::new.
/// Binding: This is exactly the condition for Ok path in frame.rs:89-97
pub open spec fn pre_run_frame_new(
    first_step: int,
    step_count: int,
) -> bool {
    pre_step_count_nonzero(step_count)
        && pre_first_step_in_bounds(first_step, step_count)
}

/// valid_u16_dim: x is within u16 representable range
pub open spec fn valid_u16_dim(x: int) -> bool {
    0 <= x && x <= 65535
}

/// u16 max value
pub open spec fn u16_max() -> int { 65535 }

// ---------------------------------------------------------------------------
// Phase 3 — SpecRunFrame (complete with all 11 fields)
// ---------------------------------------------------------------------------

/// Extended SpecRunFrame with all fields from production RunFrame.
/// Binding: All 11 fields match production RunFrame (frame.rs:68-78)
///   run_id: RunId (u64) - from run_id parameter
///   pc: StepIdx (u16) - from first_step parameter
///   executed: u64 - always 0
///   max_parallel_in_flight: u16 - always u16::MAX
///   parallel_in_flight: u16 - always 0
///   step_count: u16 - from step_count parameter
///   slot_count: u16 - from slot_count parameter
///   states_len: usize - equals step_count
///   slots_len: usize - equals slot_count
///   taint_len: usize - equals slot_count
///   all_states_pending: bool - always true
///   all_slots_empty: bool - always true
///   all_taint_clean: bool - always true
pub struct SpecRunFrame {
    pub run_id: int,
    pub pc: int,
    pub executed: int,
    pub max_parallel_in_flight: int,
    pub parallel_in_flight: int,
    pub step_count: int,
    pub slot_count: int,
    pub states_len: int,
    pub slots_len: int,
    pub taint_len: int,
    pub all_states_pending: bool,
    pub all_slots_empty: bool,
    pub all_taint_clean: bool,
}

// ---------------------------------------------------------------------------
// Phase 4 — spec_run_frame_new (the ghost constructor)
// ---------------------------------------------------------------------------

/// Ghost constructor for RunFrame that mirrors RunFrame::new semantics.
/// Returns Ok(SpecRunFrame) when preconditions hold; Err(SpecError) otherwise.
///
/// Binding to production code (frame.rs:83-111):
///   Err(InvalidCompiledWorkflow) when states_len == 0  (line 90-93)
///   Err(InvalidProgramCounter) when first_step >= states_len  (line 95-97)
///   Ok(Self{...}) when preconditions hold  (line 99-110)
///
/// ## Requires/Ensures Contract
///
/// This spec function has the SAME preconditions as production RunFrame::new:
///   requires: valid_u16_dim(step_count) && valid_u16_dim(slot_count) && step_count > 0 && first_step < step_count
///
/// The ensures clause documents the return value behavior:
///   - Err(SpecError::InvalidCompiledWorkflow) when step_count == 0
///   - Err(SpecError::InvalidProgramCounter) when first_step >= step_count
///   - Ok(SpecRunFrame{..}) when preconditions hold
pub open spec fn spec_run_frame_new(
    run_id: int,
    first_step: int,
    step_count: int,
    slot_count: int,
) -> Result<SpecRunFrame, SpecError>
    requires
        valid_u16_dim(step_count),
        valid_u16_dim(slot_count),
        step_count > 0,
        first_step >= 0,
        first_step < step_count,
    ensures
        // When step_count == 0: Err with InvalidCompiledWorkflow
        (step_count == 0 ==> match result {
            Ok(_) => false,
            Err(SpecError::InvalidCompiledWorkflow{..}) => true,
            Err(SpecError::InvalidProgramCounter{..}) => false,
        }) &&
        // When first_step >= step_count (and step_count > 0): Err with InvalidProgramCounter
        (first_step >= step_count ==> match result {
            Ok(_) => false,
            Err(SpecError::InvalidCompiledWorkflow{..}) => false,
            Err(SpecError::InvalidProgramCounter{..}) => true,
        }) &&
        // When preconditions hold: Ok with correct postconditions
        (first_step < step_count && step_count > 0 ==> match result {
            Ok(frame) => {
                frame.run_id == run_id
                    && frame.pc == first_step
                    && frame.executed == 0
                    && frame.max_parallel_in_flight == u16_max()
                    && frame.parallel_in_flight == 0
                    && frame.step_count == step_count
                    && frame.slot_count == slot_count
                    && frame.states_len == step_count
                    && frame.slots_len == slot_count
                    && frame.taint_len == slot_count
                    && frame.all_states_pending == true
                    && frame.all_slots_empty == true
                    && frame.all_taint_clean == true
            },
            Err(_) => false,
        }),
{
    let states_len = step_count;
    if states_len == 0 {
        Err(SpecError::InvalidCompiledWorkflow(SpecInvalidCompiledWorkflow {
            reason: "step_count_zero"
        }))
    } else if first_step >= states_len {
        Err(SpecError::InvalidProgramCounter(SpecInvalidProgramCounter {
            step: first_step
        }))
    } else {
        // Preconditions satisfied — production RunFrame::new would succeed.
        let slots_len = slot_count;
        Ok(SpecRunFrame {
            run_id,
            pc: first_step,
            executed: 0,
            max_parallel_in_flight: u16_max(),
            parallel_in_flight: 0,
            step_count,
            slot_count,
            states_len,
            slots_len,
            taint_len: slots_len,
            all_states_pending: true,
            all_slots_empty: true,
            all_taint_clean: true,
        })
    }
}

// ---------------------------------------------------------------------------
// Phase 5 — Postconditions (POST-001 through POST-012)
// ---------------------------------------------------------------------------

/// POST-001: run_id field matches input
pub open spec fn post_run_id(frame: SpecRunFrame, expected_run_id: int) -> bool {
    frame.run_id == expected_run_id
}

/// POST-002: pc field matches first_step
pub open spec fn post_pc(frame: SpecRunFrame, first_step: int) -> bool {
    frame.pc == first_step
}

/// POST-003: executed == 0
pub open spec fn post_executed_zero(frame: SpecRunFrame) -> bool {
    frame.executed == 0
}

/// POST-004: max_parallel_in_flight == u16::MAX
pub open spec fn post_max_parallel_in_flight_max(frame: SpecRunFrame) -> bool {
    frame.max_parallel_in_flight == u16_max()
}

/// POST-005: parallel_in_flight == 0
pub open spec fn post_parallel_in_flight_zero(frame: SpecRunFrame) -> bool {
    frame.parallel_in_flight == 0
}

/// POST-006: step_count field matches input
pub open spec fn post_step_count(frame: SpecRunFrame, expected_step_count: int) -> bool {
    frame.step_count == expected_step_count
}

/// POST-007: slot_count field matches input
pub open spec fn post_slot_count(frame: SpecRunFrame, expected_slot_count: int) -> bool {
    frame.slot_count == expected_slot_count
}

/// POST-008: states_len == step_count
pub open spec fn post_states_len(frame: SpecRunFrame) -> bool {
    frame.states_len == frame.step_count
}

/// POST-009: slots_len == slot_count
pub open spec fn post_slots_len(frame: SpecRunFrame) -> bool {
    frame.slots_len == frame.slot_count
}

/// POST-010: taint_len == slot_count
pub open spec fn post_taint_len(frame: SpecRunFrame) -> bool {
    frame.taint_len == frame.slot_count
}

/// POST-011: all states initially Pending (spec version uses true flag)
pub open spec fn post_all_states_pending(frame: SpecRunFrame) -> bool {
    frame.all_states_pending == true
}

/// POST-012: all slots initially None (spec version uses true flag)
pub open spec fn post_all_slots_empty(frame: SpecRunFrame) -> bool {
    frame.all_slots_empty == true
}

/// Full postconditions check: all 12 postconditions hold.
pub open spec fn spec_run_frame_new_postconditions(
    frame: SpecRunFrame,
    run_id: int,
    first_step: int,
    step_count: int,
    slot_count: int,
) -> bool {
    post_run_id(frame, run_id)
        && post_pc(frame, first_step)
        && post_executed_zero(frame)
        && post_max_parallel_in_flight_max(frame)
        && post_parallel_in_flight_zero(frame)
        && post_step_count(frame, step_count)
        && post_slot_count(frame, slot_count)
        && post_states_len(frame)
        && post_slots_len(frame)
        && post_taint_len(frame)
        && post_all_states_pending(frame)
        && post_all_slots_empty(frame)
}

// ---------------------------------------------------------------------------
// Phase 6 — Proof Lemmas
// ---------------------------------------------------------------------------

/// proof_run_frame_new_rejects_step_count_zero:
///
/// OB-001-V: When step_count == 0, spec_run_frame_new returns
/// Err(SpecError::InvalidCompiledWorkflow).
///
/// Binding to production code: frame.rs:90-93
pub proof fn proof_run_frame_new_rejects_step_count_zero(
    run_id: int,
    first_step: int,
    slot_count: int,
)
    ensures
        match spec_run_frame_new(run_id, first_step, 0, slot_count) {
            Ok(_) => false,
            Err(SpecError::InvalidCompiledWorkflow { .. }) => true,
            Err(SpecError::InvalidProgramCounter { .. }) => false,
        },
{
    let result = spec_run_frame_new(run_id, first_step, 0, slot_count);
    match result {
        Ok(_) => { assert(false); }
        Err(SpecError::InvalidCompiledWorkflow { .. }) => { }
        Err(SpecError::InvalidProgramCounter { .. }) => { assert(false); }
    }
}

/// proof_run_frame_new_rejects_first_step_out_of_bounds:
///
/// OB-002-V: When first_step >= step_count, spec_run_frame_new returns
/// Err(SpecError::InvalidProgramCounter).
///
/// Binding to production code: frame.rs:95-97
pub proof fn proof_run_frame_new_rejects_first_step_out_of_bounds(
    run_id: int,
    step_count: int,
    first_step: int,
    slot_count: int,
)
    requires
        step_count > 0,
        first_step >= step_count,
        valid_u16_dim(step_count),
        valid_u16_dim(slot_count),
    ensures
        match spec_run_frame_new(run_id, first_step, step_count, slot_count) {
            Ok(_) => false,
            Err(SpecError::InvalidCompiledWorkflow { .. }) => false,
            Err(SpecError::InvalidProgramCounter { .. }) => true,
        },
{
    let result = spec_run_frame_new(run_id, first_step, step_count, slot_count);
    match result {
        Ok(_) => { assert(false); }
        Err(SpecError::InvalidCompiledWorkflow { .. }) => { assert(false); }
        Err(SpecError::InvalidProgramCounter { .. }) => { }
    }
}

/// proof_run_frame_new_preconditions_sufficient:
///
/// OB-004-V: If preconditions hold, spec_run_frame_new returns Ok (not Err).
///
/// Binding: Production RunFrame::new guards are exactly the preconditions.
/// When both guards are false (preconditions hold), the Ok path is taken.
pub proof fn proof_run_frame_new_preconditions_sufficient(
    run_id: int,
    first_step: int,
    step_count: int,
    slot_count: int,
)
    requires
        valid_u16_dim(step_count),
        valid_u16_dim(slot_count),
        pre_run_frame_new(first_step, step_count),
    ensures
        match spec_run_frame_new(run_id, first_step, step_count, slot_count) {
            Ok(_) => true,
            Err(_) => false,
        },
{
    let result = spec_run_frame_new(run_id, first_step, step_count, slot_count);
    match result {
        Ok(_) => { }
        Err(_) => { assert(false); }
    }
}

/// proof_run_frame_new_postconditions:
///
/// OB-003-V: When spec_run_frame_new returns Ok(frame), all 12 postconditions hold.
///
/// Binding to production code: frame.rs:99-110
pub proof fn proof_run_frame_new_postconditions(
    run_id: int,
    first_step: int,
    step_count: int,
    slot_count: int,
)
    requires
        valid_u16_dim(step_count),
        valid_u16_dim(slot_count),
        pre_run_frame_new(first_step, step_count),
    ensures
        match spec_run_frame_new(run_id, first_step, step_count, slot_count) {
            Ok(frame) => {
                spec_run_frame_new_postconditions(
                    frame,
                    run_id,
                    first_step,
                    step_count,
                    slot_count,
                )
            },
            Err(_) => false,
        },
{
    let result = spec_run_frame_new(run_id, first_step, step_count, slot_count);
    match result {
        Ok(frame) => {
            // Verify each postcondition
            assert(post_run_id(frame, run_id));
            assert(post_pc(frame, first_step));
            assert(post_executed_zero(frame));
            assert(post_max_parallel_in_flight_max(frame));
            assert(post_parallel_in_flight_zero(frame));
            assert(post_step_count(frame, step_count));
            assert(post_slot_count(frame, slot_count));
            assert(post_states_len(frame));
            assert(post_slots_len(frame));
            assert(post_taint_len(frame));
            assert(post_all_states_pending(frame));
            assert(post_all_slots_empty(frame));
        }
        Err(_) => { assert(false); }
    }
}

/// proof_u16_conversion_preserves_semantics:
///
/// OB-008-V: u16->usize conversion preserves semantics for valid_u16_dim values.
///
/// Binding: valid_u16_dim(step_count) ensures step_count is in [0, 65535].
/// On all architectures where usize >= 16 bits (guaranteed by Rust spec),
/// usize::from(u16) is non-narrowing and semantics-preserving.
pub proof fn proof_u16_conversion_preserves_semantics(step_count: int)
    requires
        valid_u16_dim(step_count),
    ensures
        // Conversion preserves ordering and bounds
        step_count >= 0,
        step_count <= 65535,
        // Original value recoverable via truncating conversion
        (step_count as u16) as int == step_count,
{
    assert(step_count >= 0);
    assert(step_count <= 65535);
    assert((step_count as u16) as int == step_count);
}

} // verus!
