#![forbid(unsafe_code)]

use super::{
    append_lifecycle_event, current_state, duplicate_error, invalid_error, next_event_sequence,
    stale_error,
};
use chrono::Utc;
use vb_core::ids::RunId;
use vb_core::workflow::{LifecycleCommand, LifecycleState, check_lifecycle_transition};
use vb_storage::{FjallJournal, JournalEvent};

pub fn retry(run: RunId, journal: &FjallJournal) -> super::LifecycleResult<()> {
    let state = current_state(run, journal)?;
    if state == LifecycleState::Active {
        return Err(duplicate_error(
            run,
            "retry",
            String::from("run already retried"),
        ));
    }
    if state.is_terminal() {
        return Err(stale_error(
            run,
            "retry",
            format!("retry not valid from {state:?} state"),
        ));
    }
    if !check_lifecycle_transition(state, LifecycleCommand::Retry) {
        return Err(invalid_error(
            run,
            "retry",
            format!("retry not valid from {state:?} state"),
        ));
    }
    let event = JournalEvent::RunRetried {
        run,
        seq: next_event_sequence(run, journal)?,
        timestamp: Utc::now(),
    };
    append_lifecycle_event(journal, run, &event)
}
