#![forbid(unsafe_code)]

use super::{
    append_lifecycle_event, current_state, duplicate_error, invalid_error, next_event_sequence,
    stale_error,
};
use vb_core::ids::RunId;
use vb_core::workflow::{LifecycleCommand, LifecycleState, check_lifecycle_transition};
use vb_storage::{FjallJournal, JournalEvent};

pub fn cancel(run: RunId, journal: &FjallJournal) -> super::LifecycleResult<()> {
    let state = current_state(run, journal)?;
    if state == LifecycleState::Cancelled {
        return Err(duplicate_error(
            run,
            "cancel",
            String::from("run already cancelled"),
        ));
    }
    if state.is_terminal() {
        return Err(stale_error(
            run,
            "cancel",
            format!("run already in terminal state {state:?}"),
        ));
    }
    if !check_lifecycle_transition(state, LifecycleCommand::Cancel) {
        return Err(invalid_error(
            run,
            "cancel",
            format!("cancel not valid from {state:?} state"),
        ));
    }
    let event = JournalEvent::RunCancelled {
        run,
        seq: next_event_sequence(run, journal)?,
        attempt: 1,
        reason: None,
    };
    append_lifecycle_event(journal, run, &event)
}
