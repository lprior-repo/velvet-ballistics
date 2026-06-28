// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `vb_rpch_hydrate_events` Verus spec.
//
// ============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_rpch_hydrate_events.rs` Verus spec.
//
// Structure:
//   1. A direct `#[path]` inclusion of the verbatim production mirror
//      at
//      `verification/verus/production_inner/hydrate_preconditions_production.rs`.
//      The mirror is a line-for-line copy of
//      crates/vb_storage/src/recovery/hydrate.rs:20-70. The mirror
//      is marked `#[verifier::external]` at module level so its
//      types are opaque to Verus.
//
//   2. A spec-side mirror slice type using `SpecJournalEventMarker`
//      (a unit struct declared in `verus!` context). The production
//      `hydrate_events_preconditions` body is `!events.is_empty()`,
//      which is invariant under the element type. Using a unit
//      marker in spec mode is a faithful structural projection.
//
//   3. The production-bound exec wrappers
//      `hydrate_events_preconditions_mirror` and
//      `hydrate_dimensions_positive_mirror` are declared in `verus!`
//      context below with bodies that exactly reproduce the
//      production logic. The bodies are marked
//      `#[verifier::external]` so Verus skips body verification.
//
// ============================================================================
// WHY NOT DIRECT USAGE OF prod_src TYPES IN SPEC
// ============================================================================
// The production mirror's `JournalEvent` is included via
// `#[verifier::external]` module-level, which makes it opaque to
// Verus. Spec functions cannot reference opaque types in
// `assume_specification` contracts. The spec-side mirror uses a unit
// marker `SpecJournalEventMarker` which Verus can see and reason
// about. The `hydrate_events_preconditions_mirror` exec wrapper
// below takes `&[SpecJournalEventMarker]` and returns
// `!events.is_empty()`, which is invariant under the element type
// (the production body returns the same value for
// `&[JournalEvent]`).
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Production source: `crates/vb_storage/src/recovery/hydrate.rs:20-70`.
//
// Production functions mirrored (via `prod_src` drift-detection):
//   - `prod_src::hydrate_events_preconditions`     <- hydrate.rs:62-64
//   - `prod_src::hydrate_dimensions_positive`      <- hydrate.rs:68-70
//
// Spec-side mirror (used in Verus proofs and exec wrappers):
//   - `hydrate_events_preconditions_mirror(events: &[SpecJournalEventMarker]) -> bool`
//   - `hydrate_dimensions_positive_mirror(step_count: u16, slot_count: u16) -> bool`
//   - `SpecJournalEventMarker` (unit struct)
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The production bodies of `hydrate_events_preconditions` and
// `hydrate_dimensions_positive` are NOT verified by Verus directly.
// The production mirror module is marked `#[verifier::external]` at
// module level. The spec-side mirror bodies below are also
// `#[verifier::external]`. The `assume_specification` bridges in
// the companion spec file attach the production contracts, and the
// exec wrappers in that file invoke the spec-side mirror functions
// to discharge the contracts. Drift between the production mirror
// and the production source is reported as binding-debt tracked
// outside Verus.
//
// ============================================================================
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Drift-detection inclusion: `#[path]` to verbatim production mirror
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the verbatim production mirror at
// `production_inner/hydrate_preconditions_production.rs`. The mirror
// is marked `#[verifier::external]` at module level so the
// production bodies are opaque to Verus; the inclusion still
// validates Rust resolution (function names, parameter types, return
// types) at compile time. Any drift in the production impl surface
// breaks this Verus build.
#[verifier::external]
#[path = "production_inner/hydrate_preconditions_production.rs"]
pub mod prod_src;

// ---------------------------------------------------------------------------
// Spec-side unit marker — closed projection of `JournalEvent`
// ---------------------------------------------------------------------------
//
// `crate::JournalEvent` is a 20+ variant `#[non_exhaustive]` enum
// at `crates/vb_storage/src/events.rs:23`. The production
// `hydrate_events_preconditions` body is `!events.is_empty()`,
// which is invariant under the element type. A unit marker is a
// sound closed projection for spec reasoning.
#[derive(Clone, Copy)]
pub struct SpecJournalEventMarker {
    pub _phantom: (),
}

impl SpecJournalEventMarker {
    #[verifier::external]
    pub fn new() -> Self {
        Self { _phantom: () }
    }
}

// ---------------------------------------------------------------------------
// Spec-side mirror — production body-identical
// ---------------------------------------------------------------------------
//
// The bodies are byte-for-byte copies of the production logic.
// `#[verifier::external]` skips body verification. The
// `assume_specification` bridges in the companion spec file attach
// the production contracts: `hydrate_events_preconditions_mirror`
// returns `events@.len() > 0`, and
// `hydrate_dimensions_positive_mirror` returns
// `step_count > 0 && slot_count > 0`.
#[verifier::external]
pub fn hydrate_events_preconditions_mirror(events: &[SpecJournalEventMarker]) -> bool {
    !events.is_empty()
}

#[verifier::external]
pub fn hydrate_dimensions_positive_mirror(step_count: u16, slot_count: u16) -> bool {
    step_count > 0 && slot_count > 0
}

// ---------------------------------------------------------------------------
// Phantom drift-detection helper
// ---------------------------------------------------------------------------
//
// The body is `#[verifier::external]` (opaque to Verus), but the
// `prod_src::*` references force Rust to resolve the production
// function names at compile time. A rename of any of these
// production functions (or its parameter types) breaks this fn's
// compilation.
#[verifier::external]
fn prod_methods_drift_check(events: &[prod_src::JournalEvent], step_count: u16, slot_count: u16) {
    let _ = prod_src::hydrate_events_preconditions(events);
    let _ = prod_src::hydrate_dimensions_positive(step_count, slot_count);
}

} // verus!
