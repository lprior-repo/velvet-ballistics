#![forbid(unsafe_code)]
//! Replay sequence validation helpers.

use crate::{error::JournalError, events::JournalEvent, types::EventSeq};

pub(crate) fn next_seq(seq: EventSeq) -> Result<EventSeq, JournalError> {
    seq.get()
        .checked_add(1)
        .map(EventSeq::new)
        .ok_or(JournalError::SequenceOverflow)
}

pub(crate) fn validate_replayed_event(
    run: vb_core::RunId,
    expected: EventSeq,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    if event.run_id() != run {
        return Err(JournalError::WrongRun {
            expected: run,
            actual: event.run_id(),
        });
    }
    if event.seq() != expected {
        return Err(JournalError::SequenceGap {
            expected,
            actual: event.seq(),
        });
    }
    Ok(())
}
