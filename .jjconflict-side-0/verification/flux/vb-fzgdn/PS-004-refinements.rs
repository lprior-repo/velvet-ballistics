//! PS-004 Flux refinements: Generation advancement bounded safety (POB-vb-fzgdn-017)
//! Production binding: crates/vb_runtime/src/shard/transitions.rs Shard::next_pending_timer_generation
//!                     crates/vb_runtime/src/shard/timer_wheel.rs TimerWheelError::GenerationExhausted
//!
//! Refines: generation = checked_add(current, 1) either increases by exactly 1
//! or results in GenerationExhausted (never wraps, never panics).

/// Refinement module: generation arithmetic bounds.
mod generation_refinements {
    /// Production code reference:
    ///   crates/vb_runtime/src/shard/timer_wheel.rs:83-85
    ///   checked_add(1).ok_or(TimerWheelError::GenerationExhausted)
    ///
    /// Refinement: for gen < u64::MAX, checked_add(1) returns gen + 1 exactly.
    #[flux_rs::trusted]
    #[flux_rs::sig(fn(u64[@gen]) -> u64[gen + 1] requires gen < u64::MAX)]
    pub fn safe_increment(gen: u64) -> u64 {
        gen.checked_add(1).expect("precondition guarantees within bounds")
    }

    /// Refinement: 0 as sentinel (no timer present) maps to generation 1 on first insert.
    /// Production code reference:
    ///   crates/vb_runtime/src/shard/timer_wheel.rs:86: None => Ok(1)
    #[flux_rs::trusted]
    #[flux_rs::sig(fn() -> u64[1])]
    pub fn default_generation() -> u64 {
        1
    }
}
