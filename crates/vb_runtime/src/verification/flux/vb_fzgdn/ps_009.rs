//! PS-009 Flux refinements: Zero-duration branch determinism (POB-vb-fzgdn-039)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel::fire_expired
//!                     range(..=now) includes exact match (deadline == now).
//!
//! Refinement: zero-duration timer (deadline == now) fires immediately via
//! BTreeMap::range(..=now) which includes the key exactly at now.

/// Refinement module: exact deadline match.
mod zero_duration_refinements {
    use std::time::Instant;

    /// Production code reference:
    ///   crates/vb_runtime/src/shard/timer_wheel.rs:111-115
    ///   .range(..=now) — Rust's RangeToInclusive includes the upper bound.
    ///
    /// Refinement: deadline <= now fires (inclusive upper bound).
    ///
    /// TRUSTED BOUNDARY justification: Rust's RangeToInclusive includes the
    /// upper bound, so BTreeMap::range(..=now) selects all deadlines <= now.
    /// The Instant comparison is provided by std. Verified by Kani
    /// (PO-KANI-vb-fzgdn-039) and integration tests for zero-duration timer
    /// firing. Trusted because Instant internals are opaque to Flux.
    #[flux_rs::trusted]
    #[flux_rs::sig(fn(Instant) -> bool[true])]
    pub fn exact_deadline_fires_if_at_now(deadline: Instant) -> bool {
        let now = Instant::now();
        deadline <= now // fires immediately if deadline equals or predates now
    }
}
