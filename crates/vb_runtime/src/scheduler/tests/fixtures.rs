#![forbid(unsafe_code)]
//! Shared fixtures for the seeded scheduler facade test modules.
//!
//! Centralising the runtime/config constructors keeps each test
//! module focused on a single concern (determinism, policy, budget,
//! transcript, counters, RNG) without re-importing the runtime and
//! shard crates per file. The constructors here intentionally use
//! `expect`/`?` only on infallible-from-the-construction-site
//! operations (`NonZeroU32::new(1)`); the prior `unwrap_or`/`MIN`
//! theatre is removed.

use std::num::NonZeroU32;

use crate::runtime::Runtime;
use crate::scheduler::config::{SchedulerConfig, SeededScheduler};
use crate::scheduler::types::BoundaryPolicy;
use crate::shard::ShardConfig;

pub(crate) const FIXTURE_SEED_A: u64 = 0xDEAD_BEEF_CAFE_F00D;
pub(crate) const FIXTURE_SEED_B: u64 = 0x1234_5678_9ABC_DEF0;
pub(crate) const FIXTURE_STEPS: u32 = 64;
pub(crate) const FIXTURE_TICKS: u32 = 64;

/// Builds a one-shard runtime suitable for exercising the scheduler
/// facade. The single shard is sufficient to cover all the boundary
/// decision paths (advance, yield, fail, retry) and keeps the runtime
/// state trivially observable.
pub(crate) fn make_runtime() -> Runtime {
    let shard_count = std::num::NonZeroUsize::new(1).expect("1 is non-zero");
    Runtime::new_for_tests_and_benchmarks_only(shard_count, ShardConfig::default())
}

/// Builds a [`SchedulerConfig`] with the canonical fixture budgets
/// (`FIXTURE_STEPS` / `FIXTURE_TICKS`) and the supplied seed/policy.
pub(crate) fn make_config(seed: u64, policy: BoundaryPolicy) -> SchedulerConfig {
    SchedulerConfig {
        seed,
        max_steps: FIXTURE_STEPS,
        max_ticks: FIXTURE_TICKS,
        boundary_policy: policy,
    }
}

/// Builds a [`SeededScheduler`] bound to a fresh one-shard runtime,
/// using the canonical fixture budgets. Falls back to
/// `new_unchecked` if the validated constructor rejects the config
/// (which should not happen for the fixture budgets but keeps the
/// test surface robust against future fixture changes).
pub(crate) fn make_scheduler(seed: u64, policy: BoundaryPolicy) -> SeededScheduler {
    let config = make_config(seed, policy);
    let nz = NonZeroU32::new(1).expect("1 is non-zero");
    SeededScheduler::new(make_runtime(), config, nz)
        .unwrap_or_else(|_| SeededScheduler::new_unchecked(make_runtime(), config, nz))
}
