use vb_core::ids::ActionId;

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

/// Resolves the relative timeout carried by the awaiting node.
///
/// `WaitUntil`, `WaitEvent`, and `Ask` all read one numeric timeout slot.
/// The value is milliseconds from the suspension boundary, the same unit as
/// the runtime's other typed times (`timeout_ms`, grant time). The engine
/// validates the slot before suspension, so a missing or non-numeric slot can
/// only come from a hand-built node; it resolves to an immediately due timer
/// rather than panicking.
///
/// Shared with timer recovery: a recomputed deadline is conservative (the full
/// timeout from the recovery point) because wall-clock instants are not
/// durable, but it preserves live timeout semantics for resumed timers.
pub(crate) fn awaiting_deadline(
    state: &RunState,
    step: StepIdx,
) -> RuntimeResult<std::time::Instant> {
    let now = std::time::Instant::now();
    let Some(slot) = awaiting_timeout_slot(state, step) else {
        return Ok(now);
    };
    let Ok(value) = state.frame.read_slot(slot) else {
        return Ok(now);
    };
    crate::primitives::wait_ask::checked_timer_deadline(*value).map_err(Into::into)
}

fn awaiting_timeout_slot(state: &RunState, step: StepIdx) -> Option<vb_core::ids::SlotIdx> {
    let node = state.workflow.node(step)?;
    match &node.kind {
        vb_core::workflow::CompiledNodeKind::WaitUntil { deadline_slot } => Some(*deadline_slot),
        vb_core::workflow::CompiledNodeKind::WaitEvent {
            timeout_slot: vb_core::Deadline::Slot(slot),
            ..
        }
        | vb_core::workflow::CompiledNodeKind::Ask {
            timeout_slot: vb_core::Deadline::Slot(slot),
            ..
        } => Some(*slot),
        _ => None,
    }
}
