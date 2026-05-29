// PO-VB-DYBJ-004
// Verus artifact for WorkflowDigest exact 32-byte preservation.
// Production binding: mirrors `vb_core::ids::WorkflowDigest` constructor and
// accessor at `crates/vb_core/src/ids/mod.rs`, where the public API accepts
// exactly `[u8; 32]` and returns exactly `[u8; 32]`.

use vstd::prelude::*;

verus! {

pub struct WorkflowDigestModel {
    pub bytes: Seq<u8>,
}

pub open spec fn digest_shape(bytes: Seq<u8>) -> bool {
    bytes.len() == 32
}

pub open spec fn workflow_digest_from_bytes(bytes: Seq<u8>) -> WorkflowDigestModel
    recommends
        digest_shape(bytes),
{
    WorkflowDigestModel { bytes }
}

pub open spec fn workflow_digest_as_bytes(digest: WorkflowDigestModel) -> Seq<u8> {
    digest.bytes
}

pub proof fn proof_workflow_digest_preserves_all_32_bytes(bytes: Seq<u8>)
    requires
        digest_shape(bytes),
    ensures
        workflow_digest_as_bytes(workflow_digest_from_bytes(bytes)) == bytes,
        workflow_digest_as_bytes(workflow_digest_from_bytes(bytes)).len() == 32,
{
}

pub proof fn proof_variable_length_not_accepted(bytes: Seq<u8>)
    requires
        bytes.len() != 32,
    ensures
        !digest_shape(bytes),
{
}

} // verus!

fn main() {}
