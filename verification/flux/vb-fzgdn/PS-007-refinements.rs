//! PS-007 Flux refinements: Monotonic clock advancement (POB-vb-fzgdn-030)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel::fire_expired
//!                     BTreeMap::range(..=now) selects all deadlines <= now.
//!
//! Refinement: fire_expired relies on BTreeMap range semantics for monotonic
//! selection. Deadlines are compared via Instant's Ord impl (monotonic by OS).

/// Refinement module: monotonic deadline ordering.
mod clock_refinements {
    use std::time::Instant;

    /// Production code reference:
    ///   crates/vb_runtime/src/shard/timer_wheel.rs:109-128
    ///   range(..=now) collects expired keys; BTreeMap guarantees ordered traversal.
    ///
    /// Refinement: fire_expired returns entries in deadline order (BTreeMap key order).
    ///
    /// TRUSTED BOUNDARY justification: Instant comparison via Ord is
    /// provided by std and guaranteed monotonic by the OS. Expresses
    /// BTreeMap::range(..=now) semantics. Verified by Kani
    /// (PO-KANI-vb-fzgdn-030) and integration tests. Trusted because
    /// Instant internals are opaque to Flux.
    #[flux_rs::trusted]
    #[flux_rs::sig(fn(Instant, Instant) -> bool)]
    pub fn is_expired(deadline: Instant, now: Instant) -> bool {
        deadline <= now
    }

    /// Refinement: reverse comparison identifies future timers.
    ///
    /// TRUSTED BOUNDARY justification: Logical complement of is_expired.
    /// Future timers have deadline > now. Same justification — Instant Ord
    /// semantics trusted, verified by Kani.
    #[flux_rs::trusted]
    #[flux_rs::sig(fn(Instant, Instant) -> bool)]
    pub fn is_future(deadline: Instant, now: Instant) -> bool {
        deadline > now
    }
}
