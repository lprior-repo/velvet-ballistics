#![forbid(unsafe_code)]

use vb_core::action::ActionTicket;
use vb_core::engine::EngineError;
use vb_core::ids::SeqNo;

use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult};

pub(super) fn next_retry_sequence(original_ticket: &ActionTicket) -> RuntimeEngineResult<SeqNo> {
    original_ticket
        .seq
        .checked_add(1)
        .ok_or(RuntimeEngineError::Core(
            EngineError::InternalInvariantViolation {
                reason: "seq_overflow_on_retry".into(),
            },
        ))
}

pub(super) fn next_retry_attempt(original_ticket: &ActionTicket) -> RuntimeEngineResult<u16> {
    original_ticket
        .attempt
        .checked_add(1)
        .ok_or(RuntimeEngineError::Core(
            EngineError::InternalInvariantViolation {
                reason: "attempt_overflow_on_retry".into(),
            },
        ))
}
