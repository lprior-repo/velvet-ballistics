//! vb-ajc40 PO-039: generated admission kernel proves depth zero is not above any usize limit.
//!
//! Mirrors the kernel's `proof_empty_root_path_not_too_deep` lemma:
//! for any `max_path_segments: usize`, the empty-path depth 0 is always
//! within bounds because `usize` is non-negative.

use vstd::prelude::*;

verus! {

/// Proof that empty-path (depth 0) always satisfies the path-depth bound
/// for any valid `max_path_segments`.  This mirrors the generated kernel's
/// `proof_empty_root_path_not_too_deep` and documents the spec-level property
/// that the post-decode exec function `po_039_empty_path_post_decode` relies on.
pub proof fn po_039_scalar_empty_path_delegates_to_generated_kernel(max_path_segments: usize)
    ensures 0usize <= max_path_segments,
{
    assert(0usize <= max_path_segments);
}

}
