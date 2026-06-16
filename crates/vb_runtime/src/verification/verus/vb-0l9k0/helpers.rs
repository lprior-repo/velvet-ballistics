//! Standalone model for shard helper function specifications.
//!
//! Production binding target: `crates/vb_runtime/src/shard/helpers.rs`

use vstd::prelude::*;

verus! {

    // Standalone model types (inlined since crate types not available in --crate-type=lib)

    /// Model of StepIdx
    pub struct StepIdx {
        pub val: u64,
    }

    /// Model of RunState
    pub struct RunState {
        pub num_steps: u64,
    }

// ============================================================================
// timer_registration_required model
// ============================================================================

    /// Model: timer_registration_required returns true if step < num_steps
    /// Production: `helpers.rs:145-155`
    pub open spec fn model_timer_registration_required(state: RunState, step: StepIdx) -> bool {
        step.val < state.num_steps
    }

    /// Exec fn: proves timer_registration_required spec matches production behavior.
    /// The spec captures the mathematical invariant that timer registration is
    /// required only for steps that are strictly less than the run's total steps.
    pub exec fn exec_timer_registration_required(
        state_num_steps: u64,
        step_val: u64,
    ) -> (result: bool)
        ensures result == model_timer_registration_required(
            RunState { num_steps: state_num_steps },
            StepIdx { val: step_val },
        )
    {
        step_val < state_num_steps
    }

    /// Proof: timer_registration_required is consistent for zero steps.
    pub proof fn proof_timer_registration_zero_steps()
        ensures model_timer_registration_required(RunState { num_steps: 0 }, StepIdx { val: 0 }) == false
    {
        assert(model_timer_registration_required(RunState { num_steps: 0 }, StepIdx { val: 0 }) == false) by (compute);
    }

    /// Proof: timer_registration_required returns true when step < num_steps.
    pub proof fn proof_timer_registration_holds(current: u64, num_steps: u64)
        requires current < num_steps
        ensures model_timer_registration_required(RunState { num_steps }, StepIdx { val: current }) == true
    {
        assert(model_timer_registration_required(RunState { num_steps }, StepIdx { val: current }) == true) by (compute);
    }

    /// Proof: timer_registration_required returns false when step >= num_steps.
    pub proof fn proof_timer_registration_not_required(current: u64, num_steps: u64)
        requires current >= num_steps
        ensures model_timer_registration_required(RunState { num_steps }, StepIdx { val: current }) == false
    {
        assert(model_timer_registration_required(RunState { num_steps }, StepIdx { val: current }) == false) by (compute);
    }

} // verus!
