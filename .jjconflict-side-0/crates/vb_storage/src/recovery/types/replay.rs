#![forbid(unsafe_code)]

use super::{RecoveryError, RecoveryResult};
use crate::EventSeq;
use vb_core::{ActionId, ActionTicket, RunId, SlotIdx, StepIdx, Taint, WorkflowDigest};

#[cfg(kani)]
#[derive(Debug, Clone, Default)]
struct ReplayResolutionSet(Vec<(ActionId, StepIdx)>);

#[cfg(kani)]
impl ReplayResolutionSet {
    fn insert(&mut self, value: (ActionId, StepIdx)) -> bool {
        if self.contains(&value) {
            false
        } else {
            self.0.push(value);
            true
        }
    }

    fn contains(&self, value: &(ActionId, StepIdx)) -> bool {
        self.0.iter().any(|entry| entry == value)
    }
}

/// Snapshot of a run's runtime state at a specific event sequence.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunSnapshot {
    /// Run identifier.
    pub run: RunId,
    /// Sequence number at which this snapshot was taken.
    pub seq: EventSeq,
    /// Compiled workflow digest.
    pub workflow: WorkflowDigest,
    /// Slot values at snapshot time, compact binary form.
    pub slots: Vec<u8>,
    /// Slot taint markers at snapshot time, compact binary form.
    pub taint: Vec<u8>,
}

/// Tracks which actions have been completed during recovery to prevent
/// re-execution of non-idempotent actions.
#[derive(Debug, Clone)]
pub struct ActionReplayTracker {
    scheduled_tickets: std::collections::HashMap<(ActionId, StepIdx), ActionScheduleEvidence>,
    completed: std::collections::HashSet<(ActionId, StepIdx)>,
    failed: std::collections::HashSet<(ActionId, StepIdx)>,
    completed_envelopes: std::collections::HashMap<(ActionId, StepIdx), ActionCompletionEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActionScheduleEvidence {
    ticket: ActionTicket,
    input: SlotIdx,
    output: SlotIdx,
    action_abi_digest: WorkflowDigest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActionCompletionEvidence {
    ticket: ActionTicket,
    output: SlotIdx,
    encoded_len: u32,
    taint: Taint,
    value_digest: [u8; 32],
    action_abi_digest: WorkflowDigest,
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
