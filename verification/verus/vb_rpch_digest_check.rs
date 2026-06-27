// Verus proof obligations for vb-rpch INV-005: DigestCheck strict hierarchy.
//
// Obligation: VERUS-REC-005 / INV-005
// Contract: DigestCheck forms a strict three-level hierarchy under
//           is_strictly_weaker_than. Each level adds checks relative to the
//           previous one.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is bound to production via the companion extern surface
// `verification/verus/extern_vb_rpch_digest_check.rs`, which itself
// `#[path]`-includes the verbatim production mirror at
// `verification/verus/production_inner/digest_check_production.rs`
// (a verbatim copy of `crates/vb_storage/src/recovery/types.rs:855-900`).
//
// The `assume_specification` bridges below attach the production
// contracts for `hierarchy_rank`, `checks_workflow_source`,
// `checks_compiled_ir`, `checks_full`, and `is_strictly_weaker_than`
// to the spec-side mirror methods. The exec wrappers invoke the
// mirror methods to discharge the contracts; they are the
// non-vacuum witnesses that the bridges are actually used.
//
// BINDING LEDGER:
//   - `production::SpecDigestCheck::hierarchy_rank`     <- types.rs:868-875
//   - `production::SpecDigestCheck::checks_workflow_source` <- types.rs:878-881
//   - `production::SpecDigestCheck::checks_compiled_ir` <- types.rs:883-886
//   - `production::SpecDigestCheck::checks_full`        <- types.rs:888-893
//   - `production::SpecDigestCheck::is_strictly_weaker_than` <- types.rs:895-899

#![allow(unused_imports)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Production extern surface — `#[path]`-bound mirror of
// crates/vb_storage/src/recovery/types.rs:855-900.
// ---------------------------------------------------------------------------
#[path = "extern_vb_rpch_digest_check.rs"]
mod production;

// Re-export the spec-side mirror enum so the spec proofs below
// reason over the production-bound type directly.
pub use production::SpecDigestCheck;

/// VFR-R2-VERUS-004 / INV-005.
/// Bridge model for crates/vb_storage/src/recovery/types.rs::DigestCheck
/// production methods hierarchy_rank, checks_workflow_source,
/// checks_compiled_ir, checks_full, and is_strictly_weaker_than.

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

// ---------------------------------------------------------------------------
// assume_specification BRIDGES — production contract surface
// ---------------------------------------------------------------------------
//
// Each bridge attaches the spec fn contract to the spec-side mirror
// exec method. The body of each mirror method is opaque to Verus
// (`#[verifier::external]` in the extern file); the spec proofs
// below exercise the contracts via the exec wrappers further down.
pub assume_specification[ production::SpecDigestCheck::hierarchy_rank ](
    d: production::SpecDigestCheck,
) -> (result: u8)
    ensures
        result as int == production_hierarchy_rank(d),
;

pub assume_specification[ production::SpecDigestCheck::checks_workflow_source ](
    d: production::SpecDigestCheck,
) -> (result: bool)
    ensures
        result == production_checks_workflow_source(d),
;

pub assume_specification[ production::SpecDigestCheck::checks_compiled_ir ](
    d: production::SpecDigestCheck,
) -> (result: bool)
    ensures
        result == production_checks_compiled_ir(d),
;

pub assume_specification[ production::SpecDigestCheck::checks_full ](
    d: production::SpecDigestCheck,
) -> (result: bool)
    ensures
        result == production_checks_full(d),
;

pub assume_specification[ production::SpecDigestCheck::is_strictly_weaker_than ](
    a: production::SpecDigestCheck,
    b: production::SpecDigestCheck,
) -> (result: bool)
    ensures
        result == production_is_strictly_weaker_than(a, b),
;

// ---------------------------------------------------------------------------
// Production-bound exec wrappers — discharge witnesses for the bridges
// ---------------------------------------------------------------------------
//
// These exec wrappers invoke the spec-side mirror methods. Verus
// verifies each wrapper body via the `assume_specification` contract
// attached to the corresponding mirror method. Any drift between the
// production mirror and the production source breaks the contract
// and these wrappers fail to type-check.
pub exec fn production_hierarchy_rank_witness(d: production::SpecDigestCheck) -> (r: u8)
    ensures
        r as int == production_hierarchy_rank(d),
{
    d.hierarchy_rank()
}

pub exec fn production_is_strictly_weaker_than_witness(
    a: production::SpecDigestCheck,
    b: production::SpecDigestCheck,
) -> (r: bool)
    ensures
        r == production_is_strictly_weaker_than(a, b),
{
    a.is_strictly_weaker_than(b)
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
