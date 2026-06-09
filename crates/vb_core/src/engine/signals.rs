#![forbid(unsafe_code)]
//! Engine signal types and step budget.

use crate::errors::EngineError;
use crate::ids::SlotIdx;
use crate::limits::MAX_STEP_BUDGET;
use crate::value::{SlotValue, Taint};

/// Bounded number of steps a caller may execute in one engine slice.
///
/// The hard ceiling is [`MAX_STEP_BUDGET`]. Any value provided to [`StepBudget::new`]
/// that exceeds this ceiling is clamped, and [`StepBudget::try_take`] returns an
/// error if the internal counter somehow exceeds the ceiling.
#[derive(Debug, Clone)]
pub struct StepBudget {
    remaining: u64,
}

impl StepBudget {
    /// Largest bounded execution slice representable by the runtime.
    pub const MAX: Self = Self {
        remaining: MAX_STEP_BUDGET,
    };

    /// Creates a budget clamped to the hard ceiling. Zero is valid and
    /// executes no transitions.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self {
            remaining: if value > MAX_STEP_BUDGET {
                MAX_STEP_BUDGET
            } else {
                value
            },
        }
    }

    /// Attempts to consume one transition from the budget.
    ///
    /// Returns `Ok(true)` when a transition was consumed, `Ok(false)` when the
    /// budget is exhausted, and an error if the remaining counter somehow
    /// exceeds the hard ceiling (a runtime invariant violation).
    ///
    /// # Invariant guarantee
    ///
    /// The `remaining > MAX_STEP_BUDGET` overflow check is kept as a defense-in-depth
    /// guard. The field is private and can only be set by [`Self::new`] (which clamps)
    /// or [`Self::MAX`] (which uses `MAX_STEP_BUDGET` directly), and `try_take` only
    /// ever decreases via `saturating_sub`. The check therefore cannot fire through
    /// normal API use, but guards against future modifications or memory corruption.
    pub fn try_take(&mut self) -> Result<bool, EngineError> {
        if self.remaining > MAX_STEP_BUDGET {
            return Err(EngineError::StepCounterOverflow);
        }
        if self.remaining == 0 {
            Ok(false)
        } else {
            self.remaining = self.remaining.saturating_sub(1);
            Ok(true)
        }
    }

    /// Remaining transitions.
    #[must_use]
    pub const fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Environment variable name for bench latency budget.
    const BENCH_LATENCY_BUDGET_US: &'static str = "VB_BENCH_LATENCY_BUDGET_US";

    /// Default budget when env var is absent.
    const DEFAULT_BUDGET: u64 = MAX_STEP_BUDGET;

    /// Creates a budget from the VB_BENCH_LATENCY_BUDGET_US environment variable.
    ///
    /// - Returns `Ok(budget)` when env var is set and parses as u64.
    ///   Value is clamped to [`MAX_STEP_BUDGET`].
    /// - Returns `Err(EngineError::BudgetParse { reason })` when env var is set
    ///   but contains non-numeric content.
    /// - Returns `Ok(Self::new(Self::DEFAULT_BUDGET))` when env var is absent.
    pub fn from_env() -> Result<Self, EngineError> {
        match std::env::var(Self::BENCH_LATENCY_BUDGET_US) {
            Ok(raw) => {
                let parsed = raw.parse::<u64>().map_err(|_| EngineError::BudgetParse {
                    reason: "invalid u64 value",
                })?;
                Ok(Self::new(parsed))
            }
            Err(std::env::VarError::NotPresent) => Ok(Self::new(Self::DEFAULT_BUDGET)),
            Err(_) => Err(EngineError::BudgetParse {
                reason: "env var access error",
            }),
        }
    }
}

/// Outcome of one or more engine transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EngineSignal {
    /// The run made progress and can continue immediately.
    Continue,
    /// The run finished with a result value and its taint level.
    Finished(SlotValue, Taint),
    /// The caller's execution slice ended before completion.
    StepBudgetExhausted,
    /// The run suspended on an action.
    AwaitingAction,
    /// The run suspended on wait. Carries the slot the wait primitive
    /// read its deadline from so the host runtime can compute the
    /// concrete `Instant` deadline for the pending timer.
    AwaitingWait {
        /// Slot the wait primitive read its deadline from. The host
        /// runtime reads this slot on the suspended run frame to
        /// obtain the actual deadline value the wait validated.
        deadline_slot: SlotIdx,
    },
    /// The run suspended on ask. Carries the optional timeout slot
    /// for ask nodes that include a timeout; `None` means the ask
    /// has no timeout and only the prompt is suspended.
    AwaitingAsk {
        /// Optional slot the ask primitive read its timeout from.
        /// `None` means the ask has no timeout.
        timeout_slot: Option<SlotIdx>,
    },
}

#[cfg(test)]
#[path = "signals/tests.rs"]
mod tests;

// Verified by: OBL-VB-BUDGET-KANI-006 (overflow guard)
#[cfg(kani)]
mod kani_overflow_guard {
    use super::StepBudget;
    use crate::errors::EngineError;
    use crate::limits::MAX_STEP_BUDGET;

    #[kani::proof]
    #[kani::unwind(4)]
    fn step_budget_overflow_guard() {
        let remaining: u64 = kani::any();
        kani::assume(remaining > MAX_STEP_BUDGET);
        let mut budget = StepBudget { remaining };
        let result = budget.try_take();
        match result {
            Err(EngineError::StepCounterOverflow) => {
                kani::assert(
                    budget.remaining() == remaining,
                    "remaining unchanged after overflow error",
                );
            }
            Err(_) => {
                kani::assert(
                    false,
                    "must return StepCounterOverflow when remaining > MAX",
                );
            }
            Ok(_) => {
                kani::assert(
                    false,
                    "must return StepCounterOverflow when remaining > MAX",
                );
            }
        }
    }

    #[kani::proof]
    #[kani::unwind(4)]
    fn step_budget_no_overflow_for_valid_range() {
        let remaining: u64 = kani::any();
        kani::assume(remaining <= MAX_STEP_BUDGET);
        let mut budget = StepBudget { remaining };
        let result = budget.try_take();
        match result {
            Err(EngineError::StepCounterOverflow) => {
                kani::assert(false, "should not overflow when remaining <= MAX");
            }
            Err(_) => {}
            Ok(_) => {}
        }
    }
}
