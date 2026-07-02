#![forbid(unsafe_code)]
//! Public types for the seeded scheduler facade.

use vb_core::ids::StepIdx;

use crate::RuntimeError;

/// Maximum allowed value for `SchedulerConfig::max_steps` and
/// `SchedulerConfig::max_ticks`. Both budgets are stored as `u32`; this
/// constant rejects impossible budgets at construction so that the
/// scheduler cannot enter a state where its internal counters could
/// silently overflow.
pub(crate) const MAX_STEP_TICK_BUDGET: u32 = u32::MAX / 2;

/// Policy for selecting a [`BoundaryDecision`] from the available
/// [`BoundaryChoice`] candidates.
///
/// All three policies are deterministic when the seed and the input
/// boundary choice sequence are fixed. Only [`Self::Random`] depends
/// on the seed value; [`Self::First`] is seed-independent by design,
/// and [`Self::RoundRobin`] depends only on the count of decisions
/// issued so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoundaryPolicy {
    /// Always pick the first candidate. Seed-independent.
    First,
    /// Pick deterministically based on the scheduler's PRNG state.
    /// Two different seeds for the same input stream produce two
    /// different decision streams.
    Random,
    /// Cycle through candidates in order. Seed-independent.
    RoundRobin,
}

/// Decision returned by the scheduler for a given [`BoundaryChoice`].
///
/// This is the canonical Antithesis-style boundary outcome: the
/// scheduler picks exactly one variant per decision, and the variant
/// is reproducible from the seed and the choice sequence.
///
/// Note: `BoundaryDecision` is `Clone` (not `Copy`) because the
/// `Fail { variant }` arm carries a [`RuntimeError`] which is itself
/// `Clone` only.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoundaryDecision {
    /// Advance to the next step on the same shard.
    Advance,
    /// Yield control to a different step (used for retries/redirects).
    Yield {
        /// Target step index to yield to.
        to_step: StepIdx,
    },
    /// Fail the current run with a typed runtime error variant.
    Fail {
        /// Runtime error variant surfaced as the failure.
        variant: RuntimeError,
    },
    /// Retry the current step after a bounded delay.
    Retry {
        /// Delay in scheduler ticks before retrying.
        delay_ticks: u32,
    },
}

/// Input to the scheduler's decision function.
///
/// The caller (runtime, test, replay harness) describes the boundary
/// surface it is currently facing. The scheduler returns one
/// [`BoundaryDecision`] chosen deterministically from the candidates
/// implied by the choice.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoundaryChoice {
    /// Free-form boundary: the scheduler may pick any of the four
    /// decision variants.
    Free,
    /// Only an [`BoundaryDecision::Advance`] outcome is acceptable.
    AdvanceOnly,
    /// Only an [`BoundaryDecision::Yield`] outcome is acceptable;
    /// caller supplies the candidate target step.
    YieldOnly {
        /// Target step index for the yield.
        to_step: StepIdx,
    },
    /// Only an [`BoundaryDecision::Fail`] outcome is acceptable;
    /// caller supplies the candidate variant.
    FailOnly {
        /// Runtime error variant for the failure.
        variant: RuntimeError,
    },
    /// Only an [`BoundaryDecision::Retry`] outcome is acceptable;
    /// caller supplies the candidate delay.
    RetryOnly {
        /// Delay in scheduler ticks before retrying.
        delay_ticks: u32,
    },
}

/// Reason `run_to_completion` stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RunEndReason {
    /// All shards reported shutdown via `Runtime::tick_all` returning
    /// `Ok(false)`. The runtime is quiescent.
    Completed,
    /// The configured `max_steps` budget was reached.
    StepBudgetExhausted {
        /// Configured budget at the time of exhaustion.
        budget: u32,
    },
    /// The configured `max_ticks` budget was reached.
    TickBudgetExhausted {
        /// Configured budget at the time of exhaustion.
        budget: u32,
    },
    /// A [`BoundaryDecision::Fail`] was emitted by the scheduler.
    FailedDecision,
}

/// Aggregate outcome of a `run_to_completion` invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunResult {
    /// Whether the runtime reached natural completion (all shards
    /// shut down) before any budget was exhausted.
    pub completed: bool,
    /// Number of ticks issued to the runtime.
    pub ticks_executed: u32,
    /// Number of steps executed by the scheduler (one per
    /// `tick_shard` / `tick_all` invocation, before the budget guard).
    pub steps_executed: u32,
    /// Reason `run_to_completion` stopped.
    pub reason: RunEndReason,
}
