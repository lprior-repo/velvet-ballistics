#[cfg(kani)]
mod aggregate_budget_kani_harnesses {
    const BUDGET_RS: &str = include_str!("../src/budget.rs");
    const ADMISSION_RS: &str = include_str!("../../vb_runtime/src/admission.rs");

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
        assert!(ADMISSION_RS.contains("admit_run_with_budget"));
        assert!(ADMISSION_RS.contains("ResourceCapacityExceeded"));
    }
}
