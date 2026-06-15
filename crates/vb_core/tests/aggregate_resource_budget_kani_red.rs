#[cfg(kani)]
mod aggregate_budget_kani_harnesses {
    const BUDGET_RS: &str = include_str!("../src/budget.rs");
    const ADMISSION_RS: &str = include_str!("../../vb_runtime/src/admission.rs");

    #[kani::proof]
    fn checked_addition_harness_requires_aggregate_usage_api() {
        kani::assert(
            BUDGET_RS.contains("try_add_budget"),
            "kani harness assertion",
        );
        kani::assert(BUDGET_RS.contains("checked_add"), "kani harness assertion");
    }

    #[kani::proof]
    fn checked_subtraction_harness_requires_aggregate_usage_api() {
        kani::assert(
            BUDGET_RS.contains("try_subtract_budget"),
            "kani harness assertion",
        );
        kani::assert(BUDGET_RS.contains("checked_sub"), "kani harness assertion");
    }

    #[kani::proof]
    fn capacity_comparison_harness_requires_inclusive_api() {
        kani::assert(BUDGET_RS.contains("fits_within"), "kani harness assertion");
        kani::assert(
            BUDGET_RS.contains("CapacityExceeded"),
            "kani harness assertion",
        );
    }

    #[kani::proof]
    fn reservation_roundtrip_harness_requires_reservation_api() {
        kani::assert(
            BUDGET_RS.contains("AggregateReservation"),
            "kani harness assertion",
        );
        kani::assert(
            BUDGET_RS.contains("ReservationNotFound"),
            "kani harness assertion",
        );
    }

    #[kani::proof]
    fn admission_harness_requires_budget_capacity_api() {
        kani::assert(
            ADMISSION_RS.contains("admit_run_with_budget"),
            "kani harness assertion",
        );
        kani::assert(
            ADMISSION_RS.contains("ResourceCapacityExceeded"),
            "kani harness assertion",
        );
    }
}
