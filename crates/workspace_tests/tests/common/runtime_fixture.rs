#![allow(dead_code)] // Per-target subsets: each test root imports only the builders it calls.
//! Shared runtime/config/tick fixtures for workspace_tests targets.
//!
//! Hoisted from byte-identical per-file copies (see bead vb-xqcaf wave 2).
//! `test_config` roots were renamed to `relaxed_config` at call sites;
//! bodies were identical (32/64/16/8/Relaxed).

use std::num::NonZeroUsize;

use vb_core::action::ActionTicket;
use vb_core::ids::{RunId, StepIdx};
use vb_core::policy::RuntimePolicy;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;

pub(crate) fn shard_count(value: usize) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(value).ok_or_else(|| format!("expected non-zero shard count, got {value}"))
}

pub(crate) fn relaxed_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 16,
        max_active_runs: 8,
        policy: RuntimePolicy::Relaxed,
    }
}

pub(crate) fn strict_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 16,
        max_active_runs: 8,
        policy: RuntimePolicy::Strict,
    }
}

pub(crate) fn tick_count(runtime: &mut Runtime, count: usize) -> Result<(), String> {
    for _ in 0..count {
        assert_eq!(
            runtime.tick_all(),
            Ok(true),
            "tick_all should return true while draining queued commands"
        );
    }
    Ok(())
}

pub(crate) fn run_one_tick(runtime: &mut Runtime) -> Result<(), String> {
    assert_eq!(runtime.tick_all(), Ok(true));
    Ok(())
}

pub(crate) fn action_ticket(runtime: &Runtime, run: RunId) -> Result<ActionTicket, String> {
    runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or_else(|| format!("run {run:?} has no pending action ticket for step 0"))
}

pub(crate) fn tick_and_drain(
    runtime: &mut Runtime,
) -> Result<Vec<vb_runtime::trace::TraceEvent>, String> {
    assert_eq!(
        runtime.tick_all(),
        Ok(true),
        "tick_all should return true when shards alive"
    );
    Ok(Vec::new())
}
