include!("vb_ajc40_admission_kernel_scalar_include.rs");

// vb-ajc40 PO-006. Post-decode query admission proof over the mechanically shared scalar kernel.
// This target intentionally does not prove postcard::from_bytes or serde.

verus! {

pub fn po_006_query_post_decode_summary(
    count: usize,
    max_path_depth: usize,
    recomputed_total: u64,
    declared_total_yield_cost: u64,
    max_yield_budget: u64,
) -> (result: Result<u64, AdmissionKernelError>)
    ensures
        count <= 65535usize && max_path_depth <= 16usize
            && declared_total_yield_cost == recomputed_total
            && recomputed_total <= max_yield_budget ==> result_is_ok_u64(result)
                && result_value_u64(result) == max_yield_budget as int - recomputed_total as int,
        count > 65535usize ==> !result_is_ok_u64(result)
            && result_error_u64(result) == AdmissionKernelError::TooManyItems,
        count <= 65535usize && max_path_depth > 16usize ==> !result_is_ok_u64(result)
            && result_error_u64(result) == AdmissionKernelError::PathTooDeep,
{
    validate_admission_summary(
        count,
        65535usize,
        max_path_depth,
        16usize,
        recomputed_total,
        declared_total_yield_cost,
        max_yield_budget,
    )
}

} // verus!
