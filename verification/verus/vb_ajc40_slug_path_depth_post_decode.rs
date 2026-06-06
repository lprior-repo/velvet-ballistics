include!("vb_ajc40_admission_kernel_scalar_include.rs");

// vb-ajc40 PO-023. Slug path-depth post-decode proof over the mechanically shared scalar kernel.

verus! {

pub fn po_023_slug_path_depth_post_decode(max_path_depth: usize) -> (result: Result<u64, AdmissionKernelError>)
    ensures
        max_path_depth <= 16usize ==> result_is_ok_u64(result) && result_value_u64(result) == 0,
        max_path_depth > 16usize ==> !result_is_ok_u64(result)
            && result_error_u64(result) == AdmissionKernelError::PathTooDeep,
{
    validate_admission_summary(1usize, 65535usize, max_path_depth, 16usize, 0u64, 0u64, 0u64)
}

} // verus!
