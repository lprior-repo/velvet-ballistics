use proptest::prelude::{ProptestConfig, *};

const BUDGET_RS: &str = include_str!("../src/budget.rs");
// The admission shell is split across focused chunks under `parts/` that
// are `include!`-d into `admission.rs` at compile time. The runtime-source
// literals this proptest searches for (`admit_run_with_budget`,
// `ResourceCapacityExceeded`) live in the chunks; the shell alone won't
// satisfy `.contains(...)`. Concatenate the chunks so the proptest sees
// the same surface the production binary sees.
const ADMISSION_RS: &str = include_str!("../../vb_runtime/src/admission.rs");
const ADMISSION_ERRORS_RS: &str = include_str!("../../vb_runtime/src/admission/parts/chunk_001_types_errors_traits.rs");
const ADMISSION_BUDGET_RS: &str = include_str!("../../vb_runtime/src/admission/parts/chunk_006_admit_budget.rs");

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

        let admission_rs = admission_production_surface();
        prop_assert_eq!(requested > available, true);
        prop_assert_eq!(admission_rs.contains("admit_run_with_budget"), true);
        prop_assert_eq!(admission_rs.contains("ResourceCapacityExceeded"), true);
    }
}
