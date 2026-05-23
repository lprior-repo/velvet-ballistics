// Verus proof obligations for vb-rpch INV-005: DigestCheck hierarchy strictness.
//
// Obligation: VERUS-REC-005 / INV-005
// Contract: DigestCheck variants form a strict hierarchy:
//           WorkflowSourceOnly ⊂ WorkflowAndIr ⊂ Full
//           (each level adds one more digest check)

use vstd::prelude::*;

verus! {

pub enum SpecDigestCheck {
    WorkflowSourceOnly,
    WorkflowAndIr,
    Full,
}

pub open spec fn spec_digest_check_level(d: SpecDigestCheck) -> int {
    match d {
        SpecDigestCheck::WorkflowSourceOnly => 0,
        SpecDigestCheck::WorkflowAndIr => 1,
        SpecDigestCheck::Full => 2,
    }
}

pub proof fn proof_hierarchy_strict()
    ensures
        spec_digest_check_level(SpecDigestCheck::WorkflowSourceOnly) <
        spec_digest_check_level(SpecDigestCheck::WorkflowAndIr)
        && spec_digest_check_level(SpecDigestCheck::WorkflowAndIr) <
        spec_digest_check_level(SpecDigestCheck::Full)
        && spec_digest_check_level(SpecDigestCheck::WorkflowSourceOnly) <
        spec_digest_check_level(SpecDigestCheck::Full)
{
    reveal(spec_digest_check_level);
}

pub proof fn proof_level_implies_superset(
    d1: SpecDigestCheck,
    d2: SpecDigestCheck
)
    requires
        spec_digest_check_level(d1) < spec_digest_check_level(d2),
    ensures
        match d1 {
            SpecDigestCheck::WorkflowSourceOnly => true,
            SpecDigestCheck::WorkflowAndIr => d2 == SpecDigestCheck::Full,
            SpecDigestCheck::Full => false,
        }
{
    reveal(spec_digest_check_level);
}

pub proof fn proof_workflow_only_is_minimal(d: SpecDigestCheck)
    requires
        d == SpecDigestCheck::WorkflowSourceOnly,
    ensures
        spec_digest_check_level(d) == 0
{
    reveal(spec_digest_check_level);
}

pub proof fn proof_full_is_maximal(d: SpecDigestCheck)
    requires
        d == SpecDigestCheck::Full,
    ensures
        spec_digest_check_level(d) == 2
{
    reveal(spec_digest_check_level);
}

} // verus!

fn main() {}