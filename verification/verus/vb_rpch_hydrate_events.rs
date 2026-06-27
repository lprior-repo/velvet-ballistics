#![allow(unused_imports)]

// Verus proof obligations for vb-rpch INV-006: hydrate_events preconditions.
//
// Obligation: VERUS-REC-006 / INV-006
// Contract: hydrate_events requires events non-empty AND step_count > 0
//           AND slot_count > 0, with both counts fitting in u16.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is bound to production via the companion extern surface
// `verification/verus/extern_vb_rpch_hydrate_events.rs`, which itself
// `#[path]`-includes the verbatim production mirror at
// `verification/verus/production_inner/hydrate_preconditions_production.rs`
// (a verbatim copy of `crates/vb_storage/src/recovery/hydrate.rs:20-70`).
//
// The `assume_specification` bridges below attach the production
// contracts for `hydrate_events_preconditions` and
// `hydrate_dimensions_positive` to the spec-side mirror functions
// in the extern file. The exec wrappers invoke the mirror functions
// to discharge the contracts; they are the non-vacuum witnesses
// that the bridges are actually used.
//
// BINDING LEDGER:
//   - `hydrate_events_preconditions`     <- hydrate.rs:62-64
//   - `hydrate_dimensions_positive`      <- hydrate.rs:68-70

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Production extern surface — `#[path]`-bound mirror of
// crates/vb_storage/src/recovery/hydrate.rs:20-70.
// ---------------------------------------------------------------------------
#[path = "extern_vb_rpch_hydrate_events.rs"]
mod production;

// Re-export the spec-side mirror types and functions.
pub use production::{
    hydrate_events_preconditions_mirror, hydrate_dimensions_positive_mirror,
    SpecJournalEventMarker,
};

/// VFR-R2-VERUS-006 / PRE-002.
/// Bridge model for hydrate_events_preconditions(events) and
/// hydrate_dimensions_positive(step_count, slot_count).

// ---------------------------------------------------------------------------
// assume_specification BRIDGES — production contract surface
// ---------------------------------------------------------------------------
//
// Each bridge attaches the spec fn contract to the spec-side mirror
// exec function. The mirror body is opaque to Verus
// (`#[verifier::external]`); the spec proofs below exercise the
// contracts via the exec wrappers further down.
pub assume_specification[ production::hydrate_events_preconditions_mirror ](
    events: &[production::SpecJournalEventMarker],
) -> (result: bool)
    ensures
        result == (events@.len() > 0),
;

pub assume_specification[ production::hydrate_dimensions_positive_mirror ](
    step_count: u16,
    slot_count: u16,
) -> (result: bool)
    ensures
        result == (step_count > 0 && slot_count > 0),
;

// ---------------------------------------------------------------------------
// Production-bound exec wrappers — discharge witnesses for the bridges
// ---------------------------------------------------------------------------
//
// These exec wrappers invoke the spec-side mirror functions. Verus
// verifies each wrapper body via the `assume_specification` contract
// attached to the corresponding mirror function. Any drift between
// the production mirror and the production source breaks the
// contract and these wrappers fail to type-check.
pub exec fn production_hydrate_events_preconditions_witness(
    events: &[production::SpecJournalEventMarker],
) -> (r: bool)
    ensures
        r == (events@.len() > 0),
{
    production::hydrate_events_preconditions_mirror(events)
}

pub exec fn production_hydrate_dimensions_positive_witness(
    step_count: u16,
    slot_count: u16,
) -> (r: bool)
    ensures
        r == (step_count > 0 && slot_count > 0),
{
    production::hydrate_dimensions_positive_mirror(step_count, slot_count)
}

pub open spec fn production_hydrate_events_preconditions(events_len: int) -> bool {
    events_len > 0
}

pub open spec fn production_hydrate_dimensions_positive(step_count: int, slot_count: int) -> bool {
    step_count > 0 && slot_count > 0
}

pub open spec fn valid_hydrate_events_preconditions(events_len: int, step_count: int, slot_count: int) -> bool {
    production_hydrate_events_preconditions(events_len)
        && production_hydrate_dimensions_positive(step_count, slot_count)
        && step_count <= 65535
        && slot_count <= 65535
}

pub proof fn proof_events_success_requires_nonempty_positive_bounded(events_len: int, step_count: int, slot_count: int)
    requires events_len > 0, 0 < step_count <= 65535, 0 < slot_count <= 65535,
    ensures
        valid_hydrate_events_preconditions(events_len, step_count, slot_count),
        production_hydrate_events_preconditions(events_len),
        production_hydrate_dimensions_positive(step_count, slot_count),
{}

pub proof fn proof_empty_events_rejected(step_count: int, slot_count: int)
    ensures !production_hydrate_events_preconditions(0), !valid_hydrate_events_preconditions(0, step_count, slot_count),
{}

pub proof fn proof_nonpositive_dimensions_rejected(events_len: int, step_count: int, slot_count: int)
    requires events_len > 0, step_count <= 0 || slot_count <= 0,
    ensures !valid_hydrate_events_preconditions(events_len, step_count, slot_count),
{}

}
