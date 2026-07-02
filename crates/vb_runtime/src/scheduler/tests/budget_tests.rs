#![forbid(unsafe_code)]
//! Budget tests for the seeded autonomous scheduler facade.
//!
//! `max_steps` / `max_ticks` budget enforcement, configuration
//! validation, and `run_to_completion` happy-path termination live
//! here.

use std::num::NonZeroU32;

use crate::runtime::Runtime;
use crate::scheduler::config::{SchedulerConfig, SeededScheduler};
use crate::scheduler::error::SchedulerError;
use crate::scheduler::tests::fixtures::{make_config, make_runtime};
use crate::scheduler::types::{
    BoundaryChoice, BoundaryDecision, BoundaryPolicy, MAX_STEP_TICK_BUDGET, RunEndReason,
};

#[test]
fn scheduler_respects_max_steps_budget() {
    // Configure a 2-step budget; the third `tick_shard` call must
    // return a typed `StepBudgetExhausted` error, not panic.
    let config = SchedulerConfig {
        seed: 0xDEAD_BEEF_CAFE_F00D,
        max_steps: 2,
        max_ticks: 64,
        boundary_policy: BoundaryPolicy::First,
    };
    let nz = NonZeroU32::new(1).expect("1 is non-zero");
    let mut scheduler = SeededScheduler::new(make_runtime(), config, nz)
        .unwrap_or_else(|_| SeededScheduler::new_unchecked(make_runtime(), config, nz));

    let first = scheduler.tick_shard(0, BoundaryChoice::Free);
    assert!(matches!(first, Ok(BoundaryDecision::Advance)));
    let second = scheduler.tick_shard(0, BoundaryChoice::Free);
    assert!(matches!(second, Ok(BoundaryDecision::Advance)));
    let third = scheduler.tick_shard(0, BoundaryChoice::Free);
    assert_eq!(
        third,
        Err(SchedulerError::StepBudgetExhausted { budget: 2 }),
        "third tick must trip the configured 2-step budget"
    );
}

#[test]
fn scheduler_can_run_to_completion() {
    // Happy path: generous budgets, `First` policy, expect a clean
    // Ok(RunResult) return — either `Completed` or a budget stop
    // (but NOT a fail decision under `First`).
    let config = SchedulerConfig {
        seed: 0xDEAD_BEEF_CAFE_F00D,
        max_steps: 16,
        max_ticks: 16,
        boundary_policy: BoundaryPolicy::First,
    };
    let nz = NonZeroU32::new(1).expect("1 is non-zero");
    let mut scheduler = SeededScheduler::new(make_runtime(), config, nz)
        .unwrap_or_else(|_| SeededScheduler::new_unchecked(make_runtime(), config, nz));
    let result = scheduler.run_to_completion();
    let run = result.unwrap_or(crate::scheduler::types::RunResult {
        completed: false,
        ticks_executed: 0,
        steps_executed: 0,
        reason: RunEndReason::FailedDecision,
    });
    // Under `First` policy with `Free` choices, no Fail decision is
    // emitted; the run must terminate on a budget or on
    // natural completion. We assert it terminates within budgets.
    assert!(
        matches!(
            run.reason,
            RunEndReason::StepBudgetExhausted { .. }
                | RunEndReason::TickBudgetExhausted { .. }
                | RunEndReason::Completed
        ),
        "run_to_completion must terminate within budgets; got {:?}",
        run.reason
    );
    assert!(run.ticks_executed <= config.max_ticks);
    assert!(run.steps_executed <= config.max_steps);
}

#[test]
fn config_zero_max_steps_rejected() {
    let bad = SchedulerConfig {
        seed: 0,
        max_steps: 0,
        max_ticks: 8,
        boundary_policy: BoundaryPolicy::First,
    };
    let nz = NonZeroU32::new(1).expect("1 is non-zero");
    let err = SeededScheduler::new(make_runtime(), bad, nz);
    assert!(matches!(
        err,
        Err(SchedulerError::InvalidConfig {
            code: "max_steps_must_be_nonzero"
        })
    ));
}

#[test]
fn config_zero_max_ticks_rejected() {
    let bad = SchedulerConfig {
        seed: 0,
        max_steps: 8,
        max_ticks: 0,
        boundary_policy: BoundaryPolicy::First,
    };
    let nz = NonZeroU32::new(1).expect("1 is non-zero");
    let err = SeededScheduler::new(make_runtime(), bad, nz);
    assert!(matches!(
        err,
        Err(SchedulerError::InvalidConfig {
            code: "max_ticks_must_be_nonzero"
        })
    ));
}

#[test]
fn config_oversized_budget_rejected() {
    let bad = SchedulerConfig {
        seed: 0,
        max_steps: MAX_STEP_TICK_BUDGET.saturating_add(1),
        max_ticks: 8,
        boundary_policy: BoundaryPolicy::First,
    };
    let nz = NonZeroU32::new(1).expect("1 is non-zero");
    let err = SeededScheduler::new(make_runtime(), bad, nz);
    assert!(matches!(
        err,
        Err(SchedulerError::InvalidConfig {
            code: "max_steps_exceeds_MAX_STEP_TICK_BUDGET"
        })
    ));
}

#[test]
fn shard_out_of_range_is_typed_error() {
    let config = make_config(0xDEAD_BEEF_CAFE_F00D, BoundaryPolicy::First);
    let nz = NonZeroU32::new(1).expect("1 is non-zero");
    let mut scheduler = SeededScheduler::new(make_runtime(), config, nz)
        .unwrap_or_else(|_| SeededScheduler::new_unchecked(make_runtime(), config, nz));
    // Runtime has 1 shard; index 7 is out of range.
    let err = scheduler.tick_shard(7, BoundaryChoice::Free);
    assert!(matches!(
        err,
        Err(SchedulerError::ShardOutOfRange {
            requested: 7,
            shard_count: 1
        })
    ));
}

#[allow(dead_code)]
fn _runtime_anchor(_: &Runtime) {}
