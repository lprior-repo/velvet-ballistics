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
//! Test chunk 029 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 7256–7339 of the original. Semantic content is
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

#[test]
fn sub_dim_returns_underflow_when_current_is_zero() {
    // B-BUDGET-003: sub_dim returns Underflow when current == 0 && requested > 0
    let current = 0u64;
    let requested = 1u64;
    let result = crate::budget::sub_dim(current, requested, "test_resource");
    assert!(result.is_err(), "sub_dim must return error for underflow");
    match result {
        Err(AggregateBudgetError::Underflow { resource }) => {
            assert_eq!(resource, "test_resource");
        }
        other => panic!("expected Underflow, got {:?}", other),
    }
}

#[test]
fn fits_within_returns_capacity_exceeded_when_requested_greater_than_available() {
    // B-BUDGET-005: fits_within returns CapacityExceeded when requested > available
    let usage = AggregateResourceUsage {
        max_steps_executable: 100,
        ..Default::default()
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 50,
        max_action_tickets: 100,
        max_parallel_in_flight: 20,
        max_gather_pages: 10,
        max_gather_items: 200,
        max_result_bytes: 2000,
        max_total_slots_written: 1000,
        max_timer_entries: 14,
        max_trace_events: 16,
        max_active_runs: 10,
        max_queue_depth: 40,
        max_journal_batch_bytes: 8192,
        max_ipc_payload_bytes: 18,
        max_blob_bytes: 20,
        max_input_bytes: 22,
        max_step_budget_per_tick: 2000,
        max_transitions_per_tick: 1000,
    };
    let result = usage.fits_within(&capacity);
    assert!(result.is_err(), "fits_within must reject over-capacity");
    match result {
        Err(AggregateBudgetError::CapacityExceeded {
            resource,
            requested,
            available,
        }) => {
            assert_eq!(resource, "max_steps_executable");
            assert_eq!(requested, 100);
            assert_eq!(available, 50);
        }
        other => panic!("expected CapacityExceeded, got {:?}", other),
    }
}

#[test]
fn check_policy_returns_policy_exceeded_when_actual_greater_than_limit() {
    // B-BUDGET-006: check_policy returns PolicyExceeded when actual > limit
    // Uses BoundednessPolicy::DEFAULT which has absolute_max_steps_executable = 1_000_000
    let usage = AggregateResourceUsage {
        max_steps_executable: 2_000_000, // exceeds DEFAULT limit of 1_000_000
        ..Default::default()
    };
    let policy = BoundednessPolicy::DEFAULT;
    let result = usage.check_policy(&policy);
    assert!(
        result.is_err(),
        "check_policy must reject policy-exceeding usage"
    );
    match result {
        Err(AggregateBudgetError::PolicyExceeded {
            resource,
            actual,
            limit,
        }) => {
            assert_eq!(resource, "max_steps_executable");
            assert_eq!(actual, 2_000_000);
            assert_eq!(limit, 1_000_000);
        }
        other => panic!("expected PolicyExceeded, got {:?}", other),
    }
}
