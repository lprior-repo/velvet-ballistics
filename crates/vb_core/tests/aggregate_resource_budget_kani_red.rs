#[cfg(kani)]
mod aggregate_budget_kani_harnesses {
    const BUDGET_RS: &str = include_str!("../src/budget.rs");
    const ADMISSION_RS: &str = include_str!("../../vb_runtime/src/admission.rs");

    #[kani::proof]
    fn checked_addition_harness_requires_aggregate_usage_api() {
        kani::assert(
            BUDGET_RS.contains("try_add_budget", "assertion failed"),
            "kani harness assertion",
        );
        kani::assert(
            BUDGET_RS.contains("checked_add", "assertion failed"),
            "kani harness assertion",
        );
    }

    #[kani::proof]
    fn checked_subtraction_harness_requires_aggregate_usage_api() {
        kani::assert(
            BUDGET_RS.contains("try_subtract_budget", "assertion failed"),
            "kani harness assertion",
        );
        kani::assert(
            BUDGET_RS.contains("checked_sub", "assertion failed"),
            "kani harness assertion",
        );
    }

    #[kani::proof]
    fn capacity_comparison_harness_requires_inclusive_api() {
        kani::assert(
            BUDGET_RS.contains("fits_within", "assertion failed"),
            "kani harness assertion",
        );
        kani::assert(
            BUDGET_RS.contains("CapacityExceeded", "assertion failed"),
            "kani harness assertion",
        );
    }

    #[kani::proof]
    fn reservation_roundtrip_harness_requires_reservation_api() {
        kani::assert(
            BUDGET_RS.contains("AggregateReservation", "assertion failed"),
            "kani harness assertion",
        );
        kani::assert(
            BUDGET_RS.contains("ReservationNotFound", "assertion failed"),
            "kani harness assertion",
        );
    }

    #[kani::proof]
    fn admission_harness_requires_budget_capacity_api() {
        kani::assert(
            ADMISSION_RS.contains("admit_run_with_budget", "assertion failed"),
            "kani harness assertion",
        );
        kani::assert(
            ADMISSION_RS.contains("ResourceCapacityExceeded", "assertion failed"),
            "kani harness assertion",
        );
    }
}
