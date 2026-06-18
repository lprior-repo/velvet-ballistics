//! Parallel in-flight branch tracking for `RunFrame`.
//!
//! - `set_max_parallel_in_flight()` — sets the concurrency ceiling.
//! - `add_parallel_in_flight()` — increments with overflow guard.
//! - `sub_parallel_in_flight()` — decrements with underflow guard.

use crate::errors::{CoreError, CoreResult};

use super::run_frame::RunFrame;

impl RunFrame {
    /// Sets the maximum allowed parallel in-flight branches.
    pub fn set_max_parallel_in_flight(&mut self, limit: u16) {
        self.max_parallel_in_flight = limit;
    }

    /// Adds to the parallel in-flight counter and updates max_parallel_in_flight
    /// if the new total exceeds the previous maximum.
    pub fn add_parallel_in_flight(&mut self, count: u16) -> CoreResult<()> {
        self.parallel_in_flight = self.parallel_in_flight.checked_add(count).ok_or(
            CoreError::InternalInvariantViolation {
                reason: "parallel_in_flight overflow",
            },
        )?;
        if self.parallel_in_flight > self.max_parallel_in_flight {
            self.max_parallel_in_flight = self.parallel_in_flight;
        }
        Ok(())
    }

    /// Subtracts from the parallel in-flight counter.
    pub fn sub_parallel_in_flight(&mut self, count: u16) -> CoreResult<()> {
        self.parallel_in_flight = self.parallel_in_flight.checked_sub(count).ok_or(
            CoreError::InternalInvariantViolation {
                reason: "parallel_in_flight underflow",
            },
        )?;
        Ok(())
    }
}
