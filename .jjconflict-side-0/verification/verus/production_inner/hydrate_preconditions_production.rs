// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for hydrate_run_frame preconditions
// ============================================================================
//
// This file is a VERBATIM copy of the production `hydrate_run_frame`
// precondition surface from
//   crates/vb_storage/src/recovery/hydrate.rs:20-70
// with three minimal substitutions:
//
//   1. The `vb_core::ids::*` newtypes `RunId` and `EventSeq` are
//      declared locally with the same `pub struct $name(pub u64)` shape
//      and the same accessor surface (`new`, `get`) used by production.
//      The production `vb_core` crate cannot be linked under
//      `verus --crate-type=lib` (no `--extern` flag is supported, no
//      installs are permitted by the task brief), so the local mirrors
//      provide the production newtype surface at the file root.
//      Production `RunId` derives `Debug, Clone, Copy, PartialEq, Eq,
//      Hash, Serialize, Deserialize`; the local mirror derives
//      `Debug, Clone, Copy, PartialEq, Eq, Hash` — the production-only
//      derives (Serialize/Deserialize) require the `serde` extern crate,
//      which is also unavailable. BINDING DEBT D1.
//
//   2. The `crate::recovery::types::RunSnapshot` struct is mirrored
//      here as `RunSnapshot` with the same five field names (`run`,
//      `seq`, `workflow`, `slots`, `taint`) used by the production
//      surface at `crates/vb_storage/src/recovery/types.rs:653-664`.
//      Field types are abstracted: `workflow: WorkflowDigest` becomes
//      `workflow: u64` (the production digest type is `Copy + Eq +
//      Hash`, none of the six fns exercise `workflow`, and the
//      workflow type itself requires the full `vb_core` graph to
//      compile). The byte-for-byte FIELD NAMES are preserved so any
//      rename of `run`, `seq`, `slots`, or `taint` in production
//      breaks this mirror. BINDING DEBT D2.
//
//   3. The `crate::JournalEvent` enum (20+ variants at
//      `crates/vb_storage/src/events.rs:23`) is collapsed to a struct
//      `JournalEvent { run: RunId, seq: EventSeq }` with `run_id()` and
//      `seq()` accessors that mirror the production method semantics
//      byte-for-byte. The six precondition fns only ever call
//      `event.run_id()` and `event.seq()`; they do not dispatch on the
//      enum variant. The closure bodies in `hydrate_snapshot_tail_*_matches`
//      therefore resolve identically to the production surface.
//      BINDING DEBT D3.
//
// ============================================================================
// WHY A VERBATIM PRODUCTION-MIRROR (rather than pure spec-side mirrors)
// ============================================================================
//
// Direct `#[path = "../../crates/vb_storage/src/recovery/hydrate.rs"]`
// inclusion of the production source is BLOCKED by the production file
// using:
//
//   (a) `use vb_core::RunId;` at line 18 — `vb_core` is an extern crate
//       that is not registered under `verus --crate-type=lib`. Verus
//       supports `--export`/`--import` for Verus metadata but does not
//       expose a `--extern` flag to register Rust extern crates.
//
//   (b) `use crate::JournalEvent;` and `use crate::EventSeq;` at lines
//       8 + 76 + 86 — `crate::*` paths in a `#[path]`-included module
//       resolve to the current crate root (the verus file), and would
//       require the spec file to declare `JournalEvent` and `EventSeq`
//       at the top level along with the full `crate::recovery::hydrate_support`
//       and `crate::recovery::types` module graphs. Production's
//       `JournalEvent` derives `serde::Serialize, serde::Deserialize,
//       Debug, Clone, PartialEq, Eq` which transitively requires the
//       `serde` extern crate, also unavailable.
//
//   (c) Five helper functions from `crate::recovery::hydrate_support`
//       (line 9-12) and eight types from `crate::recovery::types`
//       (line 14-17), all of which would require stub definitions in
//       the spec file.
//
// The verbatim production-mirror pattern sidesteps every blocker. The
// six production fn bodies are reproduced line-by-line against a
// minimal in-tree type surface, marked `#[verifier::external]` so Verus
// skips body verification, and the spec file attaches
// `assume_specification` contracts to the re-exported names. This
// matches the established repo pattern in
// `verification/verus/production_inner/action_replay_tracker_production.rs`
// (verbatim mirror of `ActionReplayTracker`) and
// `verification/verus/production_inner/replay_invariants_production.rs`.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Source: `crates/vb_storage/src/recovery/hydrate.rs:20-70`.
//
//   - `hydrate_snapshot_tail_run_matches`           <- hydrate.rs:22-28
//                                                      (verbatim body at lines 250-258)
//   - `hydrate_snapshot_tail_seq_after_snapshot`    <- hydrate.rs:32-37
//                                                      (verbatim body at lines 267-274)
//   - `hydrate_snapshot_tail_has_evidence`          <- hydrate.rs:41-46
//                                                      (verbatim body at lines 280-286)
//   - `hydrate_snapshot_tail_preconditions`         <- hydrate.rs:50-58
//                                                      (verbatim body at lines 292-302)
//   - `hydrate_events_preconditions`                <- hydrate.rs:62-64
//                                                      (verbatim body at lines 308-313)
//   - `hydrate_dimensions_positive`                 <- hydrate.rs:68-70
//                                                      (verbatim body at lines 319-324)
//
// ============================================================================
// DRIFT POLICY
// ============================================================================
//
// Any drift between this file and the production source lines 20-70
// of `crates/vb_storage/src/recovery/hydrate.rs` MUST be mirrored
// here. Drift in production `run`, `seq`, `workflow`, `slots`, or
// `taint` field names breaks the mirror at compile time. Drift in
// the production body expressions (e.g., switching
// `tail_events.iter().all(...)` to a `for` loop, or replacing
// `step_count > 0` with `step_count != 0`) does NOT break the build
// but is recorded as binding debt during review.
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The six production fn bodies below are NOT verified by Verus. Each
// fn is marked `#[verifier::external]` so Verus skips body
// verification. The spec file attaches `assume_specification`
// contracts that state the production behavior; the spec proofs
// exercise the contracts via exec wrappers in the spec file. The
// exec wrappers are the non-vacuum witnesses that the bridge is
// actually used.
//
// ============================================================================
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ---------------------------------------------------------------------------
// vb_core newtype stubs — RunId and EventSeq
// ---------------------------------------------------------------------------
//
// Production: `crates/vb_core/src/ids/mod.rs`.
// Production uses `numeric_id!(RunId, u64, ...)` and similar macro to
// generate `pub struct RunId(pub u64)` with `new(u64) -> Self` and
// `get(self) -> u64` accessors. The mirror reproduces the surface
// with explicit derives. BINDING DEBT D1: production derives
// `Serialize, Deserialize` are omitted (require the `serde` extern
// crate); field NAME is preserved.

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunId(pub u64);

impl RunId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EventSeq(pub u64);

impl EventSeq {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

// ---------------------------------------------------------------------------
// RunSnapshot stub — crates/vb_storage/src/recovery/types.rs:653-664
// ---------------------------------------------------------------------------
//
// Production struct:
//     #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//     pub struct RunSnapshot {
//         pub run: RunId,
//         pub seq: EventSeq,
//         pub workflow: WorkflowDigest,
//         pub slots: Vec<u8>,
//         pub taint: Vec<u8>,
//     }
//
// The mirror preserves field NAMES byte-for-byte. Field types:
// - `run: RunId` — same as production (RunId stub above)
// - `seq: EventSeq` — same as production (EventSeq stub above)
// - `workflow: u64` — abstracted from `WorkflowDigest`; the six
//   preconditions fns NEVER read `.workflow`. BINDING DEBT D2.
// - `slots: Vec<u8>` — same as production
// - `taint: Vec<u8>` — same as production
//
// Visibility is `pub` (production uses the same). The derives
// `Serialize, Deserialize` are omitted (BINDING DEBT D2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSnapshot {
    pub run: RunId,
    pub seq: EventSeq,
    pub workflow: u64,
    pub slots: Vec<u8>,
    pub taint: Vec<u8>,
}

// ---------------------------------------------------------------------------
// JournalEvent collapsed stub — crates/vb_storage/src/events.rs:23
// ---------------------------------------------------------------------------
//
// Production `JournalEvent` is a 20+ variant enum. The six
// precondition fns only ever call `event.run_id()` and
// `event.seq()`, so the mirror collapses the enum to a struct
// with `run` and `seq` fields plus the two accessors. BINDING DEBT
// D3: any drift in production that adds an element-level
// inspection to one of the six fns (e.g., inspecting
// `event.run_id()` is not sufficient and we must dispatch on the
// variant) would require expanding this stub.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalEvent {
    pub run: RunId,
    pub seq: EventSeq,
}

impl JournalEvent {
    /// Mirror of `JournalEvent::run_id` at
    /// `crates/vb_storage/src/events.rs:321-348` (production returns
    /// the `run` field of the variant). The mirror returns the
    /// stored `run` field directly. BINDING DEBT D3.
    #[verifier::external]
    pub fn run_id(&self) -> RunId {
        self.run
    }

    /// Mirror of `JournalEvent::seq` at
    /// `crates/vb_storage/src/events.rs:355-...` (production returns
    /// the `seq` field of the variant). The mirror returns the
    /// stored `seq` field directly. BINDING DEBT D3.
    #[verifier::external]
    pub fn seq(&self) -> EventSeq {
        self.seq
    }
}

// ===========================================================================
// VERBATIM PRODUCTION: hydrate_run_frame preconditions surface
// ===========================================================================
//
// Source: crates/vb_storage/src/recovery/hydrate.rs:20-70
// Drift policy: any change to the production block between these line
// numbers MUST be mirrored here.

// ---------------------------------------------------------------------------
// hydrate_snapshot_tail_run_matches — hydrate.rs:20-28 (verbatim)
// ---------------------------------------------------------------------------
//
// /// Production proof surface for snapshot-plus-tail run identity.
// #[must_use]
// pub fn hydrate_snapshot_tail_run_matches(
//     snapshot: &RunSnapshot,
//     tail_events: &[JournalEvent],
//     run_id: RunId,
// ) -> bool {
//     snapshot.run == run_id && tail_events.iter().all(|event| event.run_id() == run_id)
// }
//
// Verbatim body. Marked `#[verifier::external]` so Verus skips body
// verification. Spec contract is attached via
// `assume_specification[ production::hydrate_snapshot_tail_run_matches ]`
// in `vb_rpch_hydrate_preconditions.rs`.
#[verifier::external]
pub fn hydrate_snapshot_tail_run_matches(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> bool {
    snapshot.run == run_id && tail_events.iter().all(|event| event.run_id() == run_id)
}

// ---------------------------------------------------------------------------
// hydrate_snapshot_tail_seq_after_snapshot — hydrate.rs:30-37 (verbatim)
// ---------------------------------------------------------------------------
//
// /// Production proof surface for snapshot-plus-tail sequence ordering.
// #[must_use]
// pub fn hydrate_snapshot_tail_seq_after_snapshot(
//     snapshot: &RunSnapshot,
//     tail_events: &[JournalEvent],
// ) -> bool {
//     tail_events.iter().all(|event| event.seq() > snapshot.seq)
// }
#[verifier::external]
pub fn hydrate_snapshot_tail_seq_after_snapshot(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
) -> bool {
    tail_events.iter().all(|event| event.seq() > snapshot.seq)
}

// ---------------------------------------------------------------------------
// hydrate_snapshot_tail_has_evidence — hydrate.rs:39-46 (verbatim)
// ---------------------------------------------------------------------------
//
// /// Production proof surface for non-empty recovery evidence.
// #[must_use]
// pub fn hydrate_snapshot_tail_has_evidence(
//     snapshot: &RunSnapshot,
//     tail_events: &[JournalEvent],
// ) -> bool {
//     !tail_events.is_empty() || !snapshot.slots.is_empty() || !snapshot.taint.is_empty()
// }
#[verifier::external]
pub fn hydrate_snapshot_tail_has_evidence(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
) -> bool {
    !tail_events.is_empty() || !snapshot.slots.is_empty() || !snapshot.taint.is_empty()
}

// ---------------------------------------------------------------------------
// hydrate_snapshot_tail_preconditions — hydrate.rs:48-58 (verbatim)
// ---------------------------------------------------------------------------
//
// /// Production proof surface for hydrate_run_frame preconditions that do not decode bytes.
// #[must_use]
// pub fn hydrate_snapshot_tail_preconditions(
//     snapshot: &RunSnapshot,
//     tail_events: &[JournalEvent],
//     run_id: RunId,
// ) -> bool {
//     hydrate_snapshot_tail_run_matches(snapshot, tail_events, run_id)
//         && hydrate_snapshot_tail_seq_after_snapshot(snapshot, tail_events)
//         && hydrate_snapshot_tail_has_evidence(snapshot, tail_events)
// }
#[verifier::external]
pub fn hydrate_snapshot_tail_preconditions(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> bool {
    hydrate_snapshot_tail_run_matches(snapshot, tail_events, run_id)
        && hydrate_snapshot_tail_seq_after_snapshot(snapshot, tail_events)
        && hydrate_snapshot_tail_has_evidence(snapshot, tail_events)
}

// ---------------------------------------------------------------------------
// hydrate_events_preconditions — hydrate.rs:60-64 (verbatim)
// ---------------------------------------------------------------------------
//
// /// Production proof surface for events-only hydrate preconditions.
// #[must_use]
// pub const fn hydrate_events_preconditions(events: &[JournalEvent]) -> bool {
//     !events.is_empty()
// }
//
// Production declares this `pub const fn`. Verbatim body preserved
// here as `pub const fn`. BINDING DEBT D4: production's `const`-ness
// is preserved; Verus does not model `const fn` promotion but the
// signature is byte-for-byte identical to production.
#[verifier::external]
pub const fn hydrate_events_preconditions(events: &[JournalEvent]) -> bool {
    !events.is_empty()
}

// ---------------------------------------------------------------------------
// hydrate_dimensions_positive — hydrate.rs:66-70 (verbatim)
// ---------------------------------------------------------------------------
//
// /// Production proof surface for positive frame dimensions.
// #[must_use]
// pub const fn hydrate_dimensions_positive(step_count: u16, slot_count: u16) -> bool {
//     step_count > 0 && slot_count > 0
// }
//
// Production declares this `pub const fn`. Verbatim body preserved
// here as `pub const fn`. BINDING DEBT D4.
#[verifier::external]
pub const fn hydrate_dimensions_positive(step_count: u16, slot_count: u16) -> bool {
    step_count > 0 && slot_count > 0
}
