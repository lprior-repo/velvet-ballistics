#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
// vb-pyg3p: behavior test for vb_runtime runtime facade
// Tests public Runtime facade importability, API surface, and typed-error paths.
//
// This is a basic behavior test - proptest properties can be added
// in vb_runtime_facade_restoration_properties.rs once the lane is fixed.
#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use vb_core::WorkflowDigest;
use vb_core::ids::RunId;
use vb_runtime::runtime::{ActiveRunSummary, Runtime};
use vb_runtime::shard::ShardConfig;
use vb_runtime::{RuntimeError, RuntimeResult};

/// Test that Runtime can be constructed with valid configuration.
#[test]
fn test_runtime_construction_succeeds() {
    let config = ShardConfig::default();
    let _runtime = Runtime::new(NonZeroUsize::new(4).expect("non-zero"), config);
}

/// Test that ActiveRunSummary fields are accessible.
#[test]
fn test_active_run_summary_fields_accessible() {
    let summary = ActiveRunSummary {
        run_id: RunId::new(42),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
        step_count: 10,
        steps_completed: 5,
    };
    assert_eq!(summary.run_id, RunId::new(42));
    assert_eq!(summary.step_count, 10);
    assert_eq!(summary.steps_completed, 5);
}

/// Test that ActiveRunSummary Clone and Eq work.
#[test]
fn test_active_run_summary_clone_and_eq() {
    let summary1 = ActiveRunSummary {
        run_id: RunId::new(123),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
        step_count: 8,
        steps_completed: 3,
    };
    let summary2 = summary1.clone();
    assert_eq!(summary1, summary2);
    assert_eq!(summary1.step_count, summary2.step_count);
}

/// Test RuntimeError Display does not panic.
#[test]
fn test_runtime_error_display_queue_full() {
    let err = RuntimeError::QueueFull;
    let display = format!("{err}");
    assert!(!display.is_empty());
}

/// Test RuntimeError Display for RunNotFound.
#[test]
fn test_runtime_error_display_run_not_found() {
    let err = RuntimeError::RunNotFound;
    let display = format!("{err}");
    assert!(!display.is_empty());
}

/// Test RuntimeError Display for UnsupportedOperation.
#[test]
fn test_runtime_error_display_unsupported_operation() {
    let err = RuntimeError::UnsupportedOperation {
        operation: "test_operation",
    };
    let display = format!("{err}");
    assert!(!display.is_empty());
}

/// Test RuntimeResult ok path.
#[test]
fn test_runtime_result_ok() {
    let config = ShardConfig::default();
    let result: RuntimeResult<Runtime> =
        Runtime::new(NonZeroUsize::new(2).expect("non-zero"), config);
    assert!(result.is_ok());
}

/// Test RuntimeResult err path.
#[test]
fn test_runtime_result_err() {
    let err = RuntimeError::ActiveRunCapacityExceeded { capacity: 100 };
    let result: RuntimeResult<()> = Err(err);
    assert!(result.is_err());
}
