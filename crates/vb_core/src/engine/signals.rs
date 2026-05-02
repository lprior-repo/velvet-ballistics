//! Engine signal types and step budget.

use crate::errors::EngineError;
use crate::value::SlotValue;

/// Bounded number of steps a caller may execute in one engine slice.
pub struct StepBudget {
    remaining: u64,
}

impl StepBudget {
    /// Largest bounded execution slice representable by the runtime.
    pub const MAX: Self = Self {
        remaining: u64::MAX,
    };

    /// Creates a budget. Zero is valid and executes no transitions.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self { remaining: value }
    }

    /// Attempts to consume one transition from the budget.
    pub fn try_take(&mut self) -> Result<bool, EngineError> {
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
    /// The run finished with a result value.
    Finished(SlotValue),
    /// The caller's execution slice ended before completion.
    StepBudgetExhausted,
    /// The run suspended on an action.
    AwaitingAction,
    /// The run suspended on wait.
    AwaitingWait,
    /// The run suspended on ask.
    AwaitingAsk,
}
