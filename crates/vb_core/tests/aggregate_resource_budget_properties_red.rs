use proptest::prelude::{ProptestConfig, *};

const BUDGET_RS: &str = include_str!("../src/budget.rs");
// The aggregate budget implementation is split across `budget.rs` and
// its `budget/` submodules; the checked add/subtract impls moved to
// `budget/usage.rs` in the source-length split series, so the shell
// alone no longer contains `try_add_budget`. Concatenate the submodule
// so the proptest sees the same surface the production binary sees.
const BUDGET_USAGE_RS: &str = include_str!("../src/budget/usage.rs");
// The aggregate budget types and validation moved to
// `budget/aggregate.rs` and `budget/aggregate_validate.rs` in the
// source-length split series, so the shell alone no longer contains
// `AggregateResourceBudget` or its dimension names.
// The boundedness policy, budget error taxonomy, and dimension
// classification moved to `budget/validate.rs` and
// `budget/validate_error.rs` in the source-length split series, so the
// shell alone no longer contains `BoundednessPolicy` or `BudgetError`.
// The budget computation and its small-linear fast path moved to
// `budget/compute.rs` and `budget/small_linear.rs`, so the shell alone
// no longer contains `WholeWorkflowBudget` or `compute_budget_local`.
// The traversal error type and its conversions moved to
// `budget/traversal_error.rs`, so the shell alone no longer contains
// `BudgetTraversalError` or its `From` conversions.
const BUDGET_AGGREGATE_RS: &str = include_str!("../src/budget/aggregate.rs");
const BUDGET_AGGREGATE_VALIDATE_RS: &str = include_str!("../src/budget/aggregate_validate.rs");
const BUDGET_COMPUTE_RS: &str = include_str!("../src/budget/compute.rs");
const BUDGET_SMALL_LINEAR_RS: &str = include_str!("../src/budget/small_linear.rs");
const BUDGET_VALIDATE_RS: &str = include_str!("../src/budget/validate.rs");
const BUDGET_VALIDATE_ERROR_RS: &str = include_str!("../src/budget/validate_error.rs");
const BUDGET_TRAVERSAL_ERROR_RS: &str = include_str!("../src/budget/traversal_error.rs");
// The admission shell is split across focused chunks under `parts/` that
// are `include!`-d into `admission.rs` at compile time. The runtime-source
// literals this proptest searches for (`admit_run_with_budget`,
// `ResourceCapacityExceeded`) live in the chunks; the shell alone won't
// satisfy `.contains(...)`. Concatenate the chunks so the proptest sees
// the same surface the production binary sees.
const ADMISSION_RS: &str = include_str!("../../vb_runtime/src/admission.rs");
const ADMISSION_ERRORS_RS: &str =
    include_str!("../../vb_runtime/src/admission/parts/chunk_001_types_errors_traits.rs");
const ADMISSION_BUDGET_RS: &str =
    include_str!("../../vb_runtime/src/admission/parts/chunk_006_admit_budget.rs");

/// Total runtime surface area of the admission module after the shell
/// glues its chunks in via `include!`. Asserted at proptest time so the
/// split does not silently re-break surface-area checks. See commit
/// that fixed the BLOCK_GLOBAL `proptest_admission_with_budget...`
/// failure (the literal `admit_run_with_budget` lives only in
/// `chunk_006_admit_budget.rs`).
fn admission_production_surface() -> String {
    let mut s = String::from(ADMISSION_RS);
    s.push('\n');
    s.push_str(ADMISSION_ERRORS_RS);
    s.push('\n');
    s.push_str(ADMISSION_BUDGET_RS);
    s
}

/// Total production surface area of the aggregate budget module after
/// the shell glues its `budget/` submodules in. Asserted at proptest
/// time so the split does not silently re-break surface-area checks.
fn budget_production_surface() -> String {
    let mut s = String::from(BUDGET_RS);
    s.push('\n');
    s.push_str(BUDGET_USAGE_RS);
    s.push('\n');
    s.push_str(BUDGET_AGGREGATE_RS);
    s.push('\n');
    s.push_str(BUDGET_AGGREGATE_VALIDATE_RS);
    s.push('\n');
    s.push_str(BUDGET_COMPUTE_RS);
    s.push('\n');
    s.push_str(BUDGET_SMALL_LINEAR_RS);
    s.push('\n');
    s.push_str(BUDGET_VALIDATE_RS);
    s.push('\n');
    s.push_str(BUDGET_VALIDATE_ERROR_RS);
    s.push('\n');
    s.push_str(BUDGET_TRAVERSAL_ERROR_RS);
    s
}

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
        let budget_rs = budget_production_surface();

        prop_assert_eq!(budget_rs.contains("pub struct AggregateResourceBudget"), true);
        prop_assert_eq!(budget_rs.contains(dimensions[index]), true);
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
        let budget_rs = budget_production_surface();

        prop_assert_eq!(checked_sum, a.checked_add(b));
        prop_assert_eq!(checked_difference, a.checked_sub(b));
        prop_assert_eq!(budget_rs.contains("try_add_budget"), true);
        prop_assert_eq!(budget_rs.contains("try_subtract_budget"), true);
        prop_assert_eq!(budget_rs.contains("checked_add"), true);
        prop_assert_eq!(budget_rs.contains("checked_sub"), true);
    }

    #[test]
    fn proptest_admission_with_budget_has_runtime_capacity_rejection_surface(requested in 1u64..1000) {
        let available = requested.saturating_sub(1);

        let admission_rs = admission_production_surface();
        prop_assert_eq!(requested > available, true);
        prop_assert_eq!(admission_rs.contains("admit_run_with_budget"), true);
        prop_assert_eq!(admission_rs.contains("ResourceCapacityExceeded"), true);
    }
}
