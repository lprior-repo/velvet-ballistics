#![forbid(unsafe_code)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::expect_used,
    clippy::get_first,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
//! PS-004: Generation Exhaustion — behavior tests (D1-D4).
//!
//! Tests `Shard::next_pending_timer_generation` behavior:
//! - Returns `Some(1)` for runs with no pending timer
//! - Returns `Some(g+1)` for runs with existing pending timer
//! - Returns `None` when generation would overflow u64::MAX
//! - Does not mutate state on query

use vb_core::ids::RunId;
use vb_runtime::shard::types::{Shard, ShardConfig};

fn run(id: u64) -> RunId {
    RunId::new(id)
}

// ---------- Behavior D2: next_generation returns Some(1) for new run ----------

#[test]
fn next_pending_timer_generation_returns_one_for_new_run() {
    let shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.next_pending_timer_generation(run(1)), Some(1));
}

#[test]
fn next_pending_timer_generation_returns_one_for_different_new_runs() {
    let shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.next_pending_timer_generation(run(42)), Some(1));
    assert_eq!(shard.next_pending_timer_generation(run(99)), Some(1));
    assert_eq!(shard.next_pending_timer_generation(run(7)), Some(1));
}

#[test]
fn next_pending_timer_generation_always_starts_at_one_for_new_runs() {
    let shard = Shard::new(ShardConfig::default());
    // Multiple queries on a new run all return Some(1)
    assert_eq!(shard.next_pending_timer_generation(run(1)), Some(1));
    assert_eq!(shard.next_pending_timer_generation(run(1)), Some(1));
}

#[test]
fn next_pending_timer_generation_returns_one_for_run_zero() {
    let shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.next_pending_timer_generation(run(0)), Some(1));
}

// ---------- Behavior D1: next_generation increments for existing timer ----------
// NOTE: Since we cannot directly insert pending timers from integration tests
// (pub(crate) access), we exercise the public API's "no timer" path here.
// The full insertion + increment test is in the crate-level unit tests.

#[test]
fn next_pending_timer_generation_is_pure_query_no_mutation() {
    let shard = Shard::new(ShardConfig::default());
    // Querying does not create a pending timer
    assert_eq!(shard.next_pending_timer_generation(run(1)), Some(1));
    assert_eq!(shard.next_pending_timer_generation(run(1)), Some(1));
    // pending_timer_count should still be 0
    assert_eq!(shard.pending_timer_count(), 0);
}

// ---------- Behavior D3: Generation exhaustion at u64::MAX ----------
// NOTE: Exhaustion behavior when generation is u64::MAX is tested
// in the crate-level unit tests with pub(crate) access to pending_timers.

// ---------- Cross-run independence ----------

#[test]
fn next_pending_timer_generation_is_independent_of_other_runs() {
    let shard = Shard::new(ShardConfig::default());
    // All fresh runs start at 1
    for i in 0..100u64 {
        assert_eq!(shard.next_pending_timer_generation(run(i)), Some(1));
    }
}

#[test]
fn next_pending_timer_generation_handles_max_run_id() {
    let shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.next_pending_timer_generation(run(u64::MAX)), Some(1));
}

// ---------- Shard creation preserves generation baseline ----------

#[test]
fn shard_new_has_no_pending_timers_and_returns_generation_one() {
    let shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.pending_timer_count(), 0);
    assert_eq!(shard.next_pending_timer_generation(run(1)), Some(1));
}

#[test]
fn shard_new_with_custom_config_returns_generation_one() {
    let config = ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 128,
        step_budget_per_tick: 100,
        max_active_runs: 16,
        policy: vb_core::policy::RuntimePolicy::Strict,
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.next_pending_timer_generation(run(1)), Some(1));
}
