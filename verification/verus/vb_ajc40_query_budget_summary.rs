include!("vb_ajc40_admission_kernel_scalar_include.rs");

// vb-ajc40 PO-015. Query budget-summary proof over the mechanically shared scalar kernel.

verus! {

pub fn po_015_query_budget_summary(
    recomputed_total: u64,
    declared_total_yield_cost: u64,
    max_yield_budget: u64,
) -> (result: Result<u64, AdmissionKernelError>)
    ensures
        declared_total_yield_cost == recomputed_total && recomputed_total <= max_yield_budget
            ==> result_is_ok_u64(result)
                && result_value_u64(result) == max_yield_budget as int - recomputed_total as int,
        declared_total_yield_cost != recomputed_total ==> !result_is_ok_u64(result)
            && result_error_u64(result) == AdmissionKernelError::TotalYieldCostMismatch,
        declared_total_yield_cost == recomputed_total && recomputed_total > max_yield_budget
            ==> !result_is_ok_u64(result)
                && result_error_u64(result) == AdmissionKernelError::YieldBudgetExceeded,
{
    validate_admission_summary(1usize, 65535usize, 0usize, 16usize, recomputed_total, declared_total_yield_cost, max_yield_budget)
}

} // verus!
