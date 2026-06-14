//! Extern_spec bindings for helper functions in the shard runtime.
//!
//! Production binding target: `crates/vb_runtime/src/shard/helpers.rs`

use crate::ids::StepIdx;
use crate::shard::types::RunState;
use vstd::prelude::*;

verus! {

// ============================================================================
// timer_registration_required extern_spec
// ============================================================================

/// Extern spec for timer_registration_required.
///
/// Production: `helpers.rs:145-155`
///
/// Contract: Returns true if a timer must be registered for the given step.
/// - WaitUntil nodes always require timers
/// - WaitEvent/Ask nodes require timers only if they have a timeout_slot
/// - Other node kinds do not require timers
/// - Missing steps (out of bounds) return false
#[extern_spec]
mod helpers_spec {
    use vstd::prelude::*;
    use crate::ids::StepIdx;
    use crate::shard::types::RunState;

    #[verifier::extern_spec]
    #[must_use]
    pub fn timer_registration_required(state: &RunState, step: StepIdx) -> bool;
}

} // verus!
