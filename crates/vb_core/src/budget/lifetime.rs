#![forbid(unsafe_code)]
//! Wall-clock lifetime budget for one admitted run.
//!
//! The admitted limit is a duration in seconds; runtime enforcement needs an
//! absolute deadline so repeated resumes, replay, and recovery observe one
//! instant instead of restarting the clock. Every conversion and addition is
//! checked: an unbounded or overflowing declaration is rejected before use.

use crate::budget::{AggregateResourceBudget, BudgetExhaustionOutcome};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MILLIS_PER_SECOND: u64 = 1_000;

/// Absolute wall-clock lifetime budget for one admitted run.
///
/// The value is immutable. It is derived from two durable facts, the run
/// start instant and the admitted `max_run_time_seconds`, so checking it
/// again after a resume or a checkpoint round trip yields the same deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "LifetimeBudgetWire", into = "LifetimeBudgetWire")]
pub struct RunLifetimeBudget {
    started_millis: u64,
    limit_millis: u64,
    deadline_millis: u64,
}

/// Serialized form. Decoding revalidates the deadline invariant instead of
/// trusting two independently supplied field values.
#[derive(Serialize, Deserialize)]
struct LifetimeBudgetWire {
    started_millis: u64,
    limit_millis: u64,
}

/// Lifetime declaration or expiry failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum LifetimeBudgetError {
    /// The admitted seconds limit cannot be represented in milliseconds.
    #[error("run lifetime limit overflow: {max_run_time_seconds} seconds")]
    LimitOverflow {
        /// Admitted limit in seconds.
        max_run_time_seconds: u64,
    },
    /// The wall-clock deadline cannot be represented after adding the limit.
    #[error("run lifetime deadline overflow: start {started_millis} + limit {limit_millis} millis")]
    DeadlineOverflow {
        /// Run start in milliseconds since the UNIX epoch.
        started_millis: u64,
        /// Admitted limit in milliseconds.
        limit_millis: u64,
    },
    /// The observed instant is past the deadline.
    #[error("run lifetime expired: now {now_millis} > deadline {deadline_millis}")]
    Expired {
        /// Observed instant in milliseconds since the UNIX epoch.
        now_millis: u64,
        /// Absolute deadline in milliseconds since the UNIX epoch.
        deadline_millis: u64,
    },
}

impl LifetimeBudgetError {
    /// Wall-clock exhaustion never replenishes within a run, so every lifetime
    /// failure is terminal for the caller's policy.
    #[must_use]
    pub const fn outcome(&self) -> BudgetExhaustionOutcome {
        BudgetExhaustionOutcome::Fail
    }
}

impl RunLifetimeBudget {
    /// Builds the lifetime budget for a run started at `started_millis` with
    /// the admitted `max_run_time_seconds`.
    pub fn try_new(
        started_millis: u64,
        max_run_time_seconds: u64,
    ) -> Result<Self, LifetimeBudgetError> {
        let limit_millis = max_run_time_seconds.checked_mul(MILLIS_PER_SECOND).ok_or(
            LifetimeBudgetError::LimitOverflow {
                max_run_time_seconds,
            },
        )?;
        Self::from_parts(started_millis, limit_millis)
    }

    /// Builds the lifetime budget from an admitted aggregate budget.
    pub fn from_budget(
        started_millis: u64,
        budget: &AggregateResourceBudget,
    ) -> Result<Self, LifetimeBudgetError> {
        Self::try_new(started_millis, budget.max_run_time_seconds)
    }

    fn from_parts(started_millis: u64, limit_millis: u64) -> Result<Self, LifetimeBudgetError> {
        let deadline_millis = started_millis.checked_add(limit_millis).ok_or(
            LifetimeBudgetError::DeadlineOverflow {
                started_millis,
                limit_millis,
            },
        )?;
        Ok(Self {
            started_millis,
            limit_millis,
            deadline_millis,
        })
    }

    /// Run start in milliseconds since the UNIX epoch.
    #[must_use]
    pub const fn started_millis(&self) -> u64 {
        self.started_millis
    }

    /// Admitted lifetime limit in milliseconds.
    #[must_use]
    pub const fn limit_millis(&self) -> u64 {
        self.limit_millis
    }

    /// Absolute deadline in milliseconds since the UNIX epoch.
    #[must_use]
    pub const fn deadline_millis(&self) -> u64 {
        self.deadline_millis
    }

    /// Remaining lifetime at `now_millis`, floored at zero and capped at the
    /// admitted limit so a backward clock observation cannot report more time
    /// than the run was admitted.
    #[must_use]
    pub const fn remaining_millis(&self, now_millis: u64) -> u64 {
        match self.deadline_millis.checked_sub(now_millis) {
            Some(remaining) => {
                if remaining > self.limit_millis {
                    self.limit_millis
                } else {
                    remaining
                }
            }
            None => 0,
        }
    }

    /// Whether `now_millis` is past the deadline. The exact deadline is live.
    #[must_use]
    pub const fn is_expired(&self, now_millis: u64) -> bool {
        now_millis > self.deadline_millis
    }

    /// Fails when `now_millis` is past the deadline. The exact deadline is
    /// accepted; one millisecond beyond it is rejected.
    pub fn check(&self, now_millis: u64) -> Result<(), LifetimeBudgetError> {
        if self.is_expired(now_millis) {
            return Err(LifetimeBudgetError::Expired {
                now_millis,
                deadline_millis: self.deadline_millis,
            });
        }
        Ok(())
    }
}

impl TryFrom<LifetimeBudgetWire> for RunLifetimeBudget {
    type Error = LifetimeBudgetError;

    fn try_from(wire: LifetimeBudgetWire) -> Result<Self, Self::Error> {
        Self::from_parts(wire.started_millis, wire.limit_millis)
    }
}

impl From<RunLifetimeBudget> for LifetimeBudgetWire {
    fn from(budget: RunLifetimeBudget) -> Self {
        Self {
            started_millis: budget.started_millis,
            limit_millis: budget.limit_millis,
        }
    }
}
