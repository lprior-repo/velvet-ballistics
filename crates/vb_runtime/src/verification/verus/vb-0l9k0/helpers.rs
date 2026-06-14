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

} // verus!
