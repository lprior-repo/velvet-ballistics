//! Standalone model for shard helper function specifications.
//!
//! Production binding target: `crates/vb_runtime/src/shard/helpers.rs`

use vstd::prelude::*;

verus! {

    // Standalone model types
    struct StepIdx(u64);

    impl StepIdx {
        pub closed spec fn val(&self) -> u64 { self.0 }
    }

    struct RunState {
        pub num_steps: u64,
    }

    impl RunState {
        pub closed spec fn num_steps(&self) -> u64 { self.num_steps }
    }

    /// Model: WaitUntil nodes always require timers.
    /// WaitEvent/Ask nodes require timers only if they have a timeout_slot.
    /// Other node kinds do not require timers.
    pub closed spec fn timer_registration_required(state: RunState, step: StepIdx) -> bool {
        step.val() < state.num_steps()
    }
}

