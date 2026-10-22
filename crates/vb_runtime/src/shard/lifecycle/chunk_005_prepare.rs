use vb_core::ids::ActionId;

use crate::journal::RuntimeJournalEvent;
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

use super::types::{PendingTimerKind, RunState};

fn schedule_action_abi_digest(
    state: &RunState,
    action: ActionId,
) -> RuntimeResult<vb_core::ids::WorkflowDigest> {
    let contract = state
        .action_contracts
        .iter()
        .find(|c| c.id == action)
        .ok_or(RuntimeError::UnsupportedOperation {
            operation: "action_contract_missing",
        })?;
    crate::recovery::action_contract_abi_digest(contract)
}

fn awaiting_schedule_event(
    run: RunId,
    step: StepIdx,
    kind: PendingTimerKind,
) -> RuntimeJournalEvent {
    match kind {
        PendingTimerKind::Wait => RuntimeJournalEvent::WaitScheduled { run, step },
        PendingTimerKind::Ask => RuntimeJournalEvent::AskScheduled { run, step },
    }
}