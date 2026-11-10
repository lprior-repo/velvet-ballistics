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
pub(crate) fn awaiting_deadline(state: &RunState, step: StepIdx) -> std::time::Instant {
    let now = std::time::Instant::now();
    let Some(node) = state.workflow.node(step) else {
        return now;
    };
    let slot = match &node.kind {
        vb_core::workflow::CompiledNodeKind::WaitUntil { deadline_slot } => Some(*deadline_slot),
        vb_core::workflow::CompiledNodeKind::WaitEvent {
            timeout_slot: Some(slot),
            ..
        }
        | vb_core::workflow::CompiledNodeKind::Ask {
            timeout_slot: Some(slot),
            ..
        } => Some(*slot),
        _ => None,
    };
    let Some(slot) = slot else {
        return now;
    };
    let Ok(value) = state.frame.read_slot(slot) else {
        return now;
    };
    let millis = match value {
        vb_core::value::SlotValue::I64(value) => u64::try_from((*value).max(0)).unwrap_or(0),
        vb_core::value::SlotValue::F64(value) => {
            let seconds = (*value).get() / 1000.0;
            std::time::Duration::try_from_secs_f64(seconds)
                .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
                .unwrap_or(0)
        }
        _ => 0,
    };
    now.checked_add(std::time::Duration::from_millis(millis))
        .unwrap_or(now)
}
