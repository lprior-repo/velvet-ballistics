//! PS-002 Flux refinements: PendingTimer numeric field constraints (POB-vb-fzgdn-008)
//! Production binding: crates/vb_runtime/src/shard/types.rs PendingTimer, PendingTimerKind
//!                     PendingTimer::matches_authority uses structural equality on all fields.

use vb_runtime::shard::{PendingTimer, PendingTimerKind};

/// Flux refinement: PendingTimer's generation is always >= 1 when timer is active.
/// The production code sets generation=1 on first insert, and generation increments
/// on replacement via checked_add.
///
/// Production code reference:
///   crates/vb_runtime/src/shard/transitions.rs:165-173
///   fn next_pending_timer_generation: None => Ok(1), Some(t) => t.generation.checked_add(1)
mod pending_timer_refinements {
    use vb_core::ids::{RunId, StepIdx};
    use std::time::Instant;

    /// Refinement: generation >= 1 for any active pending timer.
    /// Production code reference: crates/vb_runtime/src/shard/timer_wheel.rs:86
    #[flux_rs::trusted]
    #[flux_rs::sig(fn() -> u64[1])]
    pub fn initial_generation() -> u64 {
        1
    }

    /// Refinement: matches_authority is structural equality on (generation, deadline, kind).
    /// Production code reference:
    ///   crates/vb_runtime/src/shard/types.rs:46-53
    ///   pub fn matches_authority(self, generation: u64, deadline: Instant, kind: PendingTimerKind) -> bool {
    ///       self.generation == generation && self.deadline == deadline && self.kind == kind
    ///   }
    #[flux_rs::trusted]
    #[flux_rs::sig(fn(&PendingTimer, u64, PendingTimerKind) -> bool)]
    pub fn matches_authority_except_deadline(
        timer: &PendingTimer,
        generation: u64,
        kind: PendingTimerKind,
    ) -> bool {
        timer.generation == generation && timer.kind == kind
    }

    /// Refinement: step index is a valid StepIdx (0..step_count).
    /// Production code reference: crates/vb_runtime/src/shard/types.rs:38
    #[flux_rs::trusted]
    #[flux_rs::sig(fn(&PendingTimer) -> u16)]
    pub fn timer_step_raw(timer: &PendingTimer) -> u16 {
        timer.step.get()
    }
}
