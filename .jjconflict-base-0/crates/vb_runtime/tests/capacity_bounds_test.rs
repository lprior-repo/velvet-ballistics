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
//! PS-008: Capacity Bounds — behavior tests (H1-H3).
//!
//! Tests Shard capacity and command queue bounds alongside the numeric timer seam.
//! The Shard's command queue capacity is bounded by MAX_COMMAND_QUEUE_CAPACITY (65,536).

use vb_runtime::shard::types::{MAX_COMMAND_QUEUE_CAPACITY, Shard, ShardConfig};

// ---------- Command queue capacity bounds ----------

#[test]
fn queue_capacity_one_succeeds() {
    let config = ShardConfig {
        command_queue_capacity: 1,
        trace_capacity: 64,
        step_budget_per_tick: 100,
        max_active_runs: 16,
        policy: vb_core::policy::RuntimePolicy::Strict,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.command_queue_capacity(), 1);
}

#[test]
fn queue_capacity_max_succeeds() {
    let config = ShardConfig {
        command_queue_capacity: MAX_COMMAND_QUEUE_CAPACITY,
        trace_capacity: 64,
        step_budget_per_tick: 100,
        max_active_runs: 16,
        policy: vb_core::policy::RuntimePolicy::Strict,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.command_queue_capacity(), MAX_COMMAND_QUEUE_CAPACITY);
}

#[test]
fn queue_capacity_at_limit_accepts_enqueue() {
    let config = ShardConfig {
        command_queue_capacity: 1,
        trace_capacity: 64,
        step_budget_per_tick: 100,
        max_active_runs: 16,
        policy: vb_core::policy::RuntimePolicy::Strict,
    };
    let shard = Shard::new(config);
    assert!(shard.is_queue_full() == false);
    assert_eq!(shard.remaining_capacity(), 1);
}

// ---------- Default capacity preserves existing behavior ----------

#[test]
fn default_config_has_expected_capacity() {
    let shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.command_queue_capacity(), 1024);
}

#[test]
fn default_config_shard_is_not_full_initially() {
    let shard = Shard::new(ShardConfig::default());
    assert!(!shard.is_queue_full());
    assert_eq!(shard.command_queue_len(), 0);
}

// ---------- Pending timer count starts at zero ----------

#[test]
fn shard_pending_timer_count_starts_at_zero() {
    let shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.pending_timer_count(), 0);
}

// ---------- Active run count starts at zero ----------

#[test]
fn shard_active_run_count_starts_at_zero() {
    let shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.active_run_count(), 0);
}

// ---------- Shard shutdown state ----------

#[test]
fn shard_not_shutting_down_on_creation() {
    let shard = Shard::new(ShardConfig::default());
    assert!(!shard.is_shutting_down());
}

// ---------- Shard status reports capacity ----------

#[test]
fn shard_status_reports_command_queue_capacity() {
    let config = ShardConfig {
        command_queue_capacity: 512,
        trace_capacity: 256,
        step_budget_per_tick: 50,
        max_active_runs: 32,
        policy: vb_core::policy::RuntimePolicy::Strict,
    };
    let shard = Shard::new(config);
    let status = shard.status();
    assert_eq!(status.command_queue_capacity, 512);
    assert_eq!(status.command_queue_depth, 0);
    assert_eq!(status.active_runs, 0);
    assert_eq!(status.max_active_runs, 32);
}

#[test]
fn shard_status_reports_step_budget() {
    let config = ShardConfig {
        command_queue_capacity: 256,
        trace_capacity: 128,
        step_budget_per_tick: 200,
        max_active_runs: 8,
        policy: vb_core::policy::RuntimePolicy::Strict,
    };
    let shard = Shard::new(config);
    let status = shard.status();
    assert_eq!(status.step_budget_per_tick, 200);
}
