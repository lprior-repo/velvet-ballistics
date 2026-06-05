// Shared include for vb-ajc40 Verus post-decode obligation files.
// Mechanically mirrors `vb_ajc40_admission_kernel_scalar.rs`, generated from
// `vb_ajc40_admission_kernel.source` sha256
// e8cd350a9a0ffb712c163e5e3f327d69d148f3d361ade443546d9964efdeea8d.
// Production mirror: `crates/vb_core/src/workflow/admission_kernel.rs`.

use vstd::prelude::*;

verus! {

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionKernelError {
    TooManyItems,
    PathTooDeep,
    TotalYieldCostMismatch,
    YieldBudgetExceeded,
}

pub open spec fn result_is_ok_u64(result: Result<u64, AdmissionKernelError>) -> bool {
    matches!(result, Ok(_))
}

pub open spec fn result_value_u64(result: Result<u64, AdmissionKernelError>) -> int
    recommends result_is_ok_u64(result)
{
    match result {
        Ok(value) => value as int,
        Err(_) => 0,
    }
}

pub open spec fn result_error_u64(result: Result<u64, AdmissionKernelError>) -> AdmissionKernelError
    recommends !result_is_ok_u64(result)
{
    match result {
        Ok(_) => AdmissionKernelError::YieldBudgetExceeded,
        Err(error) => error,
    }
}

pub fn validate_admission_summary(
    count: usize,
    max_count: usize,
    max_path_depth: usize,
    max_path_segments: usize,
    recomputed_total: u64,
    declared_total_yield_cost: u64,
    max_yield_budget: u64,
) -> (result: Result<u64, AdmissionKernelError>)
    ensures
        count > max_count ==> !result_is_ok_u64(result)
            && result_error_u64(result) == AdmissionKernelError::TooManyItems,
        count <= max_count && max_path_depth > max_path_segments ==> !result_is_ok_u64(result)
            && result_error_u64(result) == AdmissionKernelError::PathTooDeep,
        count <= max_count && max_path_depth <= max_path_segments
            && declared_total_yield_cost != recomputed_total ==> !result_is_ok_u64(result)
            && result_error_u64(result) == AdmissionKernelError::TotalYieldCostMismatch,
        count <= max_count && max_path_depth <= max_path_segments
            && declared_total_yield_cost == recomputed_total
            && recomputed_total > max_yield_budget ==> !result_is_ok_u64(result)
            && result_error_u64(result) == AdmissionKernelError::YieldBudgetExceeded,
        count <= max_count && max_path_depth <= max_path_segments
            && declared_total_yield_cost == recomputed_total
            && recomputed_total <= max_yield_budget ==> result_is_ok_u64(result)
            && result_value_u64(result) == max_yield_budget as int - recomputed_total as int,
{
    if count > max_count {
        return Err(AdmissionKernelError::TooManyItems);
    }
    if max_path_depth > max_path_segments {
        return Err(AdmissionKernelError::PathTooDeep);
    }
    if declared_total_yield_cost != recomputed_total {
        return Err(AdmissionKernelError::TotalYieldCostMismatch);
    }
    if recomputed_total > max_yield_budget {
        return Err(AdmissionKernelError::YieldBudgetExceeded);
    }
    match max_yield_budget.checked_sub(recomputed_total) {
        Some(remaining) => Ok(remaining),
        None => Err(AdmissionKernelError::YieldBudgetExceeded),
    }
}

} // verus!
