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
    /// Current attempt number (1-indexed).
    pub attempt: u16,
    /// Idempotency key for deduplication and replay.
    pub idempotency_key: u128,
    /// Maximum attempts allowed (capacity bound from retry policy).
    pub capacity: u16,
}

/// Computes the canonical deterministic idempotency key for an action ticket.
#[must_use]
pub fn compute_action_idempotency_key(run: RunId, seq: SeqNo, action: ActionId) -> u128 {
    let run_part = u128::from(run.get());
    let seq_part = u128::from(seq.get());
    let action_part = u128::from(action.get());
    run_part
        .wrapping_mul(0x6c62272e07bb0143_u128)
        .wrapping_add(seq_part)
        .wrapping_mul(0x3b4f1a5b6c2d8e7f_u128)
        .wrapping_add(action_part)
        .wrapping_mul(0x5bd1e9956c7b4d3a_u128)
}

/// Returns true when a ticket carries the canonical key for its run/seq/action.
#[must_use]
pub fn action_ticket_has_valid_key(ticket: ActionTicket) -> bool {
    ticket.idempotency_key == compute_action_idempotency_key(ticket.run, ticket.seq, ticket.action)
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
    attempt: u16,
    idempotency_key: u128,
    capacity: u16,
) -> ActionTicket {
    ActionTicket {
        run,
        step,
        seq,
        action,
        attempt,
        idempotency_key,
        capacity,
    }
}
