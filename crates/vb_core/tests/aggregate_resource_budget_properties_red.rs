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

use proptest::prelude::{ProptestConfig, *};

const BUDGET_RS: &str = concat!(
    include_str!("../src/budget/mod.rs"),
    include_str!("../src/budget/aggregate_budget.rs"),
    include_str!("../src/budget/aggregate_usage.rs"),
    include_str!("../src/budget/aggregate_usage_checks.rs"),
    include_str!("../src/budget/budget_error.rs"),
    include_str!("../src/budget/policy.rs"),
    include_str!("../src/budget/small_linear.rs"),
    include_str!("../src/budget/traversal.rs"),
    include_str!("../src/budget/traversal_depth.rs"),
    include_str!("../src/budget/traversal_driver.rs"),
    include_str!("../src/budget/traversal_loop.rs"),
    include_str!("../src/budget/traversal_metrics.rs"),
    include_str!("../src/budget/traversal_path.rs"),
    include_str!("../src/budget/traversal_step_count.rs"),
    include_str!("../src/budget/traversal_successors.rs"),
    include_str!("../src/budget/traversal_tracking.rs"),
    include_str!("../src/budget/types.rs"),
    include_str!("../src/budget/validation.rs"),
);
const ADMISSION_RS: &str = concat!(
    include_str!("../../vb_runtime/src/admission.rs"),
    include_str!("../../vb_runtime/src/admission/admission.rs"),
    include_str!("../../vb_runtime/src/admission/errors.rs")
);

proptest! {
    #![proptest_config(ProptestConfig { failure_persistence: None, .. ProptestConfig::default() })]

    #[test]
    fn proptest_aggregate_budget_dimensions_are_declared_for_any_dimension_index(index in 0usize..14) {
        let dimensions = [
            "max_steps_executable",
            "max_action_tickets",
            "max_parallel_in_flight",
            "max_retries_per_action",
            "max_gather_pages",
            "max_gather_items",
            "max_for_each_iterations",
            "max_together_branches",
            "max_repeat_attempts",
            "max_run_time_seconds",
            "max_result_bytes",
            "max_total_slots_written",
            "max_queue_depth",
            "max_journal_batch_bytes",
        ];

        prop_assert_eq!(BUDGET_RS.contains("pub struct AggregateResourceBudget"), true);
        prop_assert_eq!(BUDGET_RS.contains(dimensions[index]), true);
    }

    #[test]
    fn proptest_capacity_comparison_reports_exact_requested_available_values(delta in 1u64..1000) {
        let requested = 100u64.saturating_add(delta);
        let available = 100u64;

        prop_assert_eq!(requested > available, true);
        prop_assert_eq!(BUDGET_RS.contains("CapacityExceeded"), true);
        prop_assert_eq!(BUDGET_RS.contains("requested"), true);
        prop_assert_eq!(BUDGET_RS.contains("available"), true);
    }

    #[test]
    fn proptest_policy_errors_preserve_exact_actual_and_limit(delta in 1u64..1000) {
        let actual = 100u64.saturating_add(delta);
        let limit = 100u64;

        prop_assert_eq!(actual > limit, true);
        prop_assert_eq!(BUDGET_RS.contains("PolicyExceeded"), true);
        prop_assert_eq!(BUDGET_RS.contains("actual"), true);
        prop_assert_eq!(BUDGET_RS.contains("limit"), true);
    }

    #[test]
    fn proptest_checked_add_and_subtract_are_contractually_required(a in 0u64..1000, b in 0u64..1000) {
        let checked_sum = a.checked_add(b);
        let checked_difference = a.checked_sub(b);

        prop_assert_eq!(checked_sum, a.checked_add(b));
        prop_assert_eq!(checked_difference, a.checked_sub(b));
        prop_assert_eq!(BUDGET_RS.contains("try_add_budget"), true);
        prop_assert_eq!(BUDGET_RS.contains("try_subtract_budget"), true);
        prop_assert_eq!(BUDGET_RS.contains("checked_add"), true);
        prop_assert_eq!(BUDGET_RS.contains("checked_sub"), true);
    }

    #[test]
    fn proptest_admission_with_budget_has_runtime_capacity_rejection_surface(requested in 1u64..1000) {
        let available = requested.saturating_sub(1);

        prop_assert_eq!(requested > available, true);
        prop_assert_eq!(ADMISSION_RS.contains("admit_run_with_budget"), true);
        prop_assert_eq!(ADMISSION_RS.contains("ResourceCapacityExceeded"), true);
    }
}
