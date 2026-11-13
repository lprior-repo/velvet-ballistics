use super::super::Runtime;
use super::RegisteredExecutionError;
use crate::shard::lifecycle::RegisteredActionContext;
use vb_core::manual_gate::{BoundApprovalReceipt, GateBinding, GateDigest};
use vb_core::{ActionTicket, ScopedDispatch, ScopedGrant, ScopedGrantAuthority};
use vb_storage::executor::ExecutorError;
use vb_storage::{JournalEvent, RunInputBinding, RunPolicyPin};

pub(super) fn verify_live_dispatch(
    runtime: &Runtime,
    presented: ScopedDispatch<'_>,
    context: &RegisteredActionContext,
    ticket: ActionTicket,
) -> Result<(), RegisteredExecutionError> {
    let events = run_events(runtime, ticket.run)?;
    match (presented.scoped.authority(), presented.gate_approval) {
        (ScopedGrantAuthority::TrustedNonGate, None) => {
            reject_gated_action(&events, ticket)?;
            Ok(())
        }
        (ScopedGrantAuthority::BoundGate { receipt_digest }, Some(approval)) => {
            verify_bound_dispatch(
                &events,
                approval,
                receipt_digest,
                presented.scoped,
                context,
                ticket,
            )
        }
        _ => Err(ExecutorError::Unauthorized.into()),
    }
}

pub(super) fn verify_reconcile(
    runtime: &Runtime,
    scoped: &ScopedGrant,
    context: &RegisteredActionContext,
    ticket: ActionTicket,
) -> Result<(), RegisteredExecutionError> {
    let events = run_events(runtime, ticket.run)?;
    match scoped.authority() {
        ScopedGrantAuthority::TrustedNonGate => reject_gated_action(&events, ticket),
        ScopedGrantAuthority::BoundGate { receipt_digest } => {
            let approval = super::gate_authority::unique_durable_approval(&events, receipt_digest)?;
            verify_bound_dispatch(&events, approval, receipt_digest, scoped, context, ticket)
        }
        ScopedGrantAuthority::Untrusted => Err(ExecutorError::Unauthorized.into()),
    }
}

fn run_events(
    runtime: &Runtime,
    run: vb_core::RunId,
) -> Result<Vec<JournalEvent>, RegisteredExecutionError> {
    runtime
        .journal
        .storage_journal()
        .ok_or(ExecutorError::Unauthorized)?
        .events_for_run_full(run)
        .map_err(|_| ExecutorError::Unauthorized.into())
}

fn reject_gated_action(
    events: &[JournalEvent],
    ticket: ActionTicket,
) -> Result<(), RegisteredExecutionError> {
    if events
        .iter()
        .any(|event| gate_targets_action(event, ticket))
    {
        return Err(ExecutorError::Unauthorized.into());
    }
    Ok(())
}

fn gate_targets_action(event: &JournalEvent, ticket: ActionTicket) -> bool {
    match event {
        JournalEvent::GateOpenedEvent { gate, .. } => {
            gate.run == ticket.run && gate.action == ticket.action
        }
        JournalEvent::GateDecisionEvent { record, .. } => {
            record.run == ticket.run && record.action == ticket.action
        }
        JournalEvent::GateOpenedBoundEvent { gate, .. } => {
            gate.gate.run == ticket.run && gate.gate.action == ticket.action
        }
        JournalEvent::GateDecisionBoundEvent { record, .. } => {
            record.record.run == ticket.run && record.record.action == ticket.action
        }
        _ => false,
    }
}

fn verify_bound_dispatch(
    events: &[JournalEvent],
    approval: &BoundApprovalReceipt,
    receipt_digest: GateDigest,
    scoped: &ScopedGrant,
    context: &RegisteredActionContext,
    ticket: ActionTicket,
) -> Result<(), RegisteredExecutionError> {
    verify_receipt_fields(approval, receipt_digest, scoped, ticket)?;
    verify_live_binding(events, approval, context, ticket)?;
    super::gate_authority::verify_durable_pair(events, approval, ticket)?;
    Ok(())
}

fn verify_receipt_fields(
    approval: &BoundApprovalReceipt,
    receipt_digest: GateDigest,
    scoped: &ScopedGrant,
    ticket: ActionTicket,
) -> Result<(), RegisteredExecutionError> {
    let digest_ok = crate::manual_gate::compute_bound_receipt_digest(approval)
        .is_ok_and(|digest| digest == approval.digest);
    let v1_ok = crate::manual_gate::compute_receipt_digest(
        approval.receipt.gate.get(),
        approval.receipt.run,
        approval.receipt.decision,
        &approval.receipt.approver,
        approval.receipt.evidence_bundle_digest,
        approval.receipt.prompt_digest,
        approval.receipt.decided_at_ms,
    )
    .is_ok_and(|digest| digest == approval.receipt.digest);
    if !digest_ok
        || !v1_ok
        || approval.digest != receipt_digest
        || approval.receipt.run != ticket.run
        || approval.action != ticket.action
        || approval.binding.attempt != ticket.run_attempt
        || !approval.binding.is_valid()
        || scoped.approval_receipt != approval.digest
    {
        return Err(ExecutorError::Unauthorized.into());
    }
    Ok(())
}

fn verify_live_binding(
    events: &[JournalEvent],
    approval: &BoundApprovalReceipt,
    context: &RegisteredActionContext,
    ticket: ActionTicket,
) -> Result<(), RegisteredExecutionError> {
    let (input, policy) = admission_binding(events, ticket.run)?;
    let expected_policy = match context.admission_pin {
        vb_core::policy::PinnedAdmissionPolicy::Artifact(digest) => {
            GateDigest::from_bytes(digest.as_bytes())
        }
        vb_core::policy::PinnedAdmissionPolicy::Unverified => {
            return Err(ExecutorError::Unauthorized.into());
        }
    };
    let expected = GateBinding {
        artifact_digest: GateDigest::from_bytes(context.artifact.digest.as_bytes()),
        policy_digest: expected_policy,
        input_digest: input,
        attempt: ticket.run_attempt,
    };
    if !expected.is_valid() || expected != approval.binding || policy != expected_policy {
        return Err(ExecutorError::Unauthorized.into());
    }
    Ok(())
}

fn admission_binding(
    events: &[JournalEvent],
    run: vb_core::RunId,
) -> Result<(GateDigest, GateDigest), RegisteredExecutionError> {
    let admissions = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::RunAdmission {
                run: event_run,
                input_digest,
                policy_pin,
                ..
            }
            | JournalEvent::RunAdmissionBudget {
                run: event_run,
                input_digest,
                policy_pin,
                ..
            } if *event_run == run => Some((*input_digest, *policy_pin)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if admissions.len() != 1 {
        return Err(ExecutorError::Unauthorized.into());
    }
    let (input, policy) = admissions
        .first()
        .copied()
        .ok_or(ExecutorError::Unauthorized)?;
    let input = match input {
        RunInputBinding::Digest(digest) => GateDigest::from_bytes(digest.as_bytes()),
        RunInputBinding::LegacyUnbound => return Err(ExecutorError::Unauthorized.into()),
    };
    let policy = match policy {
        RunPolicyPin::Pinned(digest) => GateDigest::from_bytes(digest.as_bytes()),
        RunPolicyPin::Unverified => return Err(ExecutorError::Unauthorized.into()),
    };
    Ok((input, policy))
}
