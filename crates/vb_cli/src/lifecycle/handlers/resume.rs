#![forbid(unsafe_code)]

use super::{
    append_lifecycle_event, current_state, duplicate_error, invalid_error, next_event_sequence,
    stale_error,
};
use chrono::Utc;
use vb_core::ids::RunId;
use vb_core::workflow::LifecycleState;
use vb_storage::{FjallJournal, JournalEvent};

pub fn resume(run: RunId, journal: &FjallJournal) -> super::LifecycleResult<()> {
    let state = current_state(run, journal)?;
    if state == LifecycleState::Active {
        return Err(duplicate_error(
            run,
            "resume",
            String::from("run already active"),
        ));
    }
    if is_resumable(state) {
        let event = JournalEvent::RunResumed {
            run,
            seq: next_event_sequence(run, journal)?,
            timestamp: Utc::now(),
        };
        return append_lifecycle_event(journal, run, &event);
    }
    Err(resume_rejection(run, state))
}

fn is_resumable(state: LifecycleState) -> bool {
    state == LifecycleState::Cancelled || state == LifecycleState::WaitingAnswer
}

fn resume_rejection(run: RunId, state: LifecycleState) -> vb_core::errors::CoreError {
    let context = format!("resume not valid from {state:?} state");
    if state == LifecycleState::Completed {
        stale_error(run, "resume", context)
    } else {
        invalid_error(run, "resume", context)
    }
}
