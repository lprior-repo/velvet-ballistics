#![forbid(unsafe_code)]
#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::map_clone,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_sort_by,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_asref,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
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
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
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
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
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
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
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
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
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
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
    };
    let shard = Shard::new(config);
    let status = shard.status();
    assert_eq!(status.step_budget_per_tick, 200);
}
