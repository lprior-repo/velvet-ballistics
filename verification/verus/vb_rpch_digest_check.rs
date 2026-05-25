#![allow(unused_imports)]

use vstd::prelude::*;

verus! {

/// VFR-R2-VERUS-004 / INV-005.
/// Bridge model for crates/vb_storage/src/recovery/types.rs::DigestCheck
/// production methods hierarchy_rank, checks_workflow_source,
/// checks_compiled_ir, checks_full, and is_strictly_weaker_than.
pub enum SpecDigestCheck {
    WorkflowSourceOnly,
    WorkflowAndIr,
    Full,
}

pub open spec fn production_hierarchy_rank(d: SpecDigestCheck) -> int {
    match d {
        SpecDigestCheck::WorkflowSourceOnly => 1,
        SpecDigestCheck::WorkflowAndIr => 2,
        SpecDigestCheck::Full => 3,
    }
}

pub open spec fn production_checks_workflow_source(d: SpecDigestCheck) -> bool {
    production_hierarchy_rank(d) >= production_hierarchy_rank(SpecDigestCheck::WorkflowSourceOnly)
}

pub open spec fn production_checks_compiled_ir(d: SpecDigestCheck) -> bool {
    production_hierarchy_rank(d) >= production_hierarchy_rank(SpecDigestCheck::WorkflowAndIr)
}

pub open spec fn production_checks_full(d: SpecDigestCheck) -> bool {
    production_hierarchy_rank(d) >= production_hierarchy_rank(SpecDigestCheck::Full)
}

pub open spec fn production_is_strictly_weaker_than(a: SpecDigestCheck, b: SpecDigestCheck) -> bool {
    production_hierarchy_rank(a) < production_hierarchy_rank(b)
}

pub proof fn proof_strict_hierarchy()
    ensures
        production_is_strictly_weaker_than(SpecDigestCheck::WorkflowSourceOnly, SpecDigestCheck::WorkflowAndIr),
        production_is_strictly_weaker_than(SpecDigestCheck::WorkflowAndIr, SpecDigestCheck::Full),
        production_is_strictly_weaker_than(SpecDigestCheck::WorkflowSourceOnly, SpecDigestCheck::Full),
{}

pub proof fn proof_level_adds_checks(d: SpecDigestCheck)
    ensures
        production_checks_workflow_source(d),
        production_hierarchy_rank(d) >= 2 ==> production_checks_compiled_ir(d),
        production_hierarchy_rank(d) == 3 ==> production_checks_full(d),
{}

}
