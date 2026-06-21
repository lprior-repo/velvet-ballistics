use vb_core::action::ActionTicket;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;

use crate::shard::types::RunState;
use crate::{RuntimeError, RuntimeResult};

use super::chunk_003_completion::ActionFailureOutcome;

/// Applies the appropriate error handler for a failed action.
pub(crate) fn apply_error_handler(
    state: &mut RunState,
    ticket: ActionTicket,
) -> RuntimeResult<ActionFailureOutcome> {
    match crate::shard::helpers::find_error_handler_for_failure(&state.workflow, ticket.step) {
        Some((handler, error_slot)) => {
            state
                .frame
                .mark_failed(ticket.step)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?;
            write_failure_slot(state, ticket.step, error_slot)?;
            state
                .frame
                .set_pc(handler)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?;
            Ok(ActionFailureOutcome::DriveHandler { handler, error_slot })
        }
        None => Ok(ActionFailureOutcome::FailRun),
    }
}

fn write_failure_slot(
    state: &mut RunState,
    step: StepIdx,
    error_slot: Option<SlotIdx>,
) -> RuntimeResult<()> {
    match error_slot {
        Some(slot) => state
            .frame
            .write_slot(slot, SlotValue::I64(i64::from(step.get())))
            .map_err(|_| RuntimeError::InvalidActionCompletion),
        None => Ok(()),
    }
}
