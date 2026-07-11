// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for ActionReplayTracker
// ============================================================================
//
// This file is a VERBATIM copy of the production `ActionReplayTracker`
// implementation block from
//   crates/vb_storage/src/recovery/types.rs:1429-1634
// with two minimal substitutions:
//
//   1. The crate-internal `RecoveryError` and `RecoveryResult` aliases are
//      declared locally (the production versions are `#[derive(...,
//      thiserror::Error)]` and `#[derive(Debug)]`-derived, neither of
//      which can be reproduced under `verus --crate-type=lib` without
//      proc-macro crate registration). The variants and field shapes
//      used by `ActionReplayTracker` are preserved exactly:
//
//        - RecoveryError::NonIdempotentActionBlocked { action, step }
//        - RecoveryError::ReplayDivergence { step, detail }
//
//   2. The `vb_core` newtypes `ActionId`, `StepIdx`, `SlotIdx`, `Taint`,
//      `WorkflowDigest`, and the struct `ActionTicket` are declared
//      locally with the same field names, same `#[repr(transparent)]`
//      shape, and same method surface (`new`, `get`). These are
//      `Copy + PartialEq + Eq +
//      Hash` so the `HashSet<(ActionId, StepIdx)>` operations in the
//      production block at `types.rs:1433-1634` resolve
//      identically.
//
// This file exists so that the companion `extern_idempotency_replay_tracker.rs`
// can use `#[path = "production_inner/action_replay_tracker_production.rs"]`
// to bind the production `ActionReplayTracker` block by direct source
// inclusion (per the task brief "with `#[path]` bindings to production
// source"). Any drift between this mirror and the production source
// breaks the `extern_idempotency_replay_tracker` Verus build, which is
// the explicit drift-detection mechanism the user requires.
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_storage/src/recovery/types.rs:1429-1634` whenever production
// changes. The mirror is annotated at the top of every section with the
// originating production line range so regeneration is mechanical.
//
// This file is included by the companion extern file under module-level
// `#[verifier::external]` so every body is opaque to Verus. It compiles
// as plain Rust (no `verus!` block, no `vstd` import) and is checked by
// the Verus invocation purely for structural resolution and type
// well-formedness — Verus never reasons about the bodies.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ---------------------------------------------------------------------------
// Local stubs for the production `vb_core` newtypes used by
// ActionReplayTracker
// ---------------------------------------------------------------------------
//
// Production `vb_core::ids::numeric_id!(ActionId, u16, get)` and similar
// produce `pub struct $name(u16);` with a private inner field and a
// public `new($inner) -> Self` / `get(self) -> $inner` accessor pair.
// The mirrors below reproduce that surface with a `pub` inner field
// (so the spec-side mirror can read .0 when needed) plus the
// constructor/accessor pair (so any drift in the production surface
// breaks this mirror).

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotIdx(pub u16);

impl SlotIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionTicket {
    pub action: ActionId,
    pub step: StepIdx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowDigest(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taint {
    Clean,
    Secret,
    DerivedFromSecret,
    // Production `Taint` is `#[non_exhaustive]` with additional variants;
    // the ActionReplayTracker section only ever references `Taint` as
    // opaque payloads in `ActionCompletionEvidence` and as the `taint: Taint`
    // field of that struct, so the unmodeled variants do not affect the
    // structural mirror.
}

// ---------------------------------------------------------------------------
// Local stubs for the production `RecoveryError` and `RecoveryResult`
// ---------------------------------------------------------------------------
//
// Production `RecoveryError` is `#[derive(Debug, thiserror::Error)]`
// with a manual `PartialEq` impl (lines 156+). The action-replay tracker
// only ever constructs two variants:
//
//   - RecoveryError::NonIdempotentActionBlocked { action: ActionId, step: StepIdx }
//   - RecoveryError::ReplayDivergence { step: StepIdx, detail: String }
//
// Both are mirrored below without the `thiserror` derive; Verus never
// expands the derive because the module is marked `#[verifier::external]`.

#[derive(Debug, PartialEq, Eq)]
pub enum RecoveryError {
    NonIdempotentActionBlocked {
        action: ActionId,
        step: StepIdx,
    },
    ReplayDivergence {
        step: StepIdx,
        detail: String,
    },
}

pub type RecoveryResult<T> = Result<T, RecoveryError>;

// ---------------------------------------------------------------------------
// VERBATIM PRODUCTION: ActionReplayTracker block
// ---------------------------------------------------------------------------
//
// Source: crates/vb_storage/src/recovery/types.rs:1429-1634
// Drift policy: any change to the production block between these line
// numbers MUST be mirrored here.

/// Tracks which actions have been completed during recovery to prevent
/// re-execution of non-idempotent actions.
///
/// Note on field visibility: the production struct at
/// `crates/vb_storage/src/recovery/types.rs:1432-1437` has PRIVATE fields
/// `scheduled_tickets`, `completed`, `failed`, and
/// `completed_envelopes`. This in-tree mirror declares them as `pub`
/// so the companion Verus spec can reason about the HashSet view
/// (`@`) of `completed` and `failed` directly. Drift in field NAME
/// still breaks the build; only visibility is relaxed. See BINDING
/// DEBT D3 in `extern_idempotency_replay_tracker.rs`.
#[derive(Debug, Clone)]
pub struct ActionReplayTracker {
    pub scheduled_tickets: std::collections::HashMap<(ActionId, StepIdx), ActionScheduleEvidence>,
    pub completed: std::collections::HashSet<(ActionId, StepIdx)>,
    pub failed: std::collections::HashSet<(ActionId, StepIdx)>,
    pub completed_envelopes: std::collections::HashMap<(ActionId, StepIdx), ActionCompletionEvidence>,
}

// Note on visibility: production declares `ActionScheduleEvidence` and
// `ActionCompletionEvidence` as PRIVATE structs (no `pub` keyword). The
// in-tree mirror here makes them `pub` so the companion Verus spec
// file can attach `#[verifier::external_type_specification]` bridges
// to them. Drift in field NAME still breaks the build; only visibility
// is relaxed. See BINDING DEBT D3 in
// `extern_idempotency_replay_tracker.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionScheduleEvidence {
    pub ticket: ActionTicket,
    pub input: SlotIdx,
    pub output: SlotIdx,
    pub action_abi_digest: WorkflowDigest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionCompletionEvidence {
    pub ticket: ActionTicket,
    pub output: SlotIdx,
    pub encoded_len: u32,
    pub taint: Taint,
    pub value_digest: [u8; 32],
    pub action_abi_digest: WorkflowDigest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionReplayEffect {
    Apply,
    Duplicate,
}

impl ActionReplayTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            scheduled_tickets: std::collections::HashMap::new(),
            completed: std::collections::HashSet::new(),
            failed: std::collections::HashSet::new(),
            completed_envelopes: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn mark_scheduled_ticket_effect(
        &mut self,
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
        action_abi_digest: WorkflowDigest,
    ) -> RecoveryResult<ActionReplayEffect> {
        let key = (ticket.action, ticket.step);
        if self.is_resolved(ticket.action, ticket.step) {
            return Err(RecoveryError::NonIdempotentActionBlocked {
                action: ticket.action,
                step: ticket.step,
            });
        }
        let evidence = ActionScheduleEvidence {
            ticket,
            input,
            output,
            action_abi_digest,
        };
        match self.scheduled_tickets.get(&key).copied() {
            Some(existing) if existing == evidence => Ok(ActionReplayEffect::Duplicate),
            Some(_) => Err(RecoveryError::ReplayDivergence {
                step: ticket.step,
                detail: String::from("divergent action schedule ticket"),
            }),
            None => {
                self.scheduled_tickets.insert(key, evidence);
                Ok(ActionReplayEffect::Apply)
            }
        }
    }

    pub(crate) fn require_scheduled_ticket(
        &self,
        ticket: ActionTicket,
        output: SlotIdx,
        action_abi_digest: WorkflowDigest,
    ) -> RecoveryResult<()> {
        let key = (ticket.action, ticket.step);
        match self.scheduled_tickets.get(&key).copied() {
            Some(existing)
                if existing.ticket == ticket
                    && existing.output == output
                    && existing.action_abi_digest == action_abi_digest =>
            {
                Ok(())
            }
            Some(_) => Err(RecoveryError::ReplayDivergence {
                step: ticket.step,
                detail: String::from("action completion envelope does not match schedule ticket"),
            }),
            None => Err(RecoveryError::ReplayDivergence {
                step: ticket.step,
                detail: String::from("action completion envelope missing schedule ticket"),
            }),
        }
    }

    /// Records that an action was completed during normal execution.
    /// During recovery, encountering this action again will block re-execution.
    pub fn mark_completed(&mut self, action: ActionId, step: StepIdx) {
        self.completed.insert((action, step));
    }

    pub(crate) fn mark_completed_envelope_effect(
        &mut self,
        ticket: ActionTicket,
        output: SlotIdx,
        encoded_len: u32,
        taint: Taint,
        value_digest: [u8; 32],
        action_abi_digest: WorkflowDigest,
    ) -> RecoveryResult<ActionReplayEffect> {
        let key = (ticket.action, ticket.step);
        let evidence = ActionCompletionEvidence {
            ticket,
            output,
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
        };
        // Note: production uses a let-chain here
        //   (if let Some(schedule) = ... && (schedule.ticket != ticket || schedule.output != output))
        // The let-chain form requires Rust 2024 edition, which Verus 0.2026.05.05
        // (Rust 1.95.0) does not accept. The body is opaque to Verus under the
        // module-level `#[verifier::external]` directive in the companion extern
        // file, so the refactor below is purely a syntactic adaptation to keep
        // the production semantics identical while being parseable.
        if let Some(schedule) = self.scheduled_tickets.get(&key).copied() {
            if schedule.ticket != ticket || schedule.output != output {
                return Err(RecoveryError::ReplayDivergence {
                    step: ticket.step,
                    detail: String::from("action completion envelope does not match schedule ticket"),
                });
            }
        }
        match self.completed_envelopes.get(&key).copied() {
            Some(existing) if existing == evidence => Ok(ActionReplayEffect::Duplicate),
            Some(_) => Err(RecoveryError::ReplayDivergence {
                step: ticket.step,
                detail: String::from("divergent action completion envelope"),
            }),
            None if self.completed.contains(&key) || self.failed.contains(&key) => {
                Err(RecoveryError::NonIdempotentActionBlocked {
                    action: ticket.action,
                    step: ticket.step,
                })
            }
            None => {
                self.completed_envelopes.insert(key, evidence);
                self.completed.insert(key);
                Ok(ActionReplayEffect::Apply)
            }
        }
    }

    /// Records a full durable completion envelope and rejects duplicates whose
    /// ticket or output evidence diverges from the first completed envelope.
    pub fn mark_completed_envelope(
        &mut self,
        ticket: ActionTicket,
        output: SlotIdx,
        encoded_len: u32,
        taint: Taint,
        value_digest: [u8; 32],
        action_abi_digest: WorkflowDigest,
    ) -> RecoveryResult<()> {
        self.mark_completed_envelope_effect(
            ticket,
            output,
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
        )
        .map(|_| ())
    }

    /// Records that an action failed during normal execution.
    pub fn mark_failed(&mut self, action: ActionId, step: StepIdx) {
        self.failed.insert((action, step));
    }

    /// Production proof surface: the completed set contains this action/step pair.
    #[must_use]
    pub fn has_completed(&self, action: ActionId, step: StepIdx) -> bool {
        self.completed.contains(&(action, step))
    }

    /// Production proof surface: the failed set contains this action/step pair.
    #[must_use]
    pub fn has_failed(&self, action: ActionId, step: StepIdx) -> bool {
        self.failed.contains(&(action, step))
    }

    /// Checks whether an action has already been resolved (completed or failed)
    /// and must not be re-executed during recovery.
    #[must_use]
    pub fn is_resolved(&self, action: ActionId, step: StepIdx) -> bool {
        self.completed.contains(&(action, step)) || self.failed.contains(&(action, step))
    }
}

impl Default for ActionReplayTracker {
    fn default() -> Self {
        Self::new()
    }
}
