include!("vb_ajc40_admission_kernel_scalar_include.rs");

// vb-ajc40 PO-031. Slug count post-decode proof over the mechanically shared scalar kernel.

verus! {

pub fn po_031_slug_count_post_decode(count: usize) -> (result: Result<u64, AdmissionKernelError>)
    ensures
        count <= 65535usize ==> result_is_ok_u64(result) && result_value_u64(result) == 0,
        count > 65535usize ==> !result_is_ok_u64(result)
            && result_error_u64(result) == AdmissionKernelError::TooManyItems,
{
    validate_admission_summary(count, 65535usize, 0usize, 16usize, 0u64, 0u64, 0u64)
}

} // verus!
