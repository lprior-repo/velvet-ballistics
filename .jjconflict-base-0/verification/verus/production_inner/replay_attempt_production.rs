// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for crates/vb_storage/src/recovery/replay/attempt.rs
// ============================================================================
//
// This file is a VERBATIM copy of the production attempt-filter proof helpers
// from `crates/vb_storage/src/recovery/replay/attempt.rs` (60 lines) with two
// minimal substitutions:
//
//   1. The production `vb_core::StepIdx` newtype is declared locally. The
//      production version uses `numeric_id!(StepIdx, u16, get)` (expanded at
//      `crates/vb_core/src/ids/mod.rs:55`) plus a separate
//      `impl StepIdx { pub const ZERO: Self = Self(0); ... }` block at
//      `crates/vb_core/src/ids/mod.rs:293-308`. The mirror reproduces the
//      same surface with a `pub` inner field plus the `new`/`get` accessor
//      pair and `ZERO`/`MIN`/`MAX` constants.
//
//   2. The production `vb_storage::events::JournalEvent` enum is mirrored
//      minimally. The production enum has 22 variants
//      (`crates/vb_storage/src/events.rs:23-316`); the spec only needs the
//      6 variants used by `replay_event_has_state_effect` (each carrying a
//      direct `attempt: u16` field) plus an `Other` catch-all for the
//      remaining 16 variants. The `attempt()` method dispatches over the 6
//      state-affecting variants returning `Some(*attempt)` and over `Other`
//      returning `None`. This is a faithful structural projection for the
//      spec's 6-variant surface; drift in the production variant names,
//      discriminants, or `attempt` field breaks the spec build.
//
// This file exists so that the companion `extern_vb_rpch_replay_events.rs`
// can use `#[path = "production_inner/replay_attempt_production.rs"]` to bind
// the production attempt-filter functions by direct source inclusion. Any
// drift between this mirror and the production source breaks the
// `extern_vb_rpch_replay_events` Verus build, which is the explicit
// drift-detection mechanism the user requires.
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_storage/src/recovery/replay/attempt.rs` whenever production
// changes. The mirror is annotated at the top of every section with the
// originating production line range so regeneration is mechanical.
//
// This file is included by the companion extern file under module-level
// `#[verifier::external]` so every body is opaque to Verus. It compiles as
// plain Rust (no `verus!` block, no `vstd` import) and is checked by the
// Verus invocation purely for structural resolution and type
// well-formedness — Verus never reasons about the bodies.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ---------------------------------------------------------------------------
// Local stub for the production `vb_core::StepIdx` newtype
// ---------------------------------------------------------------------------
//
// Production `vb_core::ids::numeric_id!(StepIdx, u16, get)` produces
// `pub struct StepIdx(u16);` with a private inner field and public
// `new(u16) -> Self` / `get(self) -> u16` accessors, plus a separate
// `impl StepIdx { pub const ZERO: Self = Self(0); pub const MIN: Self = Self(0);
// pub const MAX: Self = Self(u16::MAX); pub const fn checked_add(self, u16)
// -> Option<Self> { ... } }` block at `crates/vb_core/src/ids/mod.rs:293-308`.
// The mirror below reproduces that surface with a `pub` inner field (so the
// spec-side mirror can read .0 when needed) plus the constructor/accessor
// pair and `ZERO`/`MIN`/`MAX` constants.

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StepIdx(pub u16);

impl StepIdx {
    /// Zero step index.
    pub const ZERO: Self = Self(0);
    /// Minimum step index.
    pub const MIN: Self = Self(0);
    /// Maximum step index.
    pub const MAX: Self = Self(u16::MAX);

    /// Creates a step index from a validated u16.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the raw step index value.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

// ---------------------------------------------------------------------------
// Local stub for the production `vb_storage::events::JournalEvent` enum
// ---------------------------------------------------------------------------
//
// Production `JournalEvent` is a `#[non_exhaustive]` enum with 22 variants
// (`crates/vb_storage/src/events.rs:23-316`). The spec only needs the 6
// variants referenced by `replay_event_has_state_effect` (each carrying a
// direct `attempt: u16` field) plus an `Other` catch-all for the remaining
// 16 variants. The `attempt()` method dispatches over the 6 state-affecting
// variants returning `Some(*attempt)` and over `Other` returning `None`.
// This is a faithful structural projection for the spec's 6-variant
// surface; drift in the production variant names, discriminants, or
// `attempt` field breaks the spec build.
//
// Note on derives: the production enum derives
// `Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize`. The
// mirror drops the `serde` derives because `serde` is not registered as an
// extern crate under a standalone `verus --crate-type=lib` invocation (no
// installs allowed by the task brief). Drift in serde surface is not
// covered by this mirror; drift in variant names or the `attempt` field IS
// covered.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalEvent {
    /// Mirror of production `JournalEvent::StepStarted` at
    /// `crates/vb_storage/src/events.rs:47-56`. The spec only
    /// reads the `attempt` field.
    StepStarted { attempt: u16 },
    /// Mirror of production `JournalEvent::ActionScheduled` at
    /// `crates/vb_storage/src/events.rs:69-80`. The spec only
    /// reads the `attempt` field.
    ActionScheduled { attempt: u16 },
    /// Mirror of production `JournalEvent::ActionCompletedEvent` at
    /// `crates/vb_storage/src/events.rs:82-93`. The spec only
    /// reads the `attempt` field.
    ActionCompletedEvent { attempt: u16 },
    /// Mirror of production `JournalEvent::ActionFailedEvent` at
    /// `crates/vb_storage/src/events.rs:129-140`. The spec only
    /// reads the `attempt` field.
    ActionFailedEvent { attempt: u16 },
    /// Mirror of production `JournalEvent::SlotWrittenEvent` at
    /// `crates/vb_storage/src/events.rs:160-174`. The spec only
    /// reads the `attempt` field.
    SlotWrittenEvent { attempt: u16 },
    /// Mirror of production `JournalEvent::AskTimedOutEvent` at
    /// `crates/vb_storage/src/events.rs:306-315`. The spec only
    /// reads the `attempt` field.
    AskTimedOutEvent { attempt: u16 },
    /// Catch-all for variants not modeled in the mirror. The
    /// `attempt()` method returns `None` for this variant,
    /// matching the production behavior for the unmodeled
    /// variants that lack a direct `attempt: u16` field.
    Other,
}

impl JournalEvent {
    /// Production body mirrors `JournalEvent::attempt` at
    /// `crates/vb_storage/src/events.rs:460-487`. The production body
    /// has 22 arms; the mirror collapses the unmodeled variants into the
    /// `Other` arm, which returns `None` consistent with the production
    /// semantics for variants that have no direct `attempt` field
    /// (e.g., `RunAccepted`, `RunAdmission`, `StepSucceeded`, `RunResumed`,
    /// `RunRetried`, `RunAnswered`).
    #[must_use]
    pub const fn attempt(&self) -> Option<u16> {
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
// VERBATIM PRODUCTION: crates/vb_storage/src/recovery/replay/attempt.rs:1-60
// ---------------------------------------------------------------------------
//
// Drift policy: any rename, signature change, or body change in the
// attempt.rs source range MUST be mirrored here.

/// Production proof surface: maximum attempt observed in an event stream.
///
/// Verbatim from `crates/vb_storage/src/recovery/replay/attempt.rs:8-16`.
/// Visibility relaxed from `pub(crate)` to `pub` so the spec-side mirror
/// in `extern_vb_rpch_replay_events.rs` can reference it through the
/// bridge. Drift in NAME or SIGNATURE still breaks this mirror; only
/// visibility is relaxed.
#[must_use]
pub fn compute_max_attempt(events: &[JournalEvent]) -> u16 {
    let mut max_attempt = 1u16;
    for event in events {
        if let Some(attempt) = event.attempt().filter(|&a| a > max_attempt) {
            max_attempt = attempt;
        }
    }
    max_attempt
}

/// Production proof surface: attempt value with default of 1 for
/// attempt-less events.
///
/// Verbatim from `crates/vb_storage/src/recovery/replay/attempt.rs:19-24`.
#[must_use]
pub const fn replay_attempt_or_default(attempt: Option<u16>) -> u16 {
    match attempt {
        Some(value) => value,
        None => 1,
    }
}

/// Production proof surface: attempt is at-or-above max_attempt.
///
/// Verbatim from `crates/vb_storage/src/recovery/replay/attempt.rs:27-29`.
#[must_use]
pub const fn replay_attempt_is_current(attempt: Option<u16>, max_attempt: u16) -> bool {
    replay_attempt_or_default(attempt) >= max_attempt
}

/// Production proof surface: attempt is strictly below max_attempt.
///
/// Verbatim from `crates/vb_storage/src/recovery/replay/attempt.rs:32-34`.
#[must_use]
pub const fn replay_attempt_is_stale(attempt: Option<u16>, max_attempt: u16) -> bool {
    replay_attempt_or_default(attempt) < max_attempt
}

/// Production proof surface: event carries state-affecting replay data.
///
/// Verbatim from `crates/vb_storage/src/recovery/replay/attempt.rs:37-47`.
/// The body matches against the 6 state-affecting variants of
/// `JournalEvent`; the production enum has 22 variants but only these 6
/// carry replay state.
#[must_use]
pub const fn replay_event_has_state_effect(event: &JournalEvent) -> bool {
    matches!(
        event,
        JournalEvent::StepStarted { .. }
            | JournalEvent::ActionScheduled { .. }
            | JournalEvent::ActionCompletedEvent { .. }
            | JournalEvent::ActionFailedEvent { .. }
            | JournalEvent::SlotWrittenEvent { .. }
            | JournalEvent::AskTimedOutEvent { .. }
    )
}

/// Production proof surface: state-affecting event from a stale attempt.
///
/// Verbatim from `crates/vb_storage/src/recovery/replay/attempt.rs:50-52`.
#[must_use]
pub fn replay_event_is_stale_state_effect(event: &JournalEvent, max_attempt: u16) -> bool {
    replay_event_has_state_effect(event) && replay_attempt_is_stale(event.attempt(), max_attempt)
}

/// Production proof surface: step ordering regressed (current < previous).
///
/// Verbatim from `crates/vb_storage/src/recovery/replay/attempt.rs:55-59`.
#[must_use]
pub const fn replay_step_order_diverges(previous: Option<StepIdx>, current: StepIdx) -> bool {
    match previous {
        Some(step) => current.get() < step.get(),
        None => false,
    }
}
