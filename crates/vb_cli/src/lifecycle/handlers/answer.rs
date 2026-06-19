#![forbid(unsafe_code)]

use super::{
    append_lifecycle_event, current_state, duplicate_error, invalid_error, next_event_sequence,
    stale_error,
};
use chrono::Utc;
use vb_core::ids::{RunId, SlotIdx, SymbolId};
use vb_core::value::ConstValue;
use vb_core::workflow::{LifecycleCommand, LifecycleState, check_lifecycle_transition};
use vb_storage::{FjallJournal, JournalEvent};

pub fn answer(run: RunId, answer: String, journal: &FjallJournal) -> super::LifecycleResult<()> {
    let state = current_state(run, journal)?;
    reject_answer_duplicate_or_stale(run, state)?;
    if !check_lifecycle_transition(state, LifecycleCommand::Answer) {
        return Err(invalid_error(
            run,
            "answer",
            format!("answer not valid from {state:?} state"),
        ));
    }
    let event = JournalEvent::RunAnswered {
        run,
        seq: next_event_sequence(run, journal)?,
        slot_idx: SlotIdx::new(0),
        answer: ConstValue::Symbol(answer_symbol(&answer)),
        timestamp: Utc::now(),
    };
    append_lifecycle_event(journal, run, &event)
}

fn reject_answer_duplicate_or_stale(
    run: RunId,
    state: LifecycleState,
) -> super::LifecycleResult<()> {
    match state {
        LifecycleState::Completed => Err(duplicate_error(
            run,
            "answer",
            String::from("run already answered"),
        )),
        LifecycleState::WaitingAnswer => Ok(()),
        LifecycleState::Pending => Err(invalid_error(run, "answer", answer_context(state))),
        _ => Err(stale_error(run, "answer", answer_context(state))),
    }
}

fn answer_context(state: LifecycleState) -> String {
    format!("answer not valid from {state:?} state")
}

fn answer_symbol(answer: &str) -> SymbolId {
    let value = answer.bytes().fold(0_u32, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(u32::from(byte))
    });
    SymbolId::new(value)
}
