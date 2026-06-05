include!("vb_ajc40_admission_kernel_scalar_include.rs");

// vb-ajc40 PO-039. Empty path/root accessor post-decode proof over the mechanically shared scalar kernel.

verus! {

pub fn po_039_empty_path_post_decode() -> (result: Result<u64, AdmissionKernelError>)
    ensures result_is_ok_u64(result) && result_value_u64(result) == 0,
{
    validate_admission_summary(1usize, 65535usize, 0usize, 16usize, 0u64, 0u64, 0u64)
}

} // verus!
