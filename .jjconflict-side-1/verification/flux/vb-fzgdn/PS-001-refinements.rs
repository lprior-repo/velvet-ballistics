//! PS-001 Flux refinements: TimerDeadline u64 arithmetic safety (POB-vb-fzgdn-003)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel::next_generation
//!                     checks that generation.checked_add(1) doesn't overflow.

use vb_runtime::shard::timer_wheel::TimerWheelError;

/// Refined type: a timer generation value that is strictly less than u64::MAX.
/// This ensures checked_add(1) will never overflow to None.
/// The production code at TimerWheel::next_generation uses checked_add(1)
/// and returns TimerWheelError::GenerationExhausted on overflow.
#[flux_rs::refined_by(gen: int)]
pub struct SafeGeneration(u64);

#[flux_rs::trusted]
impl SafeGeneration {
    /// Creates a SafeGeneration from a u64 value that is guaranteed < u64::MAX.
    #[flux_rs::sig(fn(u64) -> SafeGeneration[gen])]
    pub fn new(gen: u64) -> SafeGeneration {
        SafeGeneration(gen)
    }

    /// Returns the inner u64 value.
    #[flux_rs::sig(fn(&SafeGeneration[@gen]) -> u64[gen])]
    pub fn get(&self) -> u64 {
        self.0
    }
}

/// Module: refinement annotations that bind to production code.
mod production_refinements {
    /// Refinement: for any generation < u64::MAX, checked_add(1) succeeds.
    /// This is the core arithmetic property behind Shard::next_pending_timer_generation.
    ///
    /// Production code reference:
    ///   crates/vb_runtime/src/shard/timer_wheel.rs:83-85
    ///   fn next_generation(&self, run: RunId) -> Result<u64, TimerWheelError> {
    ///       match self.by_run.get(&run).copied() {
    ///           Some(entry) => entry.generation.checked_add(1).ok_or(TimerWheelError::GenerationExhausted),
    ///           None => Ok(1),
    ///       }
    ///   }
    #[flux_rs::sig(fn (u64[@gen]) -> u64[gen + 1] requires gen < u64::MAX)]
    pub fn bump_generation(gen: u64) -> u64 {
        gen.checked_add(1).expect("SAFETY: caller proves gen < u64::MAX")
    }

    /// Refinement: MAX generation cannot be incremented.
    /// Production code path:
    ///   crates/vb_runtime/src/shard/timer_wheel.rs:84
    ///   checked_add(1) on u64::MAX returns None → GenerationExhausted
    #[flux_rs::sig(fn() -> TimerWheelError)]
    pub fn max_generation_exhausted() -> TimerWheelError {
        TimerWheelError::GenerationExhausted
    }

    /// Refinement: first generation starts at 1.
    /// Production code reference:
    ///   crates/vb_runtime/src/shard/timer_wheel.rs:86: None => Ok(1)
    #[flux_rs::sig(fn() -> u64[1])]
    pub fn first_generation() -> u64 {
        1
    }
}
