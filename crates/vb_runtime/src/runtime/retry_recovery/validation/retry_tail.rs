use super::*;

pub(super) struct ScheduledRetryAction {
    pub(super) ticket: vb_core::ActionTicket,
    pub(super) seq: vb_storage::EventSeq,
    pub(super) input: vb_core::SlotIdx,
    pub(super) output: vb_core::SlotIdx,
    pub(super) action_abi_digest: vb_core::WorkflowDigest,
}

struct RetryTailProgress {
    charge: u64,
    expected_seq: u64,
    started: bool,
    resolved: bool,
}

pub(super) fn scheduled_retry_action(
    tail: &[JournalEvent],
    failed: vb_core::ActionTicket,
) -> RuntimeResult<Option<ScheduledRetryAction>> {
    let Some(action) = tail.last().and_then(scheduled_retry_fields) else {
        return Ok(None);
    };
    if action.ticket.step != failed.step {
        return Ok(None);
    }
    if tail.len() != 3 {
        return Err(invalid("rescheduled action tail length"));
    }
    Ok(Some(action))
}

fn scheduled_retry_fields(event: &JournalEvent) -> Option<ScheduledRetryAction> {
    let JournalEvent::ActionScheduledTicket {
        ticket,
        seq,
        input,
        output,
        action_abi_digest,
        ..
    } = event
    else {
        return None;
    };
    Some(ScheduledRetryAction {
        ticket: *ticket,
        seq: *seq,
        input: *input,
        output: *output,
        action_abi_digest: *action_abi_digest,
    })
}

pub(super) fn validate_retry_tail(
    tail: &[JournalEvent],
    failed: vb_core::ActionTicket,
    next: vb_core::ActionTicket,
    generation: u64,
) -> RuntimeResult<u64> {
    let mut progress = RetryTailProgress {
        charge: 0,
        expected_seq: generation,
        started: false,
        resolved: false,
    };
    for event in tail {
        validate_retry_tail_event(&mut progress, event, failed, next)?;
    }
    if !progress.resolved
        || next.run != failed.run
        || next.action != failed.action
        || next.run_attempt != failed.run_attempt
        || !vb_core::action::action_ticket_has_valid_key(next)
    {
        return Err(invalid("rescheduled action authority mismatch"));
    }
    Ok(progress.charge)
}

fn validate_retry_tail_event(
    progress: &mut RetryTailProgress,
    event: &JournalEvent,
    failed: vb_core::ActionTicket,
    next: vb_core::ActionTicket,
) -> RuntimeResult<()> {
    progress.expected_seq = progress
        .expected_seq
        .checked_add(1)
        .ok_or_else(|| invalid("tail sequence overflow"))?;
    if event.run_id() != failed.run || event.seq().get() != progress.expected_seq {
        return Err(invalid("tail sequence/run mismatch"));
    }
    progress.charge = progress
        .charge
        .checked_add(retry_tail_charge(event)?)
        .ok_or_else(|| invalid("tail charge overflow"))?;
    update_retry_tail_phase(progress, event, failed, next)
}

fn update_retry_tail_phase(
    progress: &mut RetryTailProgress,
    event: &JournalEvent,
    failed: vb_core::ActionTicket,
    next: vb_core::ActionTicket,
) -> RuntimeResult<()> {
    match event {
        JournalEvent::StepStarted { step, attempt, .. }
            if *step == failed.step
                && *attempt == failed.run_attempt
                && !progress.started
                && !progress.resolved =>
        {
            progress.started = true;
            Ok(())
        }
        JournalEvent::WaitResolvedEvent { step, attempt, .. }
            if *step == failed.step
                && *attempt == failed.run_attempt
                && progress.started
                && !progress.resolved =>
        {
            progress.resolved = true;
            Ok(())
        }
        JournalEvent::ActionScheduledTicket { ticket, .. }
            if *ticket == next && progress.started && progress.resolved =>
        {
            Ok(())
        }
        _ => Err(invalid("rescheduled action tail is inconsistent")),
    }
}

pub(super) fn validate_rescheduled_ticket(
    failed: vb_core::ActionTicket,
    next: vb_core::ActionTicket,
    timer: &RetryTimerV1,
) -> RuntimeResult<()> {
    if next.attempt != timer.next_attempt
        || next.capacity != failed.capacity
        || next.idempotency_key != failed.idempotency_key
    {
        return Err(invalid("rescheduled action authority mismatch"));
    }
    Ok(())
}

pub(super) fn validate_rescheduled_action(
    action: &ScheduledRetryAction,
    workflow: &CompiledWorkflow,
    contracts: &[vb_core::action::ActionContract],
) -> RuntimeResult<()> {
    let next = action.ticket;
    let contract = contracts
        .get(usize::from(next.action.get()))
        .ok_or_else(|| invalid("next action contract"))?;
    let node = workflow
        .node(next.step)
        .ok_or_else(|| invalid("next action node"))?;
    if action.action_abi_digest != crate::recovery::action_contract_abi_digest(contract)?
        || node.output.slot() != Some(action.output)
        || !matches!(node.kind, vb_core::CompiledNodeKind::Do { input: expected, .. } if expected == action.input)
    {
        return Err(invalid("rescheduled action ABI mismatch"));
    }
    Ok(())
}

fn retry_tail_charge(event: &JournalEvent) -> RuntimeResult<u64> {
    match event {
        JournalEvent::StepStarted { .. }
        | JournalEvent::WaitResolvedEvent { .. }
        | JournalEvent::ActionScheduledTicket { .. } => crate::journal::storage_event_bytes(event),
        _ => Err(invalid("unsupported tail charge")),
    }
}
