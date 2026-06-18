#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
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
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
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
//! Test chunk 028 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 6979–7254 of the original. Semantic content is
//! preserved exactly; only the file structure changed.
//! Budget module integration tests.

use crate::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceUsage, BoundednessPolicy,
    BudgetError, WholeWorkflowBudget,
};
use crate::engine::StepBudget;
use crate::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, ExprBranch, ResourceContract, SlotBranch, WorkflowError,
};

use super::prelude::*;

#[test]
fn whole_workflow_budget_policy_at_exact_limit() -> Result<(), String> {
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        max_total_slots: 65_535,
        max_fanout: 64,
        max_nesting_depth: 8,
        absolute_max_action_tickets: 100,
        absolute_max_parallel: 10,
        absolute_max_run_time_seconds: 3600,
        absolute_max_result_bytes: 65536,
        absolute_max_steps_executable: 1000,
        absolute_max_timer_entries: 17,
        absolute_max_trace_events: 18,
        absolute_max_journal_batch_bytes: 19,
        absolute_max_queue_depth: 20,
        absolute_max_ipc_payload_bytes: 21,
        absolute_max_blob_bytes: 22,
        absolute_max_input_bytes: 23,
        ..BoundednessPolicy::DEFAULT
    };
    let usage = AggregateResourceUsage {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 17,
        max_trace_events: 18,
        max_active_runs: 1,
        max_queue_depth: 20,
        max_journal_batch_bytes: 19,
        max_ipc_payload_bytes: 21,
        max_blob_bytes: 22,
        max_input_bytes: 23,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };
    // Usage at exact limit should pass (>= comparison, not >)
    ensure_equal(usage.check_policy(&policy), Ok(()))
}

/// Kills: check_policy > with >= — tests the over-limit case to confirm
/// the error type and values are correct, preventing `>` → `==` mutation
/// (which would only fail when exactly equal, missing the over-limit case).
#[test]
fn whole_workflow_budget_policy_exceeds_limit() -> Result<(), String> {
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        max_total_slots: 65_535,
        max_fanout: 64,
        max_nesting_depth: 8,
        absolute_max_action_tickets: 100,
        absolute_max_parallel: 10,
        absolute_max_run_time_seconds: 3600,
        absolute_max_result_bytes: 65536,
        absolute_max_steps_executable: 1000,
        ..BoundednessPolicy::DEFAULT
    };
    let usage = AggregateResourceUsage {
        // Exceeds by 1
        max_steps_executable: 1001,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 1,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };
    match usage.check_policy(&policy) {
        Err(AggregateBudgetError::PolicyExceeded {
            resource: "max_steps_executable",
            actual: 1001,
            limit: 1000,
        }) => Ok(()),
        Err(e) => Err(format!(
            "expected PolicyExceeded {{resource: max_steps_executable, actual: 1001, limit: 1000}}, got {:?}",
            e
        )),
        Ok(()) => Err("expected PolicyExceeded, got Ok(())".to_string()),
    }
}

/// Kills: check_capacity > with >= — tests the exact equality boundary
/// where requested == available should succeed (not error).
#[test]
fn whole_workflow_budget_capacity_at_exact_limit() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
    };
    let mut usage = AggregateResourceUsage::default();
    // Set requested to exactly match limit
    usage.max_steps_executable = 1000;
    // Adding the budget should succeed (usage starts at 0, so result = budget values + 1000 for steps)
    let expected = AggregateResourceUsage {
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
        max_steps_executable: 2000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_active_runs: 1,
    };
    match usage.try_add_budget(&budget) {
        Ok(actual) => ensure_equal(actual, expected),
        Err(e) => Err(format!("unexpected error: {:?}", e)),
    }
}

/// Kills: validate_step_ceilings > with >= — tests over-limit case to
/// ensure `>` → `==` mutation is killed (only fails at exact equality).
#[test]
fn validate_step_ceilings_rejects_step_over_limit_by_one() -> Result<(), String> {
    // 1_000_001 is > 1_000_000, should be rejected
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 1_000_001,
        max_transitions_per_tick: 500,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
    };
    match crate::budget::validate_step_ceilings(&budget) {
        Err(AggregateBudgetError::StepCeilingExceeded {
            requested: 1_000_001,
            limit: 1_000_000,
        }) => Ok(()),
        other => Err(format!(
            "expected StepCeilingExceeded(1_000_001, 1_000_000), got {:?}",
            other
        )),
    }
}

/// Kills: validate_step_ceilings > with >= — tests transitions boundary.
#[test]
fn validate_step_ceilings_accepts_exact_transition_hard_limit() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 5000,
        max_transitions_per_tick: 1_000_000,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
    };
    ensure_equal(crate::budget::validate_step_ceilings(&budget), Ok(()))
}

/// Kills: validate_step_ceilings > with >= — over-limit for transitions.
#[test]
fn validate_step_ceilings_rejects_transitions_over_limit_by_one() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 5000,
        max_transitions_per_tick: 1_000_001,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
    };
    match crate::budget::validate_step_ceilings(&budget) {
        Err(AggregateBudgetError::PerTickCeilingExceeded {
            requested: 1_000_001,
            limit: 1_000_000,
        }) => Ok(()),
        other => Err(format!(
            "expected PerTickCeilingExceeded(1_000_001, 1_000_000), got {:?}",
            other
        )),
    }
}

// =========================================================================
// vb-tub4: Kani proof obligations - behavior tests for private arithmetic
// =========================================================================

#[test]
fn add_dim_returns_overflow_when_max_plus_requested_exceeds_u64_max() {
    // B-BUDGET-001: add_dim returns Overflow when current == u64::MAX && requested > 0
    let current = u64::MAX;
    let requested = 1u64;
    let result = crate::budget::add_dim(current, requested, "test_resource");
    assert!(result.is_err(), "add_dim must return error for overflow");
    match result {
        Err(AggregateBudgetError::Overflow { resource }) => {
            assert_eq!(resource, "test_resource");
        }
        other => panic!("expected Overflow, got {:?}", other),
    }
}
