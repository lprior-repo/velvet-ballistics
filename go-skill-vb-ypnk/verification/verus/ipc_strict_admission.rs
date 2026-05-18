// Obligations: VERUS-IPC-001. Production linkage remains REFINE-IPC-001.
// Pure strict-admission model for IPC SubmitRun accepted-artifact evidence.
// Assumptions: storage and journal I/O are shell boundaries; this file proves
// only the pure witness predicates used by the runtime admission gate.

use vstd::prelude::*;

verus! {

pub open spec fn strict_admission_witness(
    has_required_evidence: bool,
    digest_matches: bool,
) -> bool {
    has_required_evidence && digest_matches
}

pub open spec fn reject_missing_evidence_witness(
    has_required_evidence: bool,
    digest_matches: bool,
) -> bool {
    !strict_admission_witness(has_required_evidence, digest_matches)
}

pub proof fn strict_admission_requires_required_gates(
    has_required_evidence: bool,
    digest_matches: bool,
)
    requires
        strict_admission_witness(has_required_evidence, digest_matches),
    ensures
        has_required_evidence,
        digest_matches,
{
    assert(strict_admission_witness(has_required_evidence, digest_matches));
}

pub proof fn reject_missing_evidence(digest_matches: bool)
    ensures
        reject_missing_evidence_witness(false, digest_matches),
{
    assert(!strict_admission_witness(false, digest_matches));
}

pub proof fn reject_digest_mismatch(has_required_evidence: bool)
    ensures
        reject_missing_evidence_witness(has_required_evidence, false),
{
    assert(!strict_admission_witness(has_required_evidence, false));
}

pub proof fn digest_agreement_preserved(has_required_evidence: bool)
    requires
        strict_admission_witness(has_required_evidence, true),
    ensures
        strict_admission_witness(has_required_evidence, true),
{
    assert(strict_admission_witness(has_required_evidence, true));
}

fn main() {}

} // verus!
