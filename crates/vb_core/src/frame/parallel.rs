//! Parallel in-flight branch tracking for `RunFrame`.
//!
//! - `set_max_parallel_in_flight()` — sets the concurrency ceiling.
//! - `add_parallel_in_flight()` — increments with overflow and ceiling guard.
//! - `sub_parallel_in_flight()` — decrements with underflow guard.

use crate::errors::{CoreError, CoreResult};

use super::run_frame::RunFrame;

impl RunFrame {
    /// Sets the maximum allowed parallel in-flight branches.
    pub fn set_max_parallel_in_flight(&mut self, limit: u16) {
        self.max_parallel_in_flight = limit;
    }

    /// Adds to the parallel in-flight counter, enforcing the configured ceiling.
    ///
    /// Returns `CoreError::BudgetExceeded` when the new total would exceed the
    /// `max_parallel_in_flight` ceiling set by [`Self::set_max_parallel_in_flight`].
    /// The ceiling is *not* ratcheted upward: a configured limit is normative,
    /// not a high-water mark (CF-001).
    ///
    /// Also returns `CoreError::InternalInvariantViolation` on `u16` overflow.
    pub fn add_parallel_in_flight(&mut self, count: u16) -> CoreResult<()> {
        let new_total = self.parallel_in_flight.checked_add(count).ok_or(
            CoreError::InternalInvariantViolation {
                reason: "parallel_in_flight overflow",
            },
        )?;
        if new_total > self.max_parallel_in_flight {
            return Err(CoreError::BudgetExceeded {
                budget: "parallel_in_flight",
                limit: u64::from(self.max_parallel_in_flight),
            });
        }
        self.parallel_in_flight = new_total;
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
