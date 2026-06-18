//! Idempotency key computation and validation.

use crate::action::model::ActionTicket;
use crate::ids::{ActionId, RunId, SeqNo};

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
