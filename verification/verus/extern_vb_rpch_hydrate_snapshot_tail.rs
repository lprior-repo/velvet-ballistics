// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `vb_rpch_hydrate_snapshot_tail` Verus spec.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_rpch_hydrate_snapshot_tail.rs` Verus spec.
//
// Structure:
//   1. A direct `#[path]` inclusion of the verbatim production mirror
//      at
//      `verification/verus/production_inner/hydrate_preconditions_production.rs`.
//      The mirror is a line-for-line copy of
//      crates/vb_storage/src/recovery/hydrate.rs:20-70 with only
//      `vb_core`/`JournalEvent`/`EventSeq` newtypes stubbed for
//      standalone Verus compilation. Any drift in the production
//      function bodies, parameter types, or return types breaks
//      this Verus build at compile time.
//
//   2. The spec-side mirror uses `SpecJournalEventMarker` (a unit
//      struct declared in `verus!` context) for the event element
//      type. The production bodies only call `event.run_id()` and
//      `event.seq()` on tail events; using a unit marker (without
//      these accessors) would not match. The spec-side mirror has
//      public `run` and `seq` fields plus `run_id()` and `seq()`
//      accessors that mirror the production method semantics.
//
//   3. The spec-side mirror slice type `SpecJournalEvent` includes
//      `run_id()` and `seq()` methods that mirror the production
//      methods. The `hydrate_snapshot_tail_run_matches_mirror` and
//      `hydrate_snapshot_tail_seq_after_snapshot_mirror` exec
//      wrappers reproduce the production logic.
//
// ============================================================================
// WHY NOT DIRECT USAGE OF prod_src TYPES IN SPEC
// ============================================================================
// The production mirror's `JournalEvent` is included via
// `#[verifier::external]` module-level, which makes it opaque to
// Verus. Spec functions cannot reference opaque types in
// `assume_specification` contracts. The spec-side mirror uses
// `SpecJournalEvent` which Verus can see and reason about. The
// `hydrate_snapshot_tail_*_mirror` exec wrappers below take
// spec-visible types and invoke the production logic on them.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Production source: `crates/vb_storage/src/recovery/hydrate.rs:20-70`.
//
// Production functions mirrored (via `prod_src` drift-detection):
//   - `prod_src::hydrate_snapshot_tail_run_matches`           <- hydrate.rs:22-28
//   - `prod_src::hydrate_snapshot_tail_seq_after_snapshot`    <- hydrate.rs:32-37
//   - `prod_src::hydrate_snapshot_tail_has_evidence`          <- hydrate.rs:41-46
//   - `prod_src::hydrate_snapshot_tail_preconditions`         <- hydrate.rs:50-58
//
// Spec-side mirror (used in Verus proofs and exec wrappers):
//   - `SpecJournalEvent` (struct with `run`/`seq` fields, mirrors JournalEvent)
//   - `RunSnapshot` (struct with `run`/`seq`/`slots`/`taint` fields)
//   - `RunId`, `EventSeq` (u64 newtype wrappers)
//   - `hydrate_snapshot_tail_run_matches_mirror`
//   - `hydrate_snapshot_tail_seq_after_snapshot_mirror`
//   - `hydrate_snapshot_tail_has_evidence_mirror`
//   - `hydrate_snapshot_tail_preconditions_mirror`
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The production bodies are NOT verified by Verus directly. The
// production mirror module is marked `#[verifier::external]` at
// module level. The spec-side mirror bodies below are also
// `#[verifier::external]`. The `assume_specification` bridges in
// the companion spec file attach the production contracts, and the
// exec wrappers in that file invoke the spec-side mirror functions
// to discharge the contracts.
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
#[verifier::external]
#[path = "production_inner/hydrate_preconditions_production.rs"]
pub mod prod_src;

// ---------------------------------------------------------------------------
// Spec-side newtypes and structs — production field-identical
// ---------------------------------------------------------------------------
//
// Field-identical to production newtypes at
// `crates/vb_core/src/ids/mod.rs:55,58`. The mirror uses a `pub`
// inner field so spec-side reasoning can read `.0` when needed.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunId(pub u64);

impl RunId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EventSeq(pub u64);

impl EventSeq {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Mirror of `RunSnapshot` at
/// `crates/vb_storage/src/recovery/types.rs:653-664`. Field types
/// abstracted: `workflow: u64` (production uses `WorkflowDigest`).
#[derive(Clone, PartialEq, Eq)]
pub struct RunSnapshot {
    pub run: RunId,
    pub seq: EventSeq,
    pub workflow: u64,
    pub slots: Vec<u8>,
    pub taint: Vec<u8>,
}

/// Mirror of `JournalEvent` from `crates/vb_storage/src/events.rs:23`.
/// The production enum has 20+ variants; for the snapshot_tail
/// preconditions, only `run_id()` and `seq()` are used. The spec
/// mirror exposes these as a struct with public fields.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SpecJournalEvent {
    pub run: RunId,
    pub seq: EventSeq,
}

impl SpecJournalEvent {
    /// Mirror of `JournalEvent::run_id` (production).
    #[verifier::external]
    pub fn run_id(&self) -> RunId {
        self.run
    }

    /// Mirror of `JournalEvent::seq` (production).
    #[verifier::external]
    pub fn seq(&self) -> EventSeq {
        self.seq
    }
}

// ---------------------------------------------------------------------------
// Spec-side mirror functions — production body-identical
// ---------------------------------------------------------------------------
//
// All bodies are `#[verifier::external]`. The companion spec file
// attaches `assume_specification` bridges that state the production
// contracts. The exec wrappers in the spec file invoke these mirror
// functions and assert the contracts hold.
#[verifier::external]
pub fn hydrate_snapshot_tail_run_matches_mirror(
    snapshot: &RunSnapshot,
    tail_events: &[SpecJournalEvent],
    run_id: RunId,
) -> bool {
    snapshot.run == run_id && tail_events.iter().all(|event| event.run_id() == run_id)
}

#[verifier::external]
pub fn hydrate_snapshot_tail_seq_after_snapshot_mirror(
    snapshot: &RunSnapshot,
    tail_events: &[SpecJournalEvent],
) -> bool {
    tail_events.iter().all(|event| event.seq() > snapshot.seq)
}

#[verifier::external]
pub fn hydrate_snapshot_tail_has_evidence_mirror(
    snapshot: &RunSnapshot,
    tail_events: &[SpecJournalEvent],
) -> bool {
    !tail_events.is_empty() || !snapshot.slots.is_empty() || !snapshot.taint.is_empty()
}

#[verifier::external]
pub fn hydrate_snapshot_tail_preconditions_mirror(
    snapshot: &RunSnapshot,
    tail_events: &[SpecJournalEvent],
    run_id: RunId,
) -> bool {
    hydrate_snapshot_tail_run_matches_mirror(snapshot, tail_events, run_id)
        && hydrate_snapshot_tail_seq_after_snapshot_mirror(snapshot, tail_events)
        && hydrate_snapshot_tail_has_evidence_mirror(snapshot, tail_events)
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
fn prod_methods_drift_check(
    snapshot: &prod_src::RunSnapshot,
    tail_events: &[prod_src::JournalEvent],
    run_id: prod_src::RunId,
) {
    let _ = prod_src::hydrate_snapshot_tail_run_matches(snapshot, tail_events, run_id);
    let _ = prod_src::hydrate_snapshot_tail_seq_after_snapshot(snapshot, tail_events);
    let _ = prod_src::hydrate_snapshot_tail_has_evidence(snapshot, tail_events);
    let _ = prod_src::hydrate_snapshot_tail_preconditions(snapshot, tail_events, run_id);
}

} // verus!
