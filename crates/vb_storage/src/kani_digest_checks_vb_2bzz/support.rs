#![forbid(unsafe_code)]

use vb_core::WorkflowDigest;

/// Generate an arbitrary digest.
pub(crate) fn arbitrary_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes(kani::any())
}

/// Generate an arbitrary digest that is guaranteed to differ from `excluded`.
pub(crate) fn arbitrary_digest_except(excluded: WorkflowDigest) -> WorkflowDigest {
    let bytes: [u8; 32] = kani::any();
    kani::assume(bytes != excluded.as_bytes());
    WorkflowDigest::from_bytes(bytes)
}
