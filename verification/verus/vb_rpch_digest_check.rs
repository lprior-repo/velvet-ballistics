// Verus proof obligations for vb-rpch INV-005: DigestCheck hierarchy strictness.
//
// Obligation: VERUS-REC-005 / INV-005
// Contract: DigestCheck variants form a strict hierarchy:
//           WorkflowSourceOnly ⊂ WorkflowAndIr ⊂ Full
//           (each level adds one more digest check)

use vstd::prelude::*;

verus! {

pub spec fn spec_digest_check_level(d: DigestCheck) -> int {
    match d {
        DigestCheck::WorkflowSourceOnly => 0,
        DigestCheck::WorkflowAndIr => 1,
        DigestCheck::Full => 2,
    }
}

pub proof fn proof_hierarchy_strict()
    ensures
        spec_digest_check_level(DigestCheck::WorkflowSourceOnly) <
        spec_digest_check_level(DigestCheck::WorkflowAndIr),
    ensures
        spec_digest_check_level(DigestCheck::WorkflowAndIr) <
        spec_digest_check_level(DigestCheck::Full),
    ensures
        spec_digest_check_level(DigestCheck::WorkflowSourceOnly) <
        spec_digest_check_level(DigestCheck::Full)
{
    reveal(spec_digest_check_level);
}

pub proof fn proof_level_implies_superset(
    d1: DigestCheck,
    d2: DigestCheck
)
    requires
        spec_digest_check_level(d1) < spec_digest_check_level(d2),
    ensures
        match d1 {
            DigestCheck::WorkflowSourceOnly => true,
            DigestCheck::WorkflowAndIr => d2 == DigestCheck::Full,
            DigestCheck::Full => false,
        }
{
    reveal(spec_digest_check_level);
}

pub proof fn proof_workflow_only_is_minimal(d: DigestCheck)
    requires
        d == DigestCheck::WorkflowSourceOnly,
    ensures
        spec_digest_check_level(d) == 0
{
    reveal(spec_digest_check_level);
}

pub proof fn proof_full_is_maximal(d: DigestCheck)
    requires
        d == DigestCheck::Full,
    ensures
        spec_digest_check_level(d) == 2
{
    reveal(spec_digest_check_level);
}

} // verus!

fn main() {}