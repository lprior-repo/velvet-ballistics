use crate::ids::{ActionId, RunId, SeqNo, StepIdx};
use serde::{Deserialize, Serialize};

/// Unique ticket tracking one action invocation across suspension boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionTicket {
    /// Owning run.
    pub run: RunId,
    /// Step that issued the action.
    pub step: StepIdx,
    /// Monotonic sequence within the run.
    pub seq: SeqNo,
    /// Action being invoked.
    pub action: ActionId,
    /// Durable run attempt number (1-indexed). Increments on each run retry.
    pub run_attempt: u16,
    /// Current per-action attempt number (1-indexed).
    pub attempt: u16,
    /// Idempotency key for deduplication and replay.
    pub idempotency_key: u128,
    /// Maximum per-action attempts allowed (capacity bound from retry policy).
    pub capacity: u16,
}

/// Computes the canonical deterministic idempotency key for an action ticket.
/// Includes run_attempt to differentiate authority across run retries.
#[must_use]
pub fn compute_action_idempotency_key(run: RunId, run_attempt: u16, seq: SeqNo, action: ActionId) -> u128 {
    let run_part = u128::from(run.get());
    let attempt_part = u128::from(run_attempt);
    let seq_part = u128::from(seq.get());
    let action_part = u128::from(action.get());
    run_part
        .wrapping_mul(0x6c62272e07bb0143_u128)
        .wrapping_add(attempt_part)
        .wrapping_mul(0x3b4f1a5b6c2d8e7f_u128)
        .wrapping_add(seq_part)
        .wrapping_mul(0x5bd1e9956c7b4d3a_u128)
        .wrapping_add(action_part)
}

/// Returns true when a ticket carries the canonical key for its run/seq/action.
#[must_use]
pub fn action_ticket_has_valid_key(ticket: ActionTicket) -> bool {
    ticket.idempotency_key
        == compute_action_idempotency_key(ticket.run, ticket.run_attempt, ticket.seq, ticket.action)
}

/// Issues an action ticket for a Do-node suspension.
///
/// Constructs a new `ActionTicket` from the run metadata, action contract,
/// and current attempt counter. The ticket tracks this invocation across
/// suspension boundaries.
pub fn issue_action_ticket(
    run: RunId,
    step: StepIdx,
    seq: SeqNo,
    action: ActionId,
    run_attempt: u16,
    attempt: u16,
    idempotency_key: u128,
    capacity: u16,
) -> ActionTicket {
    ActionTicket {
        run,
        step,
        seq,
        action,
        run_attempt,
        attempt,
        idempotency_key,
        capacity,
    }
}