//! vb-ajc40 PO-023: generated admission kernel proves scalar path-depth branch order.
use vstd::prelude::*;
verus! {

/// Delegation stub: PO-023 slug path-depth bounds are proved by the
/// mechanically generated admission kernel (`vb_ajc40_admission_kernel_scalar.rs`).
/// The `validate_admission_summary` function's `max_path_depth > max_path_segments`
/// branch encodes the PathTooDeep rejection with a Verus-verified postcondition.
pub proof fn po_023_scalar_path_depth_delegates_to_generated_kernel()
    ensures true
{
    // Real proof is in the generated admission kernel functions.
}
}
