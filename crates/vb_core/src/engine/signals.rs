//! Engine signal types and step budget.

use crate::errors::EngineError;
use crate::limits::MAX_STEP_BUDGET;
use crate::value::{SlotValue, Taint};

/// Bounded number of steps a caller may execute in one engine slice.
///
/// The hard ceiling is [`MAX_STEP_BUDGET`]. Any value provided to [`StepBudget::new`]
/// that exceeds this ceiling is clamped, and [`StepBudget::try_take`] returns an
/// error if the internal counter somehow exceeds the ceiling.
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
}

/// Outcome of one or more engine transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineSignal {
    /// The run made progress and can continue immediately.
    Continue,
    /// The run finished with a result value and its taint level.
    Finished(SlotValue, Taint),
    /// The caller's execution slice ended before completion.
    StepBudgetExhausted,
    /// The run suspended on an action.
    AwaitingAction,
    /// The run suspended on wait.
    AwaitingWait,
    /// The run suspended on ask.
    AwaitingAsk,
}
