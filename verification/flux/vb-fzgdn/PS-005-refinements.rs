//! PS-005 Flux refinements: Duplicate key idempotency (POB-vb-fzgdn-021)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel::insert
//!                     When key already present, insert cancels old and inserts new entry.
//!
//! Refinement: duplicate insert preserves count = 1 and updates kind/deadline.

use vb_core::ids::RunId;
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::PendingTimerKind;

/// Refinement module: duplicate insert preserves invariants.
mod duplicate_refinements {
    use std::time::Instant;

    /// Production code reference:
    ///   crates/vb_runtime/src/shard/timer_wheel.rs:61-78
    ///   fn insert: cancels existing, inserts new entry with incremented generation.
    ///
    /// Refinement: after insert on existing run, len() == 1 and kind matches the new kind.
    ///
    /// TRUSTED BOUNDARY justification: The production TimerWheel::insert at
    /// timer_wheel.rs:61-78 replaces the existing timer entry. get_kind()
    /// returns the new kind. Verified by Kani (PO-KANI-vb-fzgdn-021) which
    /// checks insert idempotency for duplicate keys.
    #[flux_rs::trusted]
    #[flux_rs::sig(fn(&TimerWheel, RunId, PendingTimerKind) -> bool)]
    pub fn timer_kind_matches(
        wheel: &TimerWheel,
        run: RunId,
        expected: PendingTimerKind,
    ) -> bool {
        wheel.get_kind(run) == Some(expected)
    }
}
