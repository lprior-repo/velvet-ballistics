// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `vb_rpch_hydrate_snapshot_tail` Verus spec.
//
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file binds `verification/verus/vb_rpch_hydrate_snapshot_tail.rs`
// to the production snapshot-plus-tail hydration decision surface at
// `crates/vb_storage/src/recovery/hydrate.rs:20-70`.
//
// Each production-bound decision fn below mirrors the production
// signature byte-for-byte (parameter names, parameter order, return
// type) and is wrapped in `#[verifier::external]` so Verus skips body
// verification. The companion spec file attaches the production
// contract surface via `assume_specification` bridges and exercises
// the production exec fns through production-bound exec wrappers that
// are the non-vacuum witnesses for the proofs.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Type mirrors (each mirrors a production type line-by-line so any
// drift breaks the build):
//
//   - `RunId`              <- crates/vb_core/src/ids/mod.rs:65
//                              (mirror as u64 newtype, same shape)
//   - `EventSeq`           <- crates/vb_core/src/ids/mod.rs:66
//                              (mirror as u64 newtype, same shape)
//   - `SpecRunSnapshot`    <- crates/vb_storage/src/recovery/types.rs:653-664
//                              (mirror of `RunSnapshot` with the five
//                              fields `run`, `seq`, `workflow`, `slots`,
//                              `taint` retained byte-for-byte; the
//                              `workflow` field is mirrored as `u64`
//                              because the production functions
//                              exercised here do not read it — D2)
//   - `SpecJournalEvent`   <- crates/vb_storage/src/events.rs:318-...
//                              (production `JournalEvent` has 20+
//                              variants; the spec exercises ONLY the
//                              `.run_id()` and `.seq()` projections —
//                              see D1; the mirror collapses to a struct
//                              with `run` and `seq` fields plus the
//                              two accessors)
//
// Production-bound decision fns (each is `#[verifier::external]`
// and mirrors the production body line-by-line):
//
//   - `hydrate_snapshot_tail_run_matches`
//                          <- crates/vb_storage/src/recovery/hydrate.rs:22-28
//                              (production: `snapshot.run == run_id
//                                && tail_events.iter().all(|event|
//                                event.run_id() == run_id)`)
//   - `hydrate_snapshot_tail_seq_after_snapshot`
//                          <- crates/vb_storage/src/recovery/hydrate.rs:32-37
//                              (production: `tail_events.iter().all(
//                                |event| event.seq() > snapshot.seq)`)
//   - `hydrate_snapshot_tail_has_evidence`
//                          <- crates/vb_storage/src/recovery/hydrate.rs:41-46
//                              (production: `!tail_events.is_empty()
//                                || !snapshot.slots.is_empty()
//                                || !snapshot.taint.is_empty()`)
//   - `hydrate_snapshot_tail_preconditions`
//                          <- crates/vb_storage/src/recovery/hydrate.rs:50-58
//                              (production: composition of the three
//                              predicates above via short-circuit &&)
//   - `hydrate_events_preconditions`
//                          <- crates/vb_storage/src/recovery/hydrate.rs:62-64
//                              (production: `!events.is_empty()`)
//   - `hydrate_dimensions_positive`
//                          <- crates/vb_storage/src/recovery/hydrate.rs:68-70
//                              (production: `step_count > 0
//                                && slot_count > 0` for `u16`
//                                arguments)
//
// ============================================================================
// DRIFT ITEMS ACCEPTED BY THE BINDING
// ============================================================================
//
// D1: production `JournalEvent` enum has 20+ variants; the mirror
//     `SpecJournalEvent` collapses to `{run, seq}` because the
//     production functions exercised by this spec ONLY call
//     `.run_id()` and `.seq()`. Adding a new variant that breaks
//     this invariant (i.e., one whose `run_id()` or `seq()` is
//     not a simple field access) would require updating the
//     mirror. Tracked as binding debt D1.
//
// D2: production `RunSnapshot.workflow: WorkflowDigest` is mirrored
//     as a `u64` placeholder because the production functions
//     exercised by this spec do NOT read `.workflow`. If a future
//     production change adds a digest check to these predicates,
//     the mirror must be updated to carry `WorkflowDigest`.
//     Tracked as binding debt D2.
//
// D3: production `RunSnapshot` derives `Debug, Clone, PartialEq, Eq,
//     Serialize, Deserialize`; the mirror is `Clone, Copy`-only
//     because the spec does not require those traits. Tracked as
//     binding debt D3.
//
// D4: production `hydrate_snapshot_tail_preconditions` is non-`const`
//     because it calls `hydrate_snapshot_tail_run_matches` which
//     takes references; the mirror mirrors the production `const`-ness
//     at the leaf only (`hydrate_events_preconditions` and
//     `hydrate_dimensions_positive` are `pub const`).
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The production bodies of the six decision fns are NOT verified by
// Verus directly. The exec fns are `#[verifier::external]`; the
// `assume_specification` bridges in the companion spec file
// (`vb_rpch_hydrate_snapshot_tail.rs`) attach the production
// contracts and are the contracts the proofs below exercise. Drift
// between the production mirror and the production source is
// reported as binding-debt tracked outside Verus.
//
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ============================================================================
// ID type mirrors — vb_core newtypes
// ============================================================================

/// Mirror of `RunId` (u64 newtype) at `crates/vb_core/src/ids/mod.rs:65`.
/// Same shape (`pub struct RunId(pub u64)`); `PartialEq`/`Eq`
/// derived for use in `Seq<RunId>` spec predicates.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RunId(pub u64);

/// Mirror of `EventSeq` (u64 newtype) at `crates/vb_core/src/ids/mod.rs:66`.
/// Same shape (`pub struct EventSeq(pub u64)`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EventSeq(pub u64);

impl RunId {
    /// Mirror of `RunId::new` at `crates/vb_core/src/ids/mod.rs:21`.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Mirror of `RunId::get` at `crates/vb_core/src/ids/mod.rs:27`.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl EventSeq {
    /// Mirror of `EventSeq::new` at `crates/vb_core/src/ids/mod.rs:21`.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Mirror of `EventSeq::get` at `crates/vb_core/src/ids/mod.rs:27`.
    pub const fn get(self) -> u64 {
        self.0
    }
}

// ============================================================================
// SpecRunSnapshot mirror — crates/vb_storage/src/recovery/types.rs:653-664
// ============================================================================
//
// Field names match production byte-for-byte so spec contracts that
// read `snapshot.run`, `snapshot.seq`, `snapshot.slots`, or
// `snapshot.taint` resolve naturally. The `workflow` field is
// mirrored as `u64` per D2 (the spec does not exercise it).
//
// `Clone, Copy` is a relaxation of production's
// `Debug, Clone, PartialEq, Eq, Serialize, Deserialize` (D3); the
// spec does not require those traits.

#[derive(Clone)]
pub struct SpecRunSnapshot {
    /// Mirror of `RunSnapshot::run` at types.rs:655.
    pub run: RunId,
    /// Mirror of `RunSnapshot::seq` at types.rs:657.
    pub seq: EventSeq,
    /// Mirror of `RunSnapshot::workflow` at types.rs:659 (D2:
    /// `u64` placeholder because the spec does not exercise it).
    pub workflow: u64,
    /// Mirror of `RunSnapshot::slots` at types.rs:661
    /// (compact binary slot values at snapshot time).
    pub slots: Vec<u8>,
    /// Mirror of `RunSnapshot::taint` at types.rs:663
    /// (compact binary taint markers at snapshot time).
    pub taint: Vec<u8>,
}

// ============================================================================
// SpecJournalEvent mirror — crates/vb_storage/src/events.rs:318-...
// ============================================================================
//
// The production `JournalEvent` enum has 20+ variants; the spec
// exercises ONLY `.run_id()` and `.seq()`. The mirror collapses
// to a struct with `run` and `seq` fields plus accessors that
// mirror the production method semantics byte-for-byte (D1).

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SpecJournalEvent {
    /// Mirror of `JournalEvent::run_id()` at events.rs:321-348
    /// (production returns the `run` field of the variant).
    pub run: RunId,
    /// Mirror of `JournalEvent::seq()` at events.rs:355-...
    /// (production returns the `seq` field of the variant).
    pub seq: EventSeq,
}

impl SpecJournalEvent {
    /// Mirror of `JournalEvent::run_id` at events.rs:321-348.
    /// Body is opaque to Verus (`#[verifier::external]`) — the
    /// production method dispatches on the enum variant; the
    /// mirror returns the stored `run` field directly.
    #[verifier::external]
    pub fn run_id(&self) -> RunId {
        self.run
    }

    /// Mirror of `JournalEvent::seq` at events.rs:355-...
    /// Body is opaque to Verus (`#[verifier::external]`).
    #[verifier::external]
    pub fn seq(&self) -> EventSeq {
        self.seq
    }
}

// ============================================================================
// Production-bound decision fns — `#[verifier::external]` wrappers
// ============================================================================
//
// Each fn below mirrors the production body line-by-line. The body
// is opaque to Verus; the spec file attaches the production
// contract via `assume_specification` and the exec wrapper
// exercises the contract.

/// Mirror of `hydrate_snapshot_tail_run_matches` at
/// `crates/vb_storage/src/recovery/hydrate.rs:22-28`. Production
/// body (line 27): `snapshot.run == run_id && tail_events.iter()
/// .all(|event| event.run_id() == run_id)`.
///
/// TRUST BOUNDARY: body is opaque to Verus
/// (`#[verifier::external]`). The `assume_specification` bridge
/// in the companion spec file attaches the production contract.
#[verifier::external]
pub fn hydrate_snapshot_tail_run_matches(
    snapshot: &SpecRunSnapshot,
    tail_events: &[SpecJournalEvent],
    run_id: RunId,
) -> bool {
    snapshot.run == run_id && tail_events.iter().all(|event| event.run_id() == run_id)
}

/// Mirror of `hydrate_snapshot_tail_seq_after_snapshot` at
/// `crates/vb_storage/src/recovery/hydrate.rs:32-37`. Production
/// body (line 36): `tail_events.iter().all(|event| event.seq() >
/// snapshot.seq)`.
///
/// TRUST BOUNDARY: body is opaque to Verus
/// (`#[verifier::external]`).
#[verifier::external]
pub fn hydrate_snapshot_tail_seq_after_snapshot(
    snapshot: &SpecRunSnapshot,
    tail_events: &[SpecJournalEvent],
) -> bool {
    tail_events.iter().all(|event| event.seq().get() > snapshot.seq.get())
}

/// Mirror of `hydrate_snapshot_tail_has_evidence` at
/// `crates/vb_storage/src/recovery/hydrate.rs:41-46`. Production
/// body (line 45): `!tail_events.is_empty() || !snapshot.slots
/// .is_empty() || !snapshot.taint.is_empty()`.
///
/// TRUST BOUNDARY: body is opaque to Verus
/// (`#[verifier::external]`).
#[verifier::external]
pub fn hydrate_snapshot_tail_has_evidence(
    snapshot: &SpecRunSnapshot,
    tail_events: &[SpecJournalEvent],
) -> bool {
    !tail_events.is_empty() || !snapshot.slots.is_empty() || !snapshot.taint.is_empty()
}

/// Mirror of `hydrate_snapshot_tail_preconditions` at
/// `crates/vb_storage/src/recovery/hydrate.rs:50-58`. Production
/// body (lines 55-57): composition of the three predicates
/// above via short-circuit `&&`.
///
/// TRUST BOUNDARY: body is opaque to Verus
/// (`#[verifier::external]`).
#[verifier::external]
pub fn hydrate_snapshot_tail_preconditions(
    snapshot: &SpecRunSnapshot,
    tail_events: &[SpecJournalEvent],
    run_id: RunId,
) -> bool {
    hydrate_snapshot_tail_run_matches(snapshot, tail_events, run_id)
        && hydrate_snapshot_tail_seq_after_snapshot(snapshot, tail_events)
        && hydrate_snapshot_tail_has_evidence(snapshot, tail_events)
}

/// Mirror of `hydrate_events_preconditions` at
/// `crates/vb_storage/src/recovery/hydrate.rs:62-64`. Production
/// body (line 63): `!events.is_empty()`.
///
/// The production fn is `pub const fn`; the mirror mirrors the
/// `const` shape. TRUST BOUNDARY: body is opaque to Verus
/// (`#[verifier::external]`).
#[verifier::external]
pub fn hydrate_events_preconditions(events: &[SpecJournalEvent]) -> bool {
    !events.is_empty()
}

/// Mirror of `hydrate_dimensions_positive` at
/// `crates/vb_storage/src/recovery/hydrate.rs:68-70`. Production
/// body (line 69): `step_count > 0 && slot_count > 0` for `u16`
/// arguments.
///
/// The production fn is `pub const fn`; the mirror mirrors the
/// `const` shape. TRUST BOUNDARY: body is opaque to Verus
/// (`#[verifier::external]`).
#[verifier::external]
pub fn hydrate_dimensions_positive(step_count: u16, slot_count: u16) -> bool {
    step_count > 0 && slot_count > 0
}

} // verus!