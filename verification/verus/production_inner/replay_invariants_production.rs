// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for replay invariants (POST-009, INV-003)
// ============================================================================
//
// This file is a VERBATIM copy of the production replay-invariants
// surface (verbatim function bodies; structurally identical variant
// and field names for types). It is plain Rust (no `verus!` block,
// no `vstd` import) and is included by the companion extern file
// `verification/verus/extern_vb_rpch_replay_invariants.rs` under
// module-level `#[verifier::external]`. Verus never reasons about
// the bodies — the inclusion validates Rust resolution (field names,
// discriminant sets, fn signatures) at compile time.
//
// ----------------------------------------------------------------------------
// Production source coverage
// ----------------------------------------------------------------------------
//
//   1. `compute_max_attempt`, `replay_attempt_or_default`,
//      `replay_attempt_is_current`, `replay_attempt_is_stale`,
//      `replay_event_has_state_effect`,
//      `replay_event_is_stale_state_effect`,
//      `replay_step_order_diverges` from
//      `crates/vb_storage/src/recovery/replay/attempt.rs:1-60`.
//
//   2. `recovery_dimension_count_from_index`,
//      `recovery_seed_dimensions_positive`,
//      `recovery_observed_dimension_is_positive` from
//      `crates/vb_storage/src/recovery/replay/summary/derive.rs:249-276`.
//
//   3. The `JournalEvent` enum (24 variants) from
//      `crates/vb_storage/src/events.rs:23-316`.
//
//   4. The `RecoveryFrameSeed` struct from
//      `crates/vb_storage/src/recovery/types.rs:628-649`.
//
// ----------------------------------------------------------------------------
// Substitutions relative to production source
// ----------------------------------------------------------------------------
//
//   - The `serde::{Deserialize, Serialize}`, `postcard`, `chrono`,
//     `vb_core::*`, and `thiserror` extern crates are replaced with
//     no-op local stubs. The mirror preserves every production field
//     NAME byte-for-byte; only the field TYPES are reduced to
//     primitives (u8 / u16 / u32 / u64) or local stub enums.
//
//   - `RecoveryFrameSeed` is mirrored with all 9 production field
//     names preserved; only the summary / steps / slots /
//     pending_actions / unsupported field TYPES are reduced to local
//     stubs.
//
// ----------------------------------------------------------------------------
// DRIFT POLICY
// ============================================================================
// This file MUST be regenerated whenever production attempt.rs,
// summary/derive.rs, events.rs, or recovery/types.rs changes. The
// mirror is annotated at the top of each section with the
// originating production line range so regeneration is mechanical.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ============================================================================
// Local stubs for the production `vb_core` newtypes
// ============================================================================
//
// Production `vb_core::ids::numeric_id!(X, inner_type, get)` produces
// `pub struct X(inner_type);` with a private inner field and a public
// `new(inner) -> Self` / `get(self) -> inner_type` accessor pair.
// The mirrors below reproduce that surface with a `pub` inner field
// (so the spec-side mirror can read .0 when needed) plus the
// constructor/accessor pair (so any drift in the production surface
// breaks this mirror).

#[repr(transparent)]

pub struct ActionId(pub u16);

impl ActionId {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
}

#[repr(transparent)]

pub struct StepIdx(pub u16);

impl StepIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
}

#[repr(transparent)]

pub struct SlotIdx(pub u16);

impl SlotIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
}

#[repr(transparent)]

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

pub struct EventSeq(pub u64);

impl EventSeq {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

pub struct ActionTicket {
    pub action: ActionId,
    pub step: StepIdx,
    pub attempt: u16,
}

pub struct WorkflowDigest(pub [u8; 32]);

pub enum CapabilitySet {
    Empty,
}

pub enum RuntimePolicy {
    Default,
}

pub enum ConstValue {
    Null,
}

pub enum SlotValue {
    Null,
}

pub enum Taint {
    Clean,
    Secret,
    DerivedFromSecret,
}

pub struct DateTime<U> {
    _phantom: core::marker::PhantomData<U>,
}

impl<U> DateTime<U> {
    pub fn new() -> Self {
        Self {
            _phantom: core::marker::PhantomData,
        }
    }
}

pub struct Utc;

pub enum DurableActionOutcome {
    Ready = 1,
}

// ============================================================================
// Local stubs for RecoveryError and RecoveryResult
// ============================================================================

pub enum RecoveryError {
    NonIdempotentActionBlocked {
        action: ActionId,
        step: StepIdx,
    },
    ReplayDivergence {
        step: StepIdx,
        detail: String,
    },
    FrameDimensionOverflow {
        run: RunId,
    },
    NoRecoveryData {
        run: RunId,
    },
    CompiledIrDigestMismatch {
        expected: WorkflowDigest,
        found: WorkflowDigest,
    },
    CorruptSlotTaint {
        slot: SlotIdx,
    },
}

pub type RecoveryResult<T> = Result<T, RecoveryError>;

// ============================================================================
// Local stubs for recovery-related types
// ============================================================================

pub enum UnsupportedRecoveryState {
    Supported,
}

pub struct RecoveryRuntimeSummary {
    pub steps_written: u64,
}

pub struct RecoveredStepEntry {
    pub step: StepIdx,
}

pub struct RecoveredSlotEntry {
    pub slot: SlotIdx,
}

pub struct RecoveredPendingAction {
    pub step: StepIdx,
    pub action: ActionId,
}

// ============================================================================
// VERBATIM PRODUCTION: JournalEvent enum + attempt() method
// ============================================================================
//
// Source: crates/vb_storage/src/events.rs:23-487
// Drift policy: any change to the variant set, field names, or the
// `attempt()` method body MUST be mirrored here.
//
// Note: this file is plain Rust so the `#[non_exhaustive]`,
// `serde::{Deserialize, Serialize}`, and `Vec<u8>` payload types
// used in production events.rs are preserved here. The Verus
// `#[verifier::external]` module directive in the companion extern
// file makes all enum variants opaque to Verus spec reasoning; the
// spec file attaches `#[verifier::external_type_specification]`
// bridges per type to expose the variant set to spec fns.

/// Compact binary journal event. JSONL is a projection, not this durable format.
///
/// Production equivalent: `crates/vb_storage/src/events.rs:21-316`.

#[non_exhaustive]
pub enum JournalEvent {
    RunAccepted {
        run: RunId,
        seq: EventSeq,
        workflow: WorkflowDigest,
    },
    RunAdmission {
        run: RunId,
        seq: EventSeq,
        artifact_digest: WorkflowDigest,
        granted_capabilities: CapabilitySet,
        policy: RuntimePolicy,
    },
    StepStarted {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        attempt: u16,
    },
    StepSucceeded {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        output: SlotIdx,
    },
    ActionScheduled {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        action: ActionId,
        attempt: u16,
    },
    ActionCompletedEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        action: ActionId,
        attempt: u16,
    },
    ActionScheduledTicket {
        run: RunId,
        seq: EventSeq,
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
    },
    ActionCompletedEnvelope {
        run: RunId,
        seq: EventSeq,
        ticket: ActionTicket,
        output: SlotIdx,
        outcome: DurableActionOutcome,
        value: Vec<u8>,
        encoded_len: u32,
        taint: Taint,
        value_digest: [u8; 32],
    },
    ActionFailedEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        action: ActionId,
        attempt: u16,
    },
    ActionAbandoned {
        run: RunId,
        seq: EventSeq,
        ticket: ActionTicket,
    },
    SlotWrittenEvent {
        run: RunId,
        seq: EventSeq,
        slot: SlotIdx,
        value: Option<Vec<u8>>,
        extra: Option<Vec<u8>>,
        attempt: u16,
    },
    WaitScheduledEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        attempt: u16,
    },
    AskScheduledEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        attempt: u16,
    },
    AskAnsweredEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        attempt: u16,
    },
    WaitResolvedEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        attempt: u16,
    },
    RetryScheduledEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        attempt: u16,
    },
    RunCancelled {
        run: RunId,
        seq: EventSeq,
        attempt: u16,
        reason: Option<String>,
    },
    RunKilled {
        run: RunId,
        seq: EventSeq,
        attempt: u16,
    },
    RunFinished {
        run: RunId,
        seq: EventSeq,
        result: SlotIdx,
        attempt: u16,
    },
    RunFailedEvent {
        run: RunId,
        seq: EventSeq,
        attempt: u16,
    },
    RunResumed {
        run: RunId,
        seq: EventSeq,
        timestamp: DateTime<Utc>,
    },
    RunRetried {
        run: RunId,
        seq: EventSeq,
        timestamp: DateTime<Utc>,
    },
    RunAnswered {
        run: RunId,
        seq: EventSeq,
        slot_idx: SlotIdx,
        answer: ConstValue,
        timestamp: DateTime<Utc>,
    },
    AskTimedOutEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        attempt: u16,
    },
}

impl JournalEvent {
    /// Returns the attempt number for this event.
    ///
    /// Verbatim from `crates/vb_storage/src/events.rs:460-487`.
    #[must_use]
    #[verifier::external]
    pub const fn attempt(&self) -> Option<u16> {
        match self {
            Self::ActionScheduled { attempt, .. }
            | Self::ActionCompletedEvent { attempt, .. }
            | Self::ActionFailedEvent { attempt, .. }
            | Self::SlotWrittenEvent { attempt, .. }
            | Self::WaitScheduledEvent { attempt, .. }
            | Self::AskScheduledEvent { attempt, .. }
            | Self::AskAnsweredEvent { attempt, .. }
            | Self::WaitResolvedEvent { attempt, .. }
            | Self::RetryScheduledEvent { attempt, .. }
            | Self::StepStarted { attempt, .. }
            | Self::RunCancelled { attempt, .. }
            | Self::RunKilled { attempt, .. }
            | Self::RunFinished { attempt, .. }
            | Self::RunFailedEvent { attempt, .. }
            | Self::AskTimedOutEvent { attempt, .. } => Some(*attempt),
            Self::ActionScheduledTicket { ticket, .. }
            | Self::ActionCompletedEnvelope { ticket, .. }
            | Self::ActionAbandoned { ticket, .. } => Some(ticket.attempt),
            Self::RunAccepted { .. }
            | Self::RunAdmission { .. }
            | Self::StepSucceeded { .. }
            | Self::RunResumed { .. }
            | Self::RunRetried { .. }
            | Self::RunAnswered { .. } => None,
        }
    }
}

// ============================================================================
// VERBATIM PRODUCTION: RecoveryFrameSeed struct
// ============================================================================
//
// Source: crates/vb_storage/src/recovery/types.rs:628-649
// Drift policy: any rename of `step_count` or `slot_count`, or any
// change to their types, MUST be mirrored here.

/// Minimal live-frame seed recovered from durable journal headers/events.
///
/// Production equivalent: `crates/vb_storage/src/recovery/types.rs:629-649`.

pub struct RecoveryFrameSeed {
    pub summary: RecoveryRuntimeSummary,
    pub first_step: StepIdx,
    pub step_count: u16,
    pub slot_count: u16,
    pub pc: StepIdx,
    pub steps: Vec<RecoveredStepEntry>,
    pub slots: Vec<RecoveredSlotEntry>,
    pub pending_actions: Vec<RecoveredPendingAction>,
    pub unsupported: UnsupportedRecoveryState,
}

// ============================================================================
// VERBATIM PRODUCTION: replay attempt-filter proof helpers
// ============================================================================
//
// Source: crates/vb_storage/src/recovery/replay/attempt.rs:1-60
// Drift policy: any rename, signature change, or body change in this
// range MUST be mirrored here.
//
// `#[verifier::external]` is applied per-function so the production
// bodies are opaque to Verus. The production TYPES in this file
// (`JournalEvent`, `StepIdx`, etc.) are NOT marked external so Verus
// can match on enum variants and access struct fields. This is the
// pattern used by `extern_vb_jnz9_journal_event_seq_valid.rs` (which
// declares mirror methods with `#[verifier::external]` per-fn).
//
// Visibility relaxation: production marks `compute_max_attempt` as
// `pub(crate)` (attempt.rs:7) so only the in-tree `replay::*` callers
// can invoke it. The mirror promotes it to `pub` so the spec-side
// `exec fn` wrappers in `vb_rpch_replay_invariants.rs` can call it
// through the bridge. Drift in the function NAME or SIGNATURE still
// breaks the mirror; only visibility is relaxed.

#[must_use]
#[verifier::external]
pub fn compute_max_attempt(events: &[JournalEvent]) -> u16 {
    let mut max_attempt = 1u16;
    for event in events {
        if let Some(attempt) = event.attempt().filter(|&a| a > max_attempt) {
            max_attempt = attempt;
        }
    }
    max_attempt
}

#[must_use]
#[verifier::external]
pub const fn replay_attempt_or_default(attempt: Option<u16>) -> u16 {
    match attempt {
        Some(value) => value,
        None => 1,
    }
}

#[must_use]
#[verifier::external]
pub const fn replay_attempt_is_current(attempt: Option<u16>, max_attempt: u16) -> bool {
    replay_attempt_or_default(attempt) >= max_attempt
}

#[must_use]
#[verifier::external]
pub const fn replay_attempt_is_stale(attempt: Option<u16>, max_attempt: u16) -> bool {
    replay_attempt_or_default(attempt) < max_attempt
}

#[must_use]
#[verifier::external]
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

#[must_use]
#[verifier::external]
pub fn replay_event_is_stale_state_effect(event: &JournalEvent, max_attempt: u16) -> bool {
    replay_event_has_state_effect(event) && replay_attempt_is_stale(event.attempt(), max_attempt)
}

#[must_use]
#[verifier::external]
pub const fn replay_step_order_diverges(previous: Option<StepIdx>, current: StepIdx) -> bool {
    match previous {
        Some(step) => current.get() < step.get(),
        None => false,
    }
}

// ============================================================================
// VERBATIM PRODUCTION: seed-dimension production proof surface
// ============================================================================
//
// Source: crates/vb_storage/src/recovery/replay/summary/derive.rs:249-276
// Drift policy: any rename, signature change, or body change in this
// range MUST be mirrored here.

#[verifier::external]
pub fn recovery_dimension_count_from_index(
    max_index: Option<u16>,
    run: RunId,
) -> RecoveryResult<u16> {
    max_index
        .map(|value| {
            value
                .checked_add(1)
                .ok_or(RecoveryError::FrameDimensionOverflow { run })
        })
        .map_or(Ok(0), |result| result)
}

#[must_use]
#[verifier::external]
pub const fn recovery_seed_dimensions_positive(seed: &RecoveryFrameSeed) -> bool {
    seed.step_count > 0 && seed.slot_count > 0
}

#[must_use]
#[verifier::external]
pub const fn recovery_observed_dimension_is_positive(max_index: Option<u16>, count: u16) -> bool {
    match max_index {
        Some(_) => count > 0,
        None => count == 0,
    }
}
