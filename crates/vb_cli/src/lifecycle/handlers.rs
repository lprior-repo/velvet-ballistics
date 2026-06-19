#![forbid(unsafe_code)]
//! Lifecycle command handlers: cancel, resume, retry, answer.

mod answer;
mod cancel;
mod resume;
mod retry;
pub mod test_helpers;

pub use answer::answer;
pub use cancel::cancel;
pub use resume::resume;
pub use retry::retry;

use super::state::{EventSeqExt, LifecycleResult, current_state_from_journal};
use chrono::Utc;
use vb_core::errors::CoreError;
use vb_core::ids::RunId;
use vb_core::workflow::LifecycleState;
use vb_storage::{EventSeq, FjallJournal, JournalError, JournalEvent};

fn current_state(run: RunId, journal: &FjallJournal) -> LifecycleResult<LifecycleState> {
    current_state_from_journal(run, journal)
}

fn next_event_sequence(run: RunId, journal: &FjallJournal) -> LifecycleResult<EventSeq> {
    let events = journal
        .events_for_run(run)
        .map_err(|error| journal_error(run, format!("failed to read events: {error}")))?;
    Ok(events
        .last()
        .map(|event| event.seq().increment())
        .unwrap_or(EventSeq::ZERO))
}

fn append_lifecycle_event(
    journal: &FjallJournal,
    run: RunId,
    event: &JournalEvent,
) -> LifecycleResult<()> {
    journal
        .append_journaled(event)
        .map_err(|error| journal_append_error(run, error))
}

fn duplicate_error(run: RunId, command: &'static str, context: String) -> CoreError {
    CoreError::LifecycleDuplicateRequest {
        code: CoreError::LIFECYCLE_DUPLICATE_REQUEST_CODE,
        context,
        timestamp: Utc::now(),
        bead_id: Some(run),
        command: Some(command),
    }
}

fn stale_error(run: RunId, command: &'static str, context: String) -> CoreError {
    CoreError::LifecycleStaleRequest {
        code: CoreError::LIFECYCLE_STALE_REQUEST_CODE,
        context,
        timestamp: Utc::now(),
        bead_id: Some(run),
        command: Some(command),
    }
}

fn invalid_error(run: RunId, command: &'static str, context: String) -> CoreError {
    CoreError::LifecycleInvalidTransition {
        code: CoreError::LIFECYCLE_INVALID_TRANSITION_CODE,
        context,
        timestamp: Utc::now(),
        bead_id: Some(run),
        command: Some(command),
    }
}

fn journal_append_error(run: RunId, error: JournalError) -> CoreError {
    journal_error(run, error.to_string())
}

fn journal_error(run: RunId, context: String) -> CoreError {
    CoreError::JournalWriteFailure {
        code: CoreError::JOURNAL_WRITE_FAILURE_CODE,
        context,
        timestamp: Utc::now(),
        bead_id: Some(run),
    }
}
