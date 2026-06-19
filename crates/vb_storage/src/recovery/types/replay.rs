#![forbid(unsafe_code)]
//! Replay tracking types for idempotent action replay during recovery.

use serde::{Deserialize, Serialize};
use vb_core::{ActionId, ActionTicket, SlotIdx, StepIdx, Taint};

use super::error::RecoveryResult;

#[cfg(all(kani, feature = "kani-vb-god2f-hard-verus"))]
#[path = "replay_kani_collections.rs"]
mod kani_replay_collections;

#[cfg(all(kani, feature = "kani-vb-god2f-hard-verus"))]
use kani_replay_collections::{KaniReplayMap, KaniReplaySet};

#[cfg(all(kani, feature = "kani-vb-god2f-hard-verus"))]
const KANI_REPLAY_BOUND: usize = 8;

#[cfg(all(kani, feature = "kani-vb-god2f-hard-verus"))]
type ReplayMap<K, V> = KaniReplayMap<K, V, KANI_REPLAY_BOUND>;

#[cfg(not(all(kani, feature = "kani-vb-god2f-hard-verus")))]
type ReplayMap<K, V> = std::collections::HashMap<K, V>;

#[cfg(all(kani, feature = "kani-vb-god2f-hard-verus"))]
type ReplaySet<T> = KaniReplaySet<T, KANI_REPLAY_BOUND>;

#[cfg(not(all(kani, feature = "kani-vb-god2f-hard-verus")))]
type ReplaySet<T> = std::collections::HashSet<T>;

#[cfg(all(kani, feature = "kani-vb-god2f-hard-verus"))]
fn new_replay_map<K: Copy + Eq, V: Copy>() -> ReplayMap<K, V> {
    ReplayMap::new()
}

#[cfg(not(all(kani, feature = "kani-vb-god2f-hard-verus")))]
fn new_replay_map<K, V>() -> ReplayMap<K, V> {
    ReplayMap::new()
}

#[cfg(all(kani, feature = "kani-vb-god2f-hard-verus"))]
fn new_replay_set<T: Copy + Eq>() -> ReplaySet<T> {
    ReplaySet::new()
}

#[cfg(not(all(kani, feature = "kani-vb-god2f-hard-verus")))]
fn new_replay_set<T>() -> ReplaySet<T> {
    ReplaySet::new()
}

/// Internal evidence for a scheduled action ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActionScheduleEvidence {
    ticket: ActionTicket,
    input: SlotIdx,
    output: SlotIdx,
}

/// Internal evidence for a completed action envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActionCompletionEvidence {
    ticket: ActionTicket,
    output: SlotIdx,
    encoded_len: u32,
    taint: Taint,
    value_digest: [u8; 32],
}

/// Outcome of applying a replay event during recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionReplayEffect {
    Apply,
    Duplicate,
}

/// Tracks which actions have been completed during recovery to prevent
/// re-execution of non-idempotent actions.
#[derive(Debug, Clone)]
pub struct ActionReplayTracker {
    scheduled_tickets: ReplayMap<(ActionId, StepIdx), ActionScheduleEvidence>,
    completed: ReplaySet<(ActionId, StepIdx)>,
    failed: ReplaySet<(ActionId, StepIdx)>,
    completed_envelopes: ReplayMap<(ActionId, StepIdx), ActionCompletionEvidence>,
}

impl ActionReplayTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            scheduled_tickets: new_replay_map(),
            completed: new_replay_set(),
            failed: new_replay_set(),
            completed_envelopes: new_replay_map(),
        }
    }

    pub(crate) fn mark_scheduled_ticket_effect(
        &mut self,
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
    ) -> RecoveryResult<ActionReplayEffect> {
        let key = (ticket.action, ticket.step);
        if self.is_resolved(ticket.action, ticket.step) {
            return Err(super::error::RecoveryError::NonIdempotentActionBlocked {
                action: ticket.action,
                step: ticket.step,
            });
        }
        let evidence = ActionScheduleEvidence {
            ticket,
            input,
            output,
        };
        match self.scheduled_tickets.get(&key).copied() {
            Some(existing) if existing == evidence => Ok(ActionReplayEffect::Duplicate),
            Some(_) => Err(super::error::RecoveryError::ReplayDivergence {
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
    ) -> RecoveryResult<()> {
        let key = (ticket.action, ticket.step);
        match self.scheduled_tickets.get(&key).copied() {
            Some(existing) if existing.ticket == ticket && existing.output == output => Ok(()),
            Some(_) => Err(super::error::RecoveryError::ReplayDivergence {
                step: ticket.step,
                detail: String::from("action completion envelope does not match schedule ticket"),
            }),
            None => Err(super::error::RecoveryError::ReplayDivergence {
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
    ) -> RecoveryResult<ActionReplayEffect> {
        let key = (ticket.action, ticket.step);
        let evidence = ActionCompletionEvidence {
            ticket,
            output,
            encoded_len,
            taint,
            value_digest,
        };
        if let Some(schedule) = self.scheduled_tickets.get(&key).copied()
            && (schedule.ticket != ticket || schedule.output != output)
        {
            return Err(super::error::RecoveryError::ReplayDivergence {
                step: ticket.step,
                detail: String::from("action completion envelope does not match schedule ticket"),
            });
        }
        match self.completed_envelopes.get(&key).copied() {
            Some(existing) if existing == evidence => Ok(ActionReplayEffect::Duplicate),
            Some(_) => Err(super::error::RecoveryError::ReplayDivergence {
                step: ticket.step,
                detail: String::from("divergent action completion envelope"),
            }),
            None if self.completed.contains(&key) || self.failed.contains(&key) => {
                Err(super::error::RecoveryError::NonIdempotentActionBlocked {
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
    ) -> RecoveryResult<()> {
        self.mark_completed_envelope_effect(ticket, output, encoded_len, taint, value_digest)
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
