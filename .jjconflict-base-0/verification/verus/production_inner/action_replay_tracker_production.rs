// SPDX-License-Identifier: MIT
// Structural copy of the real ActionReplayTracker with native dependencies.
// Source: crates/vb_storage/src/recovery/types/replay.rs:45-268
// Source: crates/vb_storage/src/recovery/retry_authority.rs:5-69
// Derive attributes and private field visibility retain the existing mirror
// substitutions. RunId and SeqNo are represented by their transparent u64
// payloads. Conversion retains every ticket field without inferred values.
// RetryTimerV1 validation and canonical key validation execute the actual
// vb_storage/vb_core implementations linked with --extern.
// Existing proof assumptions and the external module boundary are unchanged;
// the production bodies are not proved by the weak bridge.
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
    pub run: u64,
    pub seq: u64,
    pub run_attempt: u16,
    pub attempt: u16,
    pub capacity: u16,
    pub fencing_token: u64,
    pub idempotency_key: u128,
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
    NonIdempotentActionBlocked { action: ActionId, step: StepIdx },
    ReplayDivergence { step: StepIdx, detail: String },
}

pub type RecoveryResult<T> = Result<T, RecoveryError>;

// ---------------------------------------------------------------------------
// VERBATIM PRODUCTION: ActionReplayTracker block
// ---------------------------------------------------------------------------
//
// Source: crates/vb_storage/src/recovery/types/replay.rs:45-268
// Drift policy: any change to the production block between these line
// numbers MUST be mirrored here.

/// Tracks which actions have been completed during recovery to prevent
/// re-execution of non-idempotent actions.
///
/// Note on field visibility: the production struct at
/// `crates/vb_storage/src/recovery/types/replay.rs:45-51` has PRIVATE fields
/// `retry_attempts`, `scheduled_tickets`, `completed`, `failed`, and
/// `completed_envelopes`. This in-tree mirror declares them as `pub`
/// so the companion Verus spec can reason about the HashSet view
/// (`@`) of `completed` and `failed` directly. Drift in field NAME
/// still breaks the build; only visibility is relaxed. See BINDING
/// DEBT D3 in `extern_idempotency_replay_tracker.rs`.
#[derive(Debug, Clone)]
pub struct ActionReplayTracker {
    pub retry_attempts: std::collections::HashMap<(ActionId, StepIdx), u16>,
    pub scheduled_tickets: std::collections::HashMap<(ActionId, StepIdx), ActionScheduleEvidence>,
    pub completed: std::collections::HashSet<(ActionId, StepIdx)>,
    pub failed: std::collections::HashSet<(ActionId, StepIdx)>,
    pub completed_envelopes:
        std::collections::HashMap<(ActionId, StepIdx), ActionCompletionEvidence>,
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
            retry_attempts: std::collections::HashMap::new(),
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
        self.consume_retry(ticket, input, output, action_abi_digest)?;
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
        if let Some(schedule) = self.scheduled_tickets.get(&key).copied()
            && (schedule.ticket != ticket || schedule.output != output)
        {
            return Err(RecoveryError::ReplayDivergence {
                step: ticket.step,
                detail: String::from("action completion envelope does not match schedule ticket"),
            });
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

    /// Checks per-action attempt identity when a full schedule is retained.
    /// Pair-only legacy histories retain their existing resolution behavior.
    pub(crate) fn require_failed_attempt_matches_schedule(
        &self,
        action: ActionId,
        step: StepIdx,
        attempt: u16,
    ) -> RecoveryResult<()> {
        if let Some(schedule) = self.scheduled_tickets.get(&(action, step))
            && schedule.ticket.attempt != attempt
        {
            return Err(RecoveryError::ReplayDivergence {
                step,
                detail: String::from("action failure attempt does not match schedule ticket"),
            });
        }
        Ok(())
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

impl ActionReplayTracker {
    pub(crate) fn authorize_retry(&mut self, timer: &vb_storage::retry_timer::RetryTimerV1) -> RecoveryResult<()> {
        timer.validate().map_err(|_| invalid(ActionTicket::from_actual(timer.failed_ticket)))?;
        let ticket = ActionTicket::from_actual(timer.failed_ticket);
        let key = (ticket.action, ticket.step);
        if !self.failed.contains(&key)
            || self.completed.contains(&key)
            || self
                .scheduled_tickets
                .get(&key)
                .is_none_or(|old| old.ticket != ticket)
            || self
                .retry_attempts
                .insert(key, timer.next_attempt)
                .is_some()
        {
            return Err(invalid(ticket));
        }
        Ok(())
    }

    pub(crate) fn consume_retry(
        &mut self,
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
        abi: WorkflowDigest,
    ) -> RecoveryResult<()> {
        let key = (ticket.action, ticket.step);
        let Some(next) = self.retry_attempts.get(&key).copied() else {
            return Ok(());
        };
        let old = self
            .scheduled_tickets
            .get(&key)
            .ok_or_else(|| invalid(ticket))?;
        let previous = old.ticket;
        if !self.failed.contains(&key)
            || self.completed.contains(&key)
            || ticket.attempt != next
            || ticket.run != previous.run
            || ticket.run_attempt != previous.run_attempt
            || ticket.capacity != previous.capacity
            || ticket.idempotency_key != previous.idempotency_key
            || ticket.seq != previous.seq
            || old.input != input
            || old.output != output
            || old.action_abi_digest != abi
            || !vb_core::action::action_ticket_has_valid_key(ticket.into_actual())
        {
            return Err(invalid(ticket));
        }
        self.retry_attempts.remove(&key);
        self.failed.remove(&key);
        self.scheduled_tickets.remove(&key);
        Ok(())
    }
}

fn invalid(ticket: ActionTicket) -> RecoveryError {
    RecoveryError::ReplayDivergence {
        step: ticket.step,
        detail: "invalid durable retry authority".into(),
    }
}

// RunId/SeqNo are transparent u64 payload projections; every ticket field is retained.
impl ActionTicket {
    fn from_actual(ticket: vb_core::ActionTicket) -> Self {
        Self { run: ticket.run.get(), seq: ticket.seq.get(),
            action: ActionId::new(ticket.action.get()), step: StepIdx::new(ticket.step.get()),
            run_attempt: ticket.run_attempt, attempt: ticket.attempt, capacity: ticket.capacity,
            fencing_token: ticket.fencing_token, idempotency_key: ticket.idempotency_key }
    }
    fn into_actual(self) -> vb_core::ActionTicket {
        vb_core::ActionTicket { run: vb_core::RunId::new(self.run), seq: vb_core::SeqNo::new(self.seq),
            action: vb_core::ActionId::new(self.action.get()), step: vb_core::StepIdx::new(self.step.get()),
            run_attempt: self.run_attempt, attempt: self.attempt, capacity: self.capacity,
            fencing_token: self.fencing_token, idempotency_key: self.idempotency_key }
    }
}
