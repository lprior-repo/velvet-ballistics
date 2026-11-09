#[cfg(kani)]
mod aggregate_budget_kani_harnesses {
    // `budget.rs` is a shell over sibling modules in `budget/`; the checked
    // add/subtract impls live in `budget/usage_arith.rs`, the capacity fit
    // and boundedness-policy check live in `budget/usage.rs`, the aggregate types
    // and validation live in `budget/aggregate.rs` and
    // `budget/aggregate_validate.rs`, the budget computation and its
    // small-linear fast path live in `budget/compute.rs` and
    // `budget/small_linear.rs`, and the boundedness policy and error
    // taxonomy live in `budget/validate.rs` and `budget/validate_error.rs`,
    // and the traversal error type and its conversions live in
    // `budget/traversal_error.rs`, so the shell alone would miss the
    // tokens these harnesses require.
    // Concatenate them, mirroring `aggregate_resource_budget_red.rs`.
    const BUDGET_RS: &str = concat!(
        include_str!("../src/budget.rs"),
        "\n",
        include_str!("../src/budget/usage.rs"),
        "\n",
        include_str!("../src/budget/usage_arith.rs"),
        "\n",
        include_str!("../src/budget/aggregate.rs"),
        "\n",
        include_str!("../src/budget/aggregate_validate.rs"),
        "\n",
        include_str!("../src/budget/compute.rs"),
        "\n",
        include_str!("../src/budget/small_linear.rs"),
        "\n",
        include_str!("../src/budget/validate.rs"),
        "\n",
        include_str!("../src/budget/validate_error.rs"),
        "\n",
        include_str!("../src/budget/traversal_error.rs"),
    );
    const ADMISSION_RS: &str = include_str!("../../vb_runtime/src/admission.rs");
    const ADMISSION_ERRORS_RS: &str =
        include_str!("../../vb_runtime/src/admission/parts/chunk_001_types_errors_traits.rs");
    const ADMISSION_BUDGET_RS: &str =
        include_str!("../../vb_runtime/src/admission/parts/chunk_006_admit_budget.rs");

    #[kani::proof]
    fn checked_addition_harness_requires_aggregate_usage_api() {
        assert!(BUDGET_RS.contains("try_add_budget"));
        assert!(BUDGET_RS.contains("checked_add"));
    }

    #[kani::proof]
    fn checked_subtraction_harness_requires_aggregate_usage_api() {
        assert!(BUDGET_RS.contains("try_subtract_budget"));
        assert!(BUDGET_RS.contains("checked_sub"));
    }

    #[kani::proof]
    fn capacity_comparison_harness_requires_inclusive_api() {
        assert!(BUDGET_RS.contains("fits_within"));
        assert!(BUDGET_RS.contains("CapacityExceeded"));
    }

    #[kani::proof]
    fn reservation_roundtrip_harness_requires_reservation_api() {
        assert!(BUDGET_RS.contains("AggregateReservation"));
        assert!(BUDGET_RS.contains("ReservationNotFound"));
    }

    #[kani::proof]
    fn admission_harness_requires_budget_capacity_api() {
        assert!(
            ADMISSION_RS.contains("admit_run_with_budget")
                || ADMISSION_BUDGET_RS.contains("admit_run_with_budget")
        );
        assert!(
            ADMISSION_RS.contains("ResourceCapacityExceeded")
                || ADMISSION_ERRORS_RS.contains("ResourceCapacityExceeded")
        );
    }
}
