#![forbid(unsafe_code)]
//! Action envelope digest and ticket verification utilities.
//!
//! Provides:
//! - `verify_action_ticket_event`: validates ticket run, attempt bounds, idempotency key
//! - `verified_action_envelope_digest`: validates ticket, length, and blake3 digest
//!
//! These are shared replay invariants consumed by hydration, summary building,
//! and core replay logic.

use crate::DurableActionOutcome;
use crate::recovery::{RecoveryError, RecoveryResult};
use vb_core::{ActionTicket, RunId};

/// Verifies that an action ticket is consistent with the expected run context.
///
/// Checks:
/// - Ticket run matches the expected run
/// - Attempt/attempt-capacity bounds are valid
/// - Idempotency key is present and correct
pub(crate) fn verify_action_ticket_event(run: RunId, ticket: ActionTicket) -> RecoveryResult<()> {
    if ticket.run != run {
        return Err(RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: String::from("action ticket run mismatch"),
        });
    }
    if ticket.attempt == 0 || ticket.capacity == 0 || ticket.attempt > ticket.capacity {
        return Err(RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: String::from("action ticket attempt bounds invalid"),
        });
    }
    if !vb_core::action::action_ticket_has_valid_key(ticket) {
        return Err(RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: String::from("action ticket idempotency key mismatch"),
        });
    }
    Ok(())
}

/// Verifies and returns a verified action envelope digest.
///
/// Validates the ticket against the run context, checks encoded length
/// matches actual value length, and confirms the blake3 hash matches
/// the expected digest.
pub(crate) fn verified_action_envelope_digest(
    run: RunId,
    ticket: ActionTicket,
    outcome: DurableActionOutcome,
    value: &[u8],
    encoded_len: u32,
    expected: [u8; 32],
) -> RecoveryResult<[u8; 32]> {
    verify_action_ticket_event(run, ticket)?;
    match outcome {
        DurableActionOutcome::Ready => {}
    }
    let actual_len = u32::try_from(value.len()).map_err(|_| RecoveryError::ReplayDivergence {
        step: ticket.step,
        detail: String::from("action completion value length exceeds u32"),
    })?;
    if actual_len != encoded_len {
        return Err(RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: String::from("action completion encoded length mismatch"),
        });
    }

    let found = *blake3::hash(value).as_bytes();
    if found == expected {
        Ok(expected)
    } else {
        Err(RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: String::from("action completion value digest mismatch"),
        })
    }
}
