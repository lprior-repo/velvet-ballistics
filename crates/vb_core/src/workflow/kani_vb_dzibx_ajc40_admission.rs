#![cfg(all(kani, feature = "kani-vb-ajc40"))]
#![forbid(unsafe_code)]
//! RPO-AJC40-004: direct Kani properties for the production admission kernel.
//! Inputs are symbolic scalar values; no production structures are hardcoded and
//! no stubs or contracts are used.

use crate::workflow::admission_kernel::{
    AdmissionKernelError, accumulate_yield_cost, validate_admission_summary,
};

#[kani::proof]
#[kani::unwind(4)]
fn vb_dzibx_ajc40_admission_kernel_boundaries() {
    let count: usize = kani::any();
    let max_count: usize = kani::any();
    let max_path_depth: usize = kani::any();
    let max_path_segments: usize = kani::any();
    let recomputed_total: u64 = kani::any();
    let declared_total_yield_cost: u64 = kani::any();
    let max_yield_budget: u64 = kani::any();

    let actual = validate_admission_summary(
        count,
        max_count,
        max_path_depth,
        max_path_segments,
        recomputed_total,
        declared_total_yield_cost,
        max_yield_budget,
    );

    kani::cover!(count > max_count, "count rejection branch reachable");
    kani::cover!(
        count <= max_count && max_path_depth > max_path_segments,
        "path-depth rejection branch reachable"
    );
    kani::cover!(
        count <= max_count
            && max_path_depth <= max_path_segments
            && declared_total_yield_cost != recomputed_total,
        "total mismatch branch reachable"
    );
    kani::cover!(
        count <= max_count
            && max_path_depth <= max_path_segments
            && declared_total_yield_cost == recomputed_total
            && recomputed_total > max_yield_budget,
        "budget rejection branch reachable"
    );
    kani::cover!(
        count <= max_count
            && max_path_depth <= max_path_segments
            && declared_total_yield_cost == recomputed_total
            && recomputed_total <= max_yield_budget,
        "success branch reachable"
    );

    if count > max_count {
        assert_eq!(actual, Err(AdmissionKernelError::TooManyItems));
    } else if max_path_depth > max_path_segments {
        assert_eq!(actual, Err(AdmissionKernelError::PathTooDeep));
    } else if declared_total_yield_cost != recomputed_total {
        assert_eq!(actual, Err(AdmissionKernelError::TotalYieldCostMismatch));
    } else {
        match max_yield_budget.checked_sub(recomputed_total) {
            Some(remaining) => assert_eq!(actual, Ok(remaining)),
            None => assert_eq!(actual, Err(AdmissionKernelError::YieldBudgetExceeded)),
        }
    }

    let accumulated_total: u64 = kani::any();
    let item_cost: u64 = kani::any();
    let accumulated = accumulate_yield_cost(accumulated_total, item_cost);
    let checked = accumulated_total.checked_add(item_cost);

    kani::cover!(checked.is_some(), "checked-add success branch reachable");
    kani::cover!(checked.is_none(), "checked-add overflow branch reachable");

    match checked {
        Some(sum) => assert_eq!(accumulated, Ok(sum)),
        None => assert_eq!(accumulated, Err(AdmissionKernelError::YieldBudgetExceeded)),
    }
}
