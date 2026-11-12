use super::*;

pub(super) fn validate_checkpoint_binding(
    run: RunId,
    event: &JournalEvent,
    checkpoint: &RunCheckpointV1,
    prefix: &RecoveryPrefixEvidence,
    recovered: &RecoveryArtifactWorkflow,
) -> RuntimeResult<()> {
    let JournalEvent::RetryTimerScheduled {
        run: event_run,
        seq,
        step,
        attempt,
        timer,
    } = event
    else {
        return Err(invalid("event shape"));
    };
    if *event_run != run {
        return Err(invalid("event run mismatch"));
    }
    let (seq, step, attempt) = (*seq, *step, *attempt);
    let ticket = timer.failed_ticket;
    let node = recovered
        .workflow
        .node(step)
        .ok_or_else(|| invalid("step missing"))?;
    let policy = node.retry.ok_or_else(|| invalid("policy missing"))?;
    let admission = checkpoint
        .admission
        .as_ref()
        .ok_or_else(|| invalid("admission missing"))?;
    crate::recovery::validate_recovered_admission_evidence(
        crate::recovery::RecoveredAdmissionEvidence {
            context: crate::recovery::RecoveredAdmissionContext {
                run,
                expected_workflow: prefix.workflow_digest,
                runtime_policy: prefix.admission.policy(),
            },
            admission,
            artifact: &recovered.artifact,
            workflow: &recovered.workflow,
        },
    )?;
    if matches!(
        admission.policy(),
        vb_core::RuntimePolicy::Strict | vb_core::RuntimePolicy::Journaled
    ) && (admission.idempotency_attested()
        != recovered
            .artifact
            .verification
            .idempotency_attested
            .as_ref()
        || admission.ir_compat_decision()
            != Some(
                crate::admission::check_artifact_ir_version(&recovered.artifact)
                    .map_err(|_| invalid("artifact compatibility decision"))?,
            ))
    {
        return Err(invalid(
            "checkpoint derived admission authority differs from artifact",
        ));
    }
    let recorded = &prefix.admission;
    if admission.artifact_digest() != recorded.artifact_digest()
        || admission.policy() != recorded.policy()
        || admission.policy_pin() != recorded.policy_pin()
        || admission.budget() != recorded.budget()
        || admission.granted_capabilities() != recorded.granted_capabilities()
        || admission.tenant() != recorded.tenant()
    {
        return Err(invalid(
            "checkpoint admission differs from durable admission",
        ));
    }
    if ticket.run != run
        || ticket.step != step
        || ticket.run_attempt != attempt
        || checkpoint.run_id != run
        || checkpoint.run_attempt.get() != attempt
        || checkpoint.retry_step != step
        || checkpoint.prior_seq.checked_add(1) != Some(seq.get())
        || checkpoint.action_attempts.get(step.as_usize()).copied() != Some(timer.next_attempt)
        || ticket.capacity != policy.max_attempts
        || timer.next_attempt > policy.max_attempts
        || timer.deadline_unix_ms.checked_sub(timer.scheduled_unix_ms) != Some(policy.delay_ms)
        || timer.generation != seq.get()
        || !matches!(node.kind, vb_core::CompiledNodeKind::Do { action, .. } if action == ticket.action)
    {
        return Err(invalid("checkpoint/failure/policy identity mismatch"));
    }
    Ok(())
}

pub(super) fn validate_failed_prefix(
    events: &[JournalEvent],
    index: usize,
    timer: &RetryTimerV1,
    workflow: &CompiledWorkflow,
    contracts: &[vb_core::action::ActionContract],
) -> RuntimeResult<()> {
    let ticket = timer.failed_ticket;
    let before = index
        .checked_sub(1)
        .and_then(|index| events.get(index))
        .ok_or_else(|| invalid("missing action failure"))?;
    if !matches!(before, JournalEvent::ActionFailedEvent { run, step, action, attempt, .. }
        if *run == ticket.run && *step == ticket.step && *action == ticket.action && *attempt == ticket.attempt)
    {
        return Err(invalid("timer is not adjacent to the exact action failure"));
    }
    let prefix = events
        .get(..index.saturating_sub(1))
        .ok_or_else(|| invalid("failure prefix"))?;
    let schedule = prefix.iter().rev().find(|event| matches!(event, JournalEvent::ActionScheduledTicket { ticket: old, .. } if old.step == ticket.step));
    let Some(JournalEvent::ActionScheduledTicket {
        ticket: old,
        input,
        output,
        action_abi_digest,
        ..
    }) = schedule
    else {
        return Err(invalid("missing failed ticket schedule"));
    };
    let node = workflow
        .node(ticket.step)
        .ok_or_else(|| invalid("action node"))?;
    let contract = contracts
        .get(usize::from(ticket.action.get()))
        .ok_or_else(|| invalid("action contract"))?;
    let digest = crate::recovery::action_contract_abi_digest(contract)?;
    if old != &ticket
        || node.output.slot() != Some(*output)
        || *action_abi_digest != digest
        || !matches!(node.kind, vb_core::CompiledNodeKind::Do { input: expected, .. } if expected == *input)
    {
        return Err(invalid("failed ticket schedule identity mismatch"));
    }
    Ok(())
}

pub(super) fn retry_action_boundary(
    tail: &[JournalEvent],
    timer: &RetryTimerV1,
    workflow: &CompiledWorkflow,
    contracts: &[vb_core::action::ActionContract],
) -> RuntimeResult<Option<(RecoveredRunBoundary, u64)>> {
    let ticket = timer.failed_ticket;
    let Some(JournalEvent::ActionScheduledTicket {
        ticket: next,
        seq,
        input,
        output,
        action_abi_digest,
        ..
    }) = tail.last()
    else {
        return Ok(None);
    };
    if next.step != ticket.step {
        return Ok(None);
    }
    if tail.len() != 3 {
        return Err(invalid("rescheduled action tail length"));
    }
    let mut charge = 0u64;
    let mut expected_seq = timer.generation;
    let mut started = false;
    let mut resolved = false;
    for event in tail {
        expected_seq = expected_seq
            .checked_add(1)
            .ok_or_else(|| invalid("tail sequence overflow"))?;
        if event.run_id() != ticket.run || event.seq().get() != expected_seq {
            return Err(invalid("tail sequence/run mismatch"));
        }
        charge = charge
            .checked_add(retry_tail_charge(event)?)
            .ok_or_else(|| invalid("tail charge overflow"))?;
        match event {
            JournalEvent::StepStarted { step, attempt, .. }
                if *step == ticket.step
                    && *attempt == ticket.run_attempt
                    && !started
                    && !resolved =>
            {
                started = true
            }
            JournalEvent::WaitResolvedEvent { step, attempt, .. }
                if *step == ticket.step
                    && *attempt == ticket.run_attempt
                    && started
                    && !resolved =>
            {
                resolved = true
            }
            JournalEvent::ActionScheduledTicket {
                ticket: scheduled, ..
            } if scheduled == next && started && resolved => {}
            _ => return Err(invalid("rescheduled action tail is inconsistent")),
        }
    }
    if !resolved
        || next.run != ticket.run
        || next.action != ticket.action
        || next.run_attempt != ticket.run_attempt
        || next.attempt != timer.next_attempt
        || next.capacity != ticket.capacity
        || next.idempotency_key != ticket.idempotency_key
        || !vb_core::action::action_ticket_has_valid_key(*next)
    {
        return Err(invalid("rescheduled action authority mismatch"));
    }
    let contract = contracts
        .get(usize::from(next.action.get()))
        .ok_or_else(|| invalid("next action contract"))?;
    let node = workflow
        .node(next.step)
        .ok_or_else(|| invalid("next action node"))?;
    if *action_abi_digest != crate::recovery::action_contract_abi_digest(contract)?
        || node.output.slot() != Some(*output)
        || !matches!(node.kind, vb_core::CompiledNodeKind::Do { input: expected, .. } if expected == *input)
    {
        return Err(invalid("rescheduled action ABI mismatch"));
    }
    Ok(Some((
        RecoveredRunBoundary::from_pending_action(
            crate::recovery::RecoveredPendingActionTicket::new(
                *seq,
                *next,
                *input,
                *output,
                *action_abi_digest,
            ),
        ),
        charge,
    )))
}

fn retry_tail_charge(event: &JournalEvent) -> RuntimeResult<u64> {
    use crate::journal::RuntimeJournalEvent as Event;
    let runtime = match event {
        JournalEvent::StepStarted {
            run, step, attempt, ..
        } => Event::StepStarted {
            run: *run,
            step: *step,
            attempt: *attempt,
        },
        JournalEvent::WaitResolvedEvent { run, step, .. } => Event::WaitResolved {
            run: *run,
            step: *step,
        },
        JournalEvent::ActionScheduledTicket {
            ticket,
            input,
            output,
            action_abi_digest,
            ..
        } => Event::ActionScheduledTicket {
            ticket: *ticket,
            input: *input,
            output: *output,
            action_abi_digest: *action_abi_digest,
        },
        _ => return Err(invalid("unsupported tail charge")),
    };
    crate::shard::Shard::journal_event_bytes(&runtime)
}
