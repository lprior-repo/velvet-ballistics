// Verus verifier-only model for vb-core-cli-accepted-path PO-002.
// Trusted shell boundary: hash computation, Fjall I/O, and postcard decode.

use vstd::prelude::*;

verus! {

pub open spec fn digest_binding_total(
    source_digest: int,
    artifact_digest: int,
    header_digest: int,
    event_digest: int,
    admission_digest: int,
) -> bool {
    source_digest == artifact_digest
        && artifact_digest == header_digest
        && header_digest == event_digest
        && event_digest == admission_digest
}

pub open spec fn digest_mismatch_rejects(
    source_digest: int,
    artifact_digest: int,
    header_digest: int,
    event_digest: int,
    admission_digest: int,
) -> bool {
    !digest_binding_total(source_digest, artifact_digest, header_digest, event_digest, admission_digest)
}

pub proof fn proof_total_binding_implies_all_equal(
    source_digest: int,
    artifact_digest: int,
    header_digest: int,
    event_digest: int,
    admission_digest: int,
)
    requires
        digest_binding_total(source_digest, artifact_digest, header_digest, event_digest, admission_digest),
    ensures
        source_digest == artifact_digest,
        source_digest == header_digest,
        source_digest == event_digest,
        source_digest == admission_digest,
{
}

pub proof fn proof_any_pairwise_mismatch_rejects(
    source_digest: int,
    artifact_digest: int,
    header_digest: int,
    event_digest: int,
    admission_digest: int,
)
    requires
        source_digest != artifact_digest
            || artifact_digest != header_digest
            || header_digest != event_digest
            || event_digest != admission_digest,
    ensures
        digest_mismatch_rejects(source_digest, artifact_digest, header_digest, event_digest, admission_digest),
{
}

pub proof fn proof_admitted_digest_matches_run_header(
    source_digest: int,
    artifact_digest: int,
    header_digest: int,
    event_digest: int,
    admission_digest: int,
)
    requires
        digest_binding_total(source_digest, artifact_digest, header_digest, event_digest, admission_digest),
    ensures
        admission_digest == header_digest,
        event_digest == header_digest,
{
}

} // verus!

fn main() {}
