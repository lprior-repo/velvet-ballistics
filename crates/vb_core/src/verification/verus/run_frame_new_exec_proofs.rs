//! Standalone Verus proofs for RunFrame::new pre/post-condition contracts.
//!
//! This file defines spec mirror types for RunFrame and proves:
//! - RunFrame::new returns Ok when step_count > 0 and first_step < step_count
//! - RunFrame::new returns Err when step_count == 0
//! - RunFrame::new returns Err when first_step >= step_count
//! - All step states are initialized to Pending
//! - All slots are initialized to None
//!
//! Production binding:
//! - RunFrame struct → crate::frame::RunFrame
//! - CoreResult → crate::errors::CoreResult
//! - CoreError → crate::errors::CoreError
//! - RunId → crate::ids::RunId
//! - StepIdx → crate::ids::StepIdx
//!
//! GOD RULE 2: Specs mirror production logic without depending on crate imports.

use vstd::prelude::*;

verus! {

    // ===========================================================================
    // Spec mirror types for RunFrame
    // ===========================================================================

    /// Mirrors crate::frame::RunFrame fields (simplified for proof).
    pub struct SpecRunFrame {
        pub run_id: u64,
        pub pc: u16,
        pub executed: u64,
        pub step_count: u16,
        pub slot_count: u16,
        pub states: Seq<SpecStepState>,
        pub slots: Seq<Option<u64>>,
    }

    /// Mirrors crate::frame::StepState (8 variants).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SpecStepState {
        Pending,
        Running,
        Succeeded,
        Failed,
        Skipped,
        Waiting,
        Asking,
        Cancelled,
    }

    /// Mirrors CoreResult<Self>.
    pub struct SpecCoreResult<T> {
        pub is_ok: bool,
        pub ok_val: T,
        pub err_code: u8,
    }

    /// Spec: valid inputs for RunFrame::new.
    pub closed spec fn spec_run_frame_new_accepts(step_count: u16, first_step: u16) -> bool {
        step_count > 0 && first_step < step_count
    }

    /// Spec: slot count must be positive.
    pub closed spec fn spec_run_frame_new_accepts_slot_count(slot_count: u16) -> bool {
        slot_count > 0
    }

    /// Spec: initial states are all Pending.
    pub closed spec fn spec_initial_states_pending(states: &Seq<SpecStepState>) -> bool {
        forall|i: int| 0 <= i && i < states.len() ==> states[i as int] == SpecStepState::Pending
    }

    /// Spec: initial slots are all None.
    pub closed spec fn spec_initial_slots_none(slots: &Vec<Option<u64>>) -> bool {
        forall|i: int| 0 <= i && i < slots.len() as int ==> slots[i as int].is_none()
    }

    // ===========================================================================
    // Spec model of RunFrame::new
    // ===========================================================================

    /// Spec: what RunFrame::new produces on valid inputs.
    /// Error codes: 0=OK, 1=step_count_zero, 2=invalid_pc, 3=slot_count_zero.
    /// Uses seq![x; n] to create a sequence of n copies of x.
    pub closed spec fn spec_run_frame_new_model(
        run_id: u64,
        first_step: u16,
        step_count: u16,
        slot_count: u16,
    ) -> SpecCoreResult<SpecRunFrame> {
        if step_count == 0 {
            SpecCoreResult { is_ok: false, ok_val: SpecRunFrame { run_id: 0, pc: 0, executed: 0, step_count: 0, slot_count: 0, states: seq![], slots: seq![] }, err_code: 1 }
        } else if first_step >= step_count {
            SpecCoreResult { is_ok: false, ok_val: SpecRunFrame { run_id: 0, pc: 0, executed: 0, step_count: 0, slot_count: 0, states: seq![], slots: seq![] }, err_code: 2 }
        } else if slot_count == 0 {
            SpecCoreResult { is_ok: false, ok_val: SpecRunFrame { run_id: 0, pc: 0, executed: 0, step_count: 0, slot_count: 0, states: seq![], slots: seq![] }, err_code: 3 }
        } else {
            // seq![x; n] creates a sequence of n copies of x (Verus spec-mode syntax).
            let states: Seq<SpecStepState> = seq![SpecStepState::Pending; step_count as nat];
            let slots: Seq<Option<u64>> = seq![None; slot_count as nat];
            SpecCoreResult { is_ok: true, ok_val: SpecRunFrame { run_id, pc: first_step, executed: 0, step_count, slot_count, states, slots }, err_code: 0 }
        }
    }

    // ===========================================================================
    // PO-RUNFRAME-001: Valid inputs produce Ok with Pending states.
    // ===========================================================================

    /// Proof: valid inputs produce Ok with correct initialization.
    pub proof fn proof_valid_inputs_produce_pending(
        run_id: u64,
        first_step: u16,
        step_count: u16,
        slot_count: u16,
    )
        requires
            step_count > 0,
            first_step < step_count,
            slot_count > 0,
        ensures
            {
                let result = spec_run_frame_new_model(run_id, first_step, step_count, slot_count);
                result.is_ok && spec_initial_states_pending(&result.ok_val.states)
            },
    {
        let result = spec_run_frame_new_model(run_id, first_step, step_count, slot_count);
        assert(result.is_ok);
        assert(spec_initial_states_pending(&result.ok_val.states));
    }

    // ===========================================================================
    // PO-RUNFRAME-002: step_count == 0 produces Err.
    // ===========================================================================

    /// Proof: RunFrame::new rejects step_count == 0.
    pub proof fn proof_step_count_zero_rejected(
        run_id: u64,
        first_step: u16,
        slot_count: u16,
    )
        requires
            slot_count > 0,
        ensures
            !spec_run_frame_new_model(run_id, first_step, 0, slot_count).is_ok,
    {
        let result = spec_run_frame_new_model(run_id, first_step, 0, slot_count);
        assert(!result.is_ok);
    }

    // ===========================================================================
    // PO-RUNFRAME-003: first_step >= step_count produces Err.
    // ===========================================================================

    /// Proof: RunFrame::new rejects first_step >= step_count.
    pub proof fn proof_first_step_out_of_bounds_rejected(
        run_id: u64,
        first_step: u16,
        step_count: u16,
        slot_count: u16,
    )
        requires
            step_count > 0,
            first_step >= step_count,
            slot_count > 0,
        ensures
            !spec_run_frame_new_model(run_id, first_step, step_count, slot_count).is_ok,
    {
        let result = spec_run_frame_new_model(run_id, first_step, step_count, slot_count);
        assert(!result.is_ok);
    }

    // ===========================================================================
    // PO-RUNFRAME-004: slot_count == 0 produces Err.
    // ===========================================================================

    /// Proof: RunFrame::new rejects slot_count == 0.
    pub proof fn proof_slot_count_zero_rejected(
        run_id: u64,
        first_step: u16,
        step_count: u16,
    )
        requires
            step_count > 0,
            first_step < step_count,
        ensures
            !spec_run_frame_new_model(run_id, first_step, step_count, 0).is_ok,
    {
        let result = spec_run_frame_new_model(run_id, first_step, step_count, 0);
        assert(!result.is_ok);
    }

    // ===========================================================================
    // PO-RUNFRAME-005: All states initialized to Pending.
    // ===========================================================================

    /// Proof: successful RunFrame creation initializes all states to Pending.
    pub proof fn proof_all_states_pending(
        run_id: u64,
        first_step: u16,
        step_count: u16,
        slot_count: u16,
    )
        requires
            step_count > 0,
            first_step < step_count,
            slot_count > 0,
        ensures
            {
                let result = spec_run_frame_new_model(run_id, first_step, step_count, slot_count);
                result.is_ok ==> forall|i: int| 0 <= i && i < result.ok_val.step_count as int
                    ==> result.ok_val.states[i as int] == SpecStepState::Pending
            },
    {
        let result = spec_run_frame_new_model(run_id, first_step, step_count, slot_count);
        if result.is_ok {
            assert(forall|i: int| 0 <= i && i < result.ok_val.step_count as int
                ==> result.ok_val.states[i as int] == SpecStepState::Pending);
        }
    }

    // ===========================================================================
    // PO-RUNFRAME-006: All slots initialized to None.
    // ===========================================================================

    /// Proof: successful RunFrame creation initializes all slots to None.
    pub proof fn proof_all_slots_none(
        run_id: u64,
        first_step: u16,
        step_count: u16,
        slot_count: u16,
    )
        requires
            step_count > 0,
            first_step < step_count,
            slot_count > 0,
        ensures
            {
                let result = spec_run_frame_new_model(run_id, first_step, step_count, slot_count);
                result.is_ok ==> forall|i: int| 0 <= i && i < result.ok_val.slot_count as int
                    ==> result.ok_val.slots[i as int].is_none()
            },
    {
        let result = spec_run_frame_new_model(run_id, first_step, step_count, slot_count);
        if result.is_ok {
            assert(forall|i: int| 0 <= i && i < result.ok_val.slot_count as int
                ==> result.ok_val.slots[i as int].is_none());
        }
    }
}
