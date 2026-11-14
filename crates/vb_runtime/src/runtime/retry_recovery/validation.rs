use super::*;

#[path = "validation/retry_tail.rs"]
mod retry_tail;
use retry_tail::{
    scheduled_retry_action, validate_rescheduled_action, validate_rescheduled_ticket,
    validate_retry_tail,
};

pub(super) fn validate_checkpoint_binding(
    run: RunId,
    event: &JournalEvent,
    checkpoint: &RunCheckpointV1,
    prefix: &RecoveryPrefixEvidence,
    recovered: &RecoveryArtifactWorkflow,
) -> RuntimeResult<()> {
    let (schedule, timer) = retry_timer_schedule(event, run)?;
    let admission = checkpoint
        .admission
        .as_ref()
        .ok_or_else(|| invalid("admission missing"))?;
    validate_checkpoint_admission(run, prefix, recovered, admission)?;
    validate_checkpoint_schedule(checkpoint, run, schedule, timer, &recovered.workflow)
}

fn retry_timer_schedule(
    event: &JournalEvent,
    run: RunId,
) -> RuntimeResult<((vb_storage::EventSeq, vb_core::StepIdx, u16), &RetryTimerV1)> {
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
    Ok(((*seq, *step, *attempt), timer))
}

fn validate_checkpoint_admission(
    run: RunId,
    prefix: &RecoveryPrefixEvidence,
    recovered: &RecoveryArtifactWorkflow,
    admission: &crate::admission::RunAdmission,
) -> RuntimeResult<()> {
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
    validate_derived_admission(admission, &recovered.artifact)?;
    validate_admission_matches_prefix(admission, &prefix.admission)
}

fn validate_derived_admission(
    admission: &crate::admission::RunAdmission,
    artifact: &vb_storage::AcceptedArtifact,
) -> RuntimeResult<()> {
    if matches!(
        admission.policy(),
        vb_core::RuntimePolicy::Strict | vb_core::RuntimePolicy::Journaled
    ) && (admission.idempotency_attested()
        != artifact.verification.idempotency_attested.as_ref()
        || admission.ir_compat_decision()
            != Some(
                crate::admission::check_artifact_ir_version(artifact)
                    .map_err(|_| invalid("artifact compatibility decision"))?,
            ))
    {
        return Err(invalid(
            "checkpoint derived admission authority differs from artifact",
        ));
    }
    Ok(())
}

fn validate_admission_matches_prefix(
    admission: &crate::admission::RunAdmission,
    recorded: &crate::admission::RunAdmission,
) -> RuntimeResult<()> {
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
    Ok(())
}

fn validate_checkpoint_schedule(
    checkpoint: &RunCheckpointV1,
    run: RunId,
    schedule: (vb_storage::EventSeq, vb_core::StepIdx, u16),
    timer: &RetryTimerV1,
    workflow: &CompiledWorkflow,
) -> RuntimeResult<()> {
    let (seq, step, attempt) = schedule;
    let ticket = timer.failed_ticket;
    let node = workflow.node(step).ok_or_else(|| invalid("step missing"))?;
    let policy = node.retry.ok_or_else(|| invalid("policy missing"))?;
    validate_checkpoint_ticket_fields(checkpoint, run, (seq, step, attempt), ticket, timer)?;
    if ticket.capacity != policy.max_attempts
        || timer.next_attempt > policy.max_attempts
        || timer.deadline_unix_ms.checked_sub(timer.scheduled_unix_ms) != Some(policy.delay_ms)
        || timer.generation != seq.get()
        || !matches!(node.kind, vb_core::CompiledNodeKind::Do { action, .. } if action == ticket.action)
    {
        return Err(invalid("checkpoint/failure/policy identity mismatch"));
    }
    Ok(())
}

fn validate_checkpoint_ticket_fields(
    checkpoint: &RunCheckpointV1,
    run: RunId,
    schedule: (vb_storage::EventSeq, vb_core::StepIdx, u16),
    ticket: vb_core::ActionTicket,
    timer: &RetryTimerV1,
) -> RuntimeResult<()> {
    let (seq, step, attempt) = schedule;
    if checkpoint.run_id != run
        || ticket.run != run
        || ticket.step != step
        || ticket.run_attempt != attempt
        || checkpoint.run_attempt.get() != attempt
        || checkpoint.retry_step != step
        || checkpoint.prior_seq.checked_add(1) != Some(seq.get())
        || checkpoint.action_attempts.get(step.as_usize()).copied() != Some(timer.next_attempt)
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
    validate_failure_adjacency(events, index, ticket)?;
    let schedule = failed_ticket_schedule(events, index, ticket)?;
    let contract = contracts
        .get(usize::from(ticket.action.get()))
        .ok_or_else(|| invalid("action contract"))?;
    validate_failed_schedule_identity(ticket, workflow, schedule, contract)
}

fn validate_failure_adjacency(
    events: &[JournalEvent],
    index: usize,
    ticket: vb_core::ActionTicket,
) -> RuntimeResult<()> {
    let before = index
        .checked_sub(1)
        .and_then(|index| events.get(index))
        .ok_or_else(|| invalid("missing action failure"))?;
    if !matches!(before, JournalEvent::ActionFailedEvent { run, step, action, attempt, .. }
        if *run == ticket.run && *step == ticket.step && *action == ticket.action && *attempt == ticket.attempt)
    {
        return Err(invalid("timer is not adjacent to the exact action failure"));
    }
    Ok(())
}

fn failed_ticket_schedule(
    events: &[JournalEvent],
    index: usize,
    ticket: vb_core::ActionTicket,
) -> RuntimeResult<&JournalEvent> {
    let prefix = events
        .get(..index.saturating_sub(1))
        .ok_or_else(|| invalid("failure prefix"))?;
    prefix.iter().rev()
        .find(|event| matches!(event, JournalEvent::ActionScheduledTicket { ticket: old, .. } if old.step == ticket.step))
        .ok_or_else(|| invalid("missing failed ticket schedule"))
}

fn validate_failed_schedule_identity(
    ticket: vb_core::ActionTicket,
    workflow: &CompiledWorkflow,
    schedule: &JournalEvent,
    contract: &vb_core::action::ActionContract,
) -> RuntimeResult<()> {
    let (old, input, output, action_abi_digest) = scheduled_failure_fields(schedule)?;
    let node = workflow
        .node(ticket.step)
        .ok_or_else(|| invalid("action node"))?;
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

fn scheduled_failure_fields(
    schedule: &JournalEvent,
) -> RuntimeResult<(
    &vb_core::ActionTicket,
    &vb_core::SlotIdx,
    &vb_core::SlotIdx,
    &vb_core::WorkflowDigest,
)> {
    match schedule {
        JournalEvent::ActionScheduledTicket {
            ticket,
            input,
            output,
            action_abi_digest,
            ..
        } => Ok((ticket, input, output, action_abi_digest)),
        _ => Err(invalid("missing failed ticket schedule")),
    }
}

pub(super) fn retry_action_boundary(
    tail: &[JournalEvent],
    timer: &RetryTimerV1,
    workflow: &CompiledWorkflow,
    contracts: &[vb_core::action::ActionContract],
) -> RuntimeResult<Option<(RecoveredRunBoundary, u64)>> {
    let ticket = timer.failed_ticket;
    let Some(action) = scheduled_retry_action(tail, ticket)? else {
        return Ok(None);
    };
    let charge = validate_retry_tail(tail, ticket, action.ticket, timer.generation)?;
    validate_rescheduled_ticket(ticket, action.ticket, timer)?;
    validate_rescheduled_action(&action, workflow, contracts)?;
    Ok(Some((
        RecoveredRunBoundary::from_pending_action(
            crate::recovery::RecoveredPendingActionTicket::new(
                action.seq,
                action.ticket,
                action.input,
                action.output,
                action.action_abi_digest,
            ),
        ),
        charge,
    )))
}
