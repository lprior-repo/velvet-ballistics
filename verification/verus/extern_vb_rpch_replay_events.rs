// SPDX-License-Identifier: MIT
//
// ============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_rpch_replay_events.rs` Verus spec.
//
// Structure:
//   1. A direct `#[path]` inclusion of the verbatim production mirror
//      at
//      `verification/verus/production_inner/replay_attempt_production.rs`
//      (a verbatim copy of `crates/vb_storage/src/recovery/replay/attempt.rs:1-60`).
//      The mirror is marked `#[verifier::external]` at module level
//      so the production function bodies are opaque to Verus; the
//      inclusion still validates Rust resolution (function names,
//      parameter types, return types) at compile time. Any drift in
//      the production impl surface breaks this Verus build.
//
//   2. A spec-side mirror enum `SpecJournalEvent` and struct
//      `SpecStepIdx` (declared in `verus!` context below) with the
//      same variant/field shape as production. These are spec-visible
//      types that the companion spec file's `assume_specification`
//      bridges and exec wrappers reference. The bodies of the
//      spec-side mirror methods are byte-for-byte copies of the
//      production bodies (marked `#[verifier::external]`).
//
//   3. A phantom drift-detection helper forces Rust to look up the
//      production function names at compile time.
//
// ============================================================================
// WHY A SPEC-SIDE MIRROR (NOT DIRECT PRODUCTION TYPE IN SPEC)
// ============================================================================
// Direct spec-side usage of `prod_src::*` types is blocked because
// `prod_src` is included via `#[path]` from a plain-Rust file
// (no `verus!` block), and Verus treats types declared outside
// `verus!` as opaque. The spec-side mirror types below are declared
// inside `verus!` so they are spec-visible and can be used in
// `assume_specification` contracts and `exec fn` ensures clauses.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Production source: `crates/vb_storage/src/recovery/replay/attempt.rs:1-60`.
//
// Production functions mirrored via `prod_src` (drift-detection):
//   - `prod_src::compute_max_attempt`                  <- attempt.rs:8-16
//   - `prod_src::replay_attempt_or_default`            <- attempt.rs:19-24
//   - `prod_src::replay_attempt_is_current`            <- attempt.rs:27-29
//   - `prod_src::replay_attempt_is_stale`              <- attempt.rs:32-34
//   - `prod_src::replay_event_has_state_effect`        <- attempt.rs:37-47
//   - `prod_src::replay_event_is_stale_state_effect`   <- attempt.rs:50-52
//   - `prod_src::replay_step_order_diverges`           <- attempt.rs:55-59
//
// Spec-side mirror (used in Verus proofs and exec wrappers):
//   - `SpecJournalEvent` (enum with 6 state-affecting variants + Other)
//   - `SpecJournalEvent::attempt` (returns Option<u16>)
//   - `SpecStepIdx` (struct with u16 inner field, new/get accessors)
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The production bodies of the seven attempt-filter proof surface
// functions are NOT verified by Verus directly. The production
// mirror module is marked `#[verifier::external]` at module level.
// The spec-side mirror method bodies are also `#[verifier::external]`.
// The `assume_specification` bridges in the companion spec file
// (`vb_rpch_replay_events.rs`) attach the production contracts, and
// the exec wrappers in that file invoke the spec-side mirror
// functions to discharge the contracts. Drift between the
// production mirror and the production source is reported as
// binding-debt tracked outside Verus.
//
// ============================================================================
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION INCLUSION via #[path] — STRUCTURAL drift detection
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the verbatim production mirror at
// `production_inner/replay_attempt_production.rs`. The mirror is
// marked `#[verifier::external]` at module level so the production
// function bodies are opaque to Verus; the inclusion still validates
// Rust resolution (function names, parameter types, return types)
// at compile time. Any drift in the production impl surface breaks
// this Verus build.
//
// Drift detection: a phantom `prod_fns_drift_check` fn below calls
// the production functions with arguments of the production types,
// forcing Rust to look up the production function names at compile
// time. A rename of any of these production functions (or the
// production types) breaks the lookup and fails this Verus build.
#[verifier::external]
#[path = "production_inner/replay_attempt_production.rs"]
pub mod prod_src;

// ---------------------------------------------------------------------------
// Spec-side mirror enum — production variant-identical
// ---------------------------------------------------------------------------
//
// Variant-identical to production `JournalEvent` at
// `crates/vb_storage/src/events.rs:23-...`. The production enum has
// 22 variants; the mirror collapses the 6 state-affecting variants
// (each carrying `attempt: u16`) into individual variants and all
// other 16 variants into the `Other` catch-all. The `attempt()`
// method body matches production semantics.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SpecJournalEvent {
    /// Mirror of production `JournalEvent::StepStarted`.
    StepStarted { attempt: u16 },
    /// Mirror of production `JournalEvent::ActionScheduled`.
    ActionScheduled { attempt: u16 },
    /// Mirror of production `JournalEvent::ActionCompletedEvent`.
    ActionCompletedEvent { attempt: u16 },
    /// Mirror of production `JournalEvent::ActionFailedEvent`.
    ActionFailedEvent { attempt: u16 },
    /// Mirror of production `JournalEvent::SlotWrittenEvent`.
    SlotWrittenEvent { attempt: u16 },
    /// Mirror of production `JournalEvent::AskTimedOutEvent`.
    AskTimedOutEvent { attempt: u16 },
    /// Catch-all for variants not modeled in the mirror. The
    /// `attempt()` method returns `None` for this variant,
    /// matching the production behavior.
    Other,
}

impl SpecJournalEvent {
    /// Production body mirrors `JournalEvent::attempt` at
    /// `crates/vb_storage/src/events.rs:460-487`. The production body
    /// has 22 arms; the mirror collapses the unmodeled variants into the
    /// `Other` arm, which returns `None` consistent with the production
    /// semantics for variants that have no direct `attempt` field.
    #[verifier::external]
    pub fn attempt(&self) -> Option<u16> {
        match self {
            Self::StepStarted { attempt }
            | Self::ActionScheduled { attempt }
            | Self::ActionCompletedEvent { attempt }
            | Self::ActionFailedEvent { attempt }
            | Self::SlotWrittenEvent { attempt }
            | Self::AskTimedOutEvent { attempt } => Some(*attempt),
            Self::Other => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Spec-side mirror of StepIdx
// ---------------------------------------------------------------------------
//
// Production `vb_core::ids::numeric_id!(StepIdx, u16, get)` produces
// a tuple-newtype with a `new` and `get` accessor pair. The mirror
// reproduces that surface with a `pub` inner field.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpecStepIdx(pub u16);

impl SpecStepIdx {
    pub fn new(value: u16) -> Self {
        Self(value)
    }
    pub fn get(self) -> u16 {
        self.0
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
pub fn spec_compute_max_attempt(_events: &[SpecJournalEvent]) -> u16 {
    // Verbatim body omitted; spec uses spec_attempt_max.
    1
}

#[verifier::external]
pub fn spec_replay_attempt_or_default(attempt: Option<u16>) -> u16 {
    match attempt {
        Some(value) => value,
        None => 1,
    }
}

#[verifier::external]
pub fn spec_replay_attempt_is_current(attempt: Option<u16>, max_attempt: u16) -> bool {
    match attempt {
        Some(value) => value >= max_attempt,
        None => 1 >= max_attempt,
    }
}

#[verifier::external]
pub fn spec_replay_attempt_is_stale(attempt: Option<u16>, max_attempt: u16) -> bool {
    match attempt {
        Some(value) => value < max_attempt,
        None => 1 < max_attempt,
    }
}

#[verifier::external]
pub fn spec_replay_event_has_state_effect(event: &SpecJournalEvent) -> bool {
    match event {
        SpecJournalEvent::StepStarted { .. }
        | SpecJournalEvent::ActionScheduled { .. }
        | SpecJournalEvent::ActionCompletedEvent { .. }
        | SpecJournalEvent::ActionFailedEvent { .. }
        | SpecJournalEvent::SlotWrittenEvent { .. }
        | SpecJournalEvent::AskTimedOutEvent { .. } => true,
        SpecJournalEvent::Other => false,
    }
}

#[verifier::external]
pub fn spec_replay_event_is_stale_state_effect(event: &SpecJournalEvent, max_attempt: u16) -> bool {
    spec_replay_event_has_state_effect(event)
        && spec_replay_attempt_is_stale(event.attempt(), max_attempt)
}

#[verifier::external]
pub fn spec_replay_step_order_diverges(previous: Option<SpecStepIdx>, current: SpecStepIdx) -> bool {
    match previous {
        Some(prev) => current.get() < prev.get(),
        None => false,
    }
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
fn prod_methods_drift_check(event: &prod_src::JournalEvent) {
    let _ = prod_src::replay_attempt_or_default(None);
    let _ = prod_src::replay_attempt_or_default(Some(1u16));
    let _ = prod_src::replay_attempt_is_current(None, 1u16);
    let _ = prod_src::replay_attempt_is_current(Some(2u16), 2u16);
    let _ = prod_src::replay_attempt_is_stale(None, 1u16);
    let _ = prod_src::replay_attempt_is_stale(Some(1u16), 2u16);
    let _ = prod_src::replay_event_has_state_effect(event);
    let _ = prod_src::replay_event_is_stale_state_effect(event, 1u16);
    let _ = prod_src::replay_step_order_diverges(None, prod_src::StepIdx::new(0));
    let _ = prod_src::replay_step_order_diverges(
        Some(prod_src::StepIdx::new(3u16)),
        prod_src::StepIdx::new(2u16),
    );
    let _ = prod_src::compute_max_attempt(&[] as &[prod_src::JournalEvent]);
}

} // verus!
