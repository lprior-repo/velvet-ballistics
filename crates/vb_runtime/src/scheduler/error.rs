#![forbid(unsafe_code)]
//! Typed errors for the seeded scheduler facade.

use core::fmt;

/// Typed scheduler errors. All variants are fallible; production code
/// must never panic on a scheduler condition.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchedulerError {
    /// The configured `max_steps` budget has been exhausted.
    ///
    /// The scheduler stopped advancing because the configured
    /// [`crate::scheduler::SchedulerConfig::max_steps`] cap was reached.
    /// Callers can extend the budget, raise the cap, or accept the
    /// partial transcript.
    StepBudgetExhausted {
        /// Configured budget at the time of exhaustion.
        budget: u32,
    },
    /// The configured `max_ticks` budget has been exhausted.
    ///
    /// Distinct from [`Self::StepBudgetExhausted`] because callers may
    /// choose to bound ticks separately from steps for replay tuning.
    TickBudgetExhausted {
        /// Configured budget at the time of exhaustion.
        budget: u32,
    },
    /// Shard index is out of range for the configured runtime.
    ShardOutOfRange {
        /// Requested shard index.
        requested: u32,
        /// Configured shard count.
        shard_count: u32,
    },
    /// Underlying runtime error propagated from
    /// [`crate::Runtime::tick_all`] or [`crate::Runtime::tick_shard`].
    Runtime {
        /// Static code identifying the runtime failure mode.
        code: &'static str,
        /// Human-readable detail preserved from the runtime error.
        detail: String,
    },
    /// Boundary decision budget counter overflow.
    ///
    /// The internal `u32` decision counter would have overflowed.
    /// This is structurally unreachable under the configured
    /// `max_steps`/`max_ticks` budgets (both bounded well below
    /// `u32::MAX`), but is reported as a typed error rather than
    /// silently wrapping or panicking.
    DecisionCounterOverflow,
    /// Configuration rejected at construction.
    InvalidConfig {
        /// Static code identifying the configuration failure mode.
        code: &'static str,
    },
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StepBudgetExhausted { budget } => {
                write!(f, "scheduler step budget exhausted at {budget}")
            }
            Self::TickBudgetExhausted { budget } => {
                write!(f, "scheduler tick budget exhausted at {budget}")
            }
            Self::ShardOutOfRange {
                requested,
                shard_count,
            } => {
                write!(
                    f,
                    "shard index {requested} out of range (shard_count={shard_count})"
                )
            }
            Self::Runtime { code, detail } => {
                write!(f, "runtime error [{code}]: {detail}")
            }
            Self::DecisionCounterOverflow => {
                write!(f, "scheduler decision counter overflow")
            }
            Self::InvalidConfig { code } => {
                write!(f, "invalid scheduler config: {code}")
            }
        }
    }
}

impl std::error::Error for SchedulerError {}
