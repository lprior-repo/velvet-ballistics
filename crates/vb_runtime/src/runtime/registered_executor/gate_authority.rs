use super::{ExecutorError, RegisteredExecutionError};
use vb_core::action::ActionTicket;
use vb_core::manual_gate::{
    ApproverId, BoundApprovalReceipt, BoundGateOutcomeRecord, BoundManualGate, GateDigest,
    GateOutcome,
};
use vb_storage::JournalEvent;
#[path = "gate_authority/pending_ticket.rs"]
mod pending_ticket;
use pending_ticket::{PendingTicket, advance_pending_ticket};

type GateOpening<'a> = (&'a BoundManualGate, &'a ApproverId, u64);

pub(super) fn verify_durable_pair(
    events: &[JournalEvent],
    approval: &BoundApprovalReceipt,
    ticket: ActionTicket,
) -> Result<(), RegisteredExecutionError> {
    if unique_durable_approval(events, approval.digest)? != approval {
        return Err(ExecutorError::Unauthorized.into());
    }
    let (gate, requester, opened_at_ms) = unique_opening(events, approval)?;
    verify_opening_matches(gate, approval)?;
    let mut pending: Option<PendingTicket> = None;
    for event in events {
        advance_pending_ticket(&mut pending, event, approval.opening_seq, gate.gate.action)?;
    }
    if pending.is_none_or(|(pending, _)| pending != ticket) {
        return Err(ExecutorError::Unauthorized.into());
    }
    verify_unique_decision(events, approval, requester, opened_at_ms)
}

fn unique_opening<'a>(
    events: &'a [JournalEvent],
    approval: &BoundApprovalReceipt,
) -> Result<GateOpening<'a>, RegisteredExecutionError> {
    let mut openings = events.iter().filter_map(|event| match event {
        JournalEvent::GateOpenedBoundEvent {
            run,
            seq,
            gate,
            opened_at_ms,
            requester,
        } if *run == approval.receipt.run && seq.get() == approval.opening_seq => {
            Some((gate, requester, *opened_at_ms))
        }
        _ => None,
    });
    let opening = openings.next().ok_or(ExecutorError::Unauthorized)?;
    if openings.next().is_some() {
        return Err(ExecutorError::Unauthorized.into());
    }
    Ok(opening)
}

fn verify_opening_matches(
    gate: &BoundManualGate,
    approval: &BoundApprovalReceipt,
) -> Result<(), RegisteredExecutionError> {
    if !gate.is_structurally_valid()
        || gate.gate.action != approval.action
        || gate.gate.id != approval.receipt.gate
        || gate.binding != approval.binding
        || !crate::manual_gate::verify_bound_receipt(approval, gate).is_ok_and(|valid| valid)
    {
        return Err(ExecutorError::Unauthorized.into());
    }
    Ok(())
}

fn unique_decision<'a>(
    events: &'a [JournalEvent],
    approval: &BoundApprovalReceipt,
) -> Result<&'a BoundGateOutcomeRecord, RegisteredExecutionError> {
    let mut decisions = events.iter().filter_map(|event| match event {
        JournalEvent::GateDecisionBoundEvent { run, seq, record }
            if *run == approval.receipt.run
                && record.opening_seq == approval.opening_seq
                && seq.get() > approval.opening_seq =>
        {
            Some(record.as_ref())
        }
        _ => None,
    });
    let record = decisions.next().ok_or(ExecutorError::Unauthorized)?;
    if decisions.next().is_some() {
        return Err(ExecutorError::Unauthorized.into());
    }
    Ok(record)
}

fn verify_unique_decision(
    events: &[JournalEvent],
    approval: &BoundApprovalReceipt,
    requester: &ApproverId,
    opened_at_ms: u64,
) -> Result<(), RegisteredExecutionError> {
    let record = unique_decision(events, approval)?;
    if !record.is_structurally_valid()
        || record.bound_receipt.as_ref() != Some(approval)
        || record.record.requester != *requester
        || record.record.opened_at_ms != opened_at_ms
        || record.opening_seq != approval.opening_seq
        || record.record.decided_at_ms < opened_at_ms
        || !matches!(
            record.record.outcome,
            GateOutcome::Approved | GateOutcome::BreakGlassApproved
        )
    {
        return Err(ExecutorError::Unauthorized.into());
    }
    Ok(())
}

pub(super) fn unique_durable_approval(
    events: &[JournalEvent],
    digest: GateDigest,
) -> Result<&BoundApprovalReceipt, RegisteredExecutionError> {
    let mut approvals = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::GateDecisionBoundEvent { record, .. } => record.bound_receipt.as_ref(),
            _ => None,
        })
        .filter(|approval| approval.digest == digest);
    let approval = approvals.next().ok_or(ExecutorError::Unauthorized)?;
    if approvals.next().is_some() {
        return Err(ExecutorError::Unauthorized.into());
    }
    Ok(approval)
}
#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::{
        ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest,
        manual_gate::{
            ApproverId, ApproverRole, BoundManualGate, GateBinding, GateDecision,
            GateDecisionInput, GateDigest, GateId, ManualGate, OperatorTrust, RiskClass,
            SeparationOfDuties,
        },
    };
    use vb_storage::{EventSeq, FjallJournal};

    fn person(value: &str) -> Result<ApproverId, Box<dyn std::error::Error>> {
        ApproverId::new(value.to_owned()).ok_or_else(|| "invalid approver".into())
    }

    fn gate() -> Result<BoundManualGate, Box<dyn std::error::Error>> {
        let digest = |byte| GateDigest::from_bytes([byte; 32]);
        Ok(BoundManualGate {
            gate: ManualGate {
                id: GateId::new(7),
                run: RunId::new(3),
                action: ActionId::new(9),
                risk: RiskClass::High,
                compensatable: false,
                required_role: ApproverRole::new("deploy".to_owned()).ok_or("invalid role")?,
                separation_of_duties: SeparationOfDuties::DistinctFromRequester,
                evidence_bundle_digest: digest(1),
                prompt_digest: digest(2),
                timeout_ms: 1_000,
                escalation_policy_digest: digest(3),
                break_glass_allowed: true,
                approval_reason_required: true,
            },
            binding: GateBinding {
                artifact_digest: digest(4),
                policy_digest: digest(5),
                input_digest: digest(6),
                attempt: 1,
            },
        })
    }

    fn valid_events(
        opener_requester: ApproverId,
        opener_time: u64,
    ) -> Result<(Vec<JournalEvent>, BoundApprovalReceipt), Box<dyn std::error::Error>> {
        let gate = gate()?;
        let requester = person("requester")?;
        let decision = GateDecisionInput {
            gate: gate.gate.id,
            decision: GateDecision::Approve,
            approver: person("approver")?,
            approver_role: gate.gate.required_role.clone(),
            requester: requester.clone(),
            reason: Some("reviewed".to_owned()),
            evidence_shown: true,
            evidence_bundle_digest: gate.gate.evidence_bundle_digest,
            delegated_to: None,
            break_glass_justification: None,
            decided_at_ms: 200,
        };
        let record = crate::manual_gate::evaluate_bound_decision(
            &gate,
            100,
            1,
            &requester,
            OperatorTrust::InProcessCaller,
            &decision,
        )?;
        let approval = record.bound_receipt.clone().ok_or("receipt missing")?;
        let ticket = ActionTicket {
            run: gate.gate.run,
            step: StepIdx::new(4),
            seq: SeqNo::new(4),
            action: gate.gate.action,
            run_attempt: 1,
            attempt: 1,
            idempotency_key: 44,
            capacity: 1,
            fencing_token: 1,
        };
        Ok((
            vec![
                JournalEvent::ActionScheduledTicket {
                    run: ticket.run,
                    seq: EventSeq::new(0),
                    ticket,
                    input: SlotIdx::new(1),
                    output: SlotIdx::new(2),
                    action_abi_digest: WorkflowDigest::from_bytes([10; 32]),
                },
                JournalEvent::GateOpenedBoundEvent {
                    run: gate.gate.run,
                    seq: EventSeq::new(1),
                    gate,
                    opened_at_ms: opener_time,
                    requester: opener_requester,
                },
                JournalEvent::GateDecisionBoundEvent {
                    run: ticket.run,
                    seq: EventSeq::new(2),
                    record: Box::new(record),
                },
            ],
            approval,
        ))
    }

    #[test]
    fn strict_malformed_opening_metadata_is_unauthorized_without_journal_mutation()
    -> Result<(), Box<dyn std::error::Error>> {
        for (requester, opened_at_ms) in [(person("other")?, 100), (person("requester")?, 101)] {
            let dir = tempfile::tempdir()?;
            let journal = FjallJournal::open(dir.path(), None)?;
            let (events, approval) = valid_events(requester, opened_at_ms)?;
            for event in &events {
                journal.append_strict(event)?;
            }
            let stored = journal.events_for_run_full(RunId::new(3))?;
            let bytes = (0..3)
                .map(|seq| journal.get_event_bytes(RunId::new(3), EventSeq::new(seq)))
                .collect::<Result<Vec<_>, _>>()?;
            let ticket = match &stored[0] {
                JournalEvent::ActionScheduledTicket { ticket, .. } => *ticket,
                _ => return Err("scheduled ticket missing".into()),
            };
            assert!(matches!(
                verify_durable_pair(&stored, &approval, ticket),
                Err(RegisteredExecutionError::Executor(
                    ExecutorError::Unauthorized
                )),
            ));
            assert_eq!(journal.events_for_run_full(RunId::new(3))?, stored);
            assert_eq!(
                (0..3)
                    .map(|seq| journal.get_event_bytes(RunId::new(3), EventSeq::new(seq)))
                    .collect::<Result<Vec<_>, _>>()?,
                bytes,
            );
        }
        Ok(())
    }
}
