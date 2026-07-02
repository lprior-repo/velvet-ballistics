#![forbid(unsafe_code)]
//! Configuration + accessor methods for [`SeededScheduler`].
//!
//! This file is part of the `scheduler` module. It contains the
//! `SchedulerConfig` validation, the `SeededScheduler` field
//! definitions, the constructor (with and without validation),
//! and the public accessors.

use core::num::NonZeroU32;

use crate::runtime::Runtime;
use crate::scheduler::error::SchedulerError;
use crate::scheduler::rng::RngState;
use crate::scheduler::transcript::BoundaryTranscript;
use crate::scheduler::types::{BoundaryPolicy, MAX_STEP_TICK_BUDGET};

/// Configuration for [`SeededScheduler`]. All fields are required;
/// there is no implicit default seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerConfig {
    /// Deterministic seed for the scheduler's splitmix64 PRNG.
    pub seed: u64,
    /// Maximum number of `tick_shard` / `tick_all` calls permitted
    /// before [`SchedulerError::StepBudgetExhausted`] is raised.
    /// Must be > 0 and ≤ [`MAX_STEP_TICK_BUDGET`].
    pub max_steps: u32,
    /// Maximum number of ticks permitted in a single
    /// `run_to_completion` call before
    /// [`SchedulerError::TickBudgetExhausted`] is raised.
    /// Must be > 0 and ≤ [`MAX_STEP_TICK_BUDGET`].
    pub max_ticks: u32,
    /// Policy used by [`SeededScheduler::decide_boundary`] to pick a
    /// [`crate::scheduler::BoundaryDecision`] from the available
    /// [`crate::scheduler::BoundaryChoice`] candidates.
    pub boundary_policy: BoundaryPolicy,
}

impl SchedulerConfig {
    /// Validates the configuration against the scheduler's invariants.
    const fn validate(&self) -> Result<(), SchedulerError> {
        if self.max_steps == 0 {
            return Err(SchedulerError::InvalidConfig {
                code: "max_steps_must_be_nonzero",
            });
        }
        if self.max_ticks == 0 {
            return Err(SchedulerError::InvalidConfig {
                code: "max_ticks_must_be_nonzero",
            });
        }
        if self.max_steps > MAX_STEP_TICK_BUDGET {
            return Err(SchedulerError::InvalidConfig {
                code: "max_steps_exceeds_MAX_STEP_TICK_BUDGET",
            });
        }
        if self.max_ticks > MAX_STEP_TICK_BUDGET {
            return Err(SchedulerError::InvalidConfig {
                code: "max_ticks_exceeds_MAX_STEP_TICK_BUDGET",
            });
        }
        Ok(())
    }
}

/// Seeded autonomous scheduler.
///
/// Owns the runtime being driven and the splitmix64 PRNG used to make
/// boundary decisions. Construct with [`Self::new`] (validates config)
/// or [`Self::new_unchecked`] (skips validation; callers must ensure
/// invariants hold).
pub struct SeededScheduler {
    pub(crate) runtime: Runtime,
    pub(crate) seed: u64,
    pub(crate) rng_state: RngState,
    pub(crate) step_count: u32,
    pub(crate) decision_count: u32,
    pub(crate) max_steps: u32,
    pub(crate) max_ticks: u32,
    pub(crate) shard_count: u32,
    pub(crate) boundary_policy: BoundaryPolicy,
    pub(crate) transcript: BoundaryTranscript,
    /// Round-robin cursor over the four decision variants. Used by
    /// [`BoundaryPolicy::RoundRobin`]; ignored by the other policies.
    pub(crate) round_robin_cursor: u8,
}

impl SeededScheduler {
    /// Creates a new scheduler bound to `runtime`. Validates the
    /// configuration; returns a typed [`SchedulerError::InvalidConfig`]
    /// on any rejected invariant.
    ///
    /// `shard_count` must equal the number of shards the runtime was
    /// constructed with. The scheduler uses it to bounds-check
    /// `tick_shard` calls.
    pub fn new(
        runtime: Runtime,
        config: SchedulerConfig,
        shard_count: NonZeroU32,
    ) -> Result<Self, SchedulerError> {
        config.validate()?;
        Ok(Self::new_unchecked(runtime, config, shard_count))
    }

    /// Creates a new scheduler without validating the configuration.
    ///
    /// Callers must ensure `config.max_steps > 0`, `config.max_ticks > 0`,
    /// both budgets are ≤ [`MAX_STEP_TICK_BUDGET`], and `shard_count`
    /// matches the runtime's shard count.
    #[must_use]
    pub fn new_unchecked(
        runtime: Runtime,
        config: SchedulerConfig,
        shard_count: NonZeroU32,
    ) -> Self {
        Self {
            runtime,
            seed: config.seed,
            rng_state: RngState::new(config.seed),
            step_count: 0,
            decision_count: 0,
            max_steps: config.max_steps,
            max_ticks: config.max_ticks,
            shard_count: shard_count.get(),
            boundary_policy: config.boundary_policy,
            transcript: BoundaryTranscript::new(),
            round_robin_cursor: 0,
        }
    }

    /// Returns the configured seed.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// Returns the number of `tick_shard` / `tick_all` calls performed
    /// since this scheduler was constructed.
    #[must_use]
    pub const fn step_count(&self) -> u32 {
        self.step_count
    }

    /// Returns the number of `decide_boundary` calls performed since
    /// this scheduler was constructed.
    #[must_use]
    pub const fn decision_count(&self) -> u32 {
        self.decision_count
    }

    /// Returns the configured maximum step budget.
    #[must_use]
    pub const fn max_steps(&self) -> u32 {
        self.max_steps
    }

    /// Returns the configured maximum tick budget.
    #[must_use]
    pub const fn max_ticks(&self) -> u32 {
        self.max_ticks
    }

    /// Returns the configured shard count.
    #[must_use]
    pub const fn shard_count(&self) -> u32 {
        self.shard_count
    }

    /// Returns the configured boundary policy.
    #[must_use]
    pub const fn boundary_policy(&self) -> BoundaryPolicy {
        self.boundary_policy
    }

    /// Returns the current PRNG state (mainly for diagnostics/tests).
    #[must_use]
    pub const fn rng_state(&self) -> u64 {
        self.rng_state.raw_state()
    }

    /// Returns the accumulated boundary transcript.
    #[must_use]
    pub fn transcript(&self) -> &BoundaryTranscript {
        &self.transcript
    }

    /// Returns a reference to the inner runtime. Provided so callers
    /// can submit runs or inspect state without taking ownership of
    /// the scheduler.
    #[must_use]
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// Returns a mutable reference to the inner runtime. Used by
    /// `tick_shard` / `tick_all` and tests that need to enqueue
    /// commands between scheduler ticks.
    ///
    /// Note: direct mutation bypasses the scheduler's decision
    /// bookkeeping. Tests that mix runtime mutation with
    /// `decide_boundary` should not assume byte-identical transcripts.
    #[must_use]
    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }
}
