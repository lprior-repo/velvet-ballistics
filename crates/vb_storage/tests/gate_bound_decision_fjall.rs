#![forbid(unsafe_code)]

use vb_core::ids::{ActionId, RunId};
use vb_core::manual_gate::{
    ApproverId, ApproverRole, BoundGateOutcomeRecord, BoundManualGate, GateBinding, GateDecision,
    GateDigest, GateId, GateOutcome, GateOutcomeRecord, ManualGate, OperatorTrust, RiskClass,
    SeparationOfDuties,
};
use vb_storage::{EventSeq, FjallJournal, JournalEvent, RecordKind};

fn gate(run: RunId) -> Result<BoundManualGate, Box<dyn std::error::Error>> {
    let digest = |byte| GateDigest::from_bytes([byte; 32]);
    Ok(BoundManualGate {
        gate: ManualGate {
            id: GateId::new(7),
            run,
            action: ActionId::new(9),
            risk: RiskClass::High,
            compensatable: false,
            required_role: ApproverRole::new("deploy-approver".to_string())
                .ok_or("invalid approver role")?,
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

fn denial(gate: &BoundManualGate) -> Result<BoundGateOutcomeRecord, Box<dyn std::error::Error>> {
    Ok(BoundGateOutcomeRecord {
        record: GateOutcomeRecord {
            gate: gate.gate.id,
            run: gate.gate.run,
            action: gate.gate.action,
            decision: Some(GateDecision::Deny),
            outcome: GateOutcome::Denied,
            approver: Some(ApproverId::new("alice".to_string()).ok_or("invalid approver")?),
            approver_role: Some(
                ApproverRole::new("deploy-approver".to_string()).ok_or("invalid approver role")?,
            ),
            requester: ApproverId::new("bob".to_string()).ok_or("invalid requester")?,
            reason: Some("denied".to_string()),
            evidence_shown: true,
            evidence_bundle_digest: gate.gate.evidence_bundle_digest,
            prompt_digest: gate.gate.prompt_digest,
            escalation_policy_digest: gate.gate.escalation_policy_digest,
            delegated_to: None,
            break_glass: false,
            break_glass_justification: None,
            opened_at_ms: 100,
            decided_at_ms: 200,
            receipt: None,
        },
        binding: gate.binding,
        opening_seq: 0,
        operator_trust: Some(OperatorTrust::LocalSocketClaim),
        bound_receipt: None,
    })
}

#[test]
fn fjall_persists_and_reopens_bound_decision_kind_55() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let run = RunId::new(55);
    let bound_gate = gate(run)?;
    let events = [
        JournalEvent::GateOpenedBoundEvent {
            run,
            seq: EventSeq::new(0),
            gate: bound_gate.clone(),
            opened_at_ms: 100,
            requester: ApproverId::new("bob".to_string()).ok_or("invalid requester")?,
        },
        JournalEvent::GateDecisionBoundEvent {
            run,
            seq: EventSeq::new(1),
            record: Box::new(denial(&bound_gate)?),
        },
    ];
    assert_eq!(
        events
            .iter()
            .map(JournalEvent::record_kind)
            .collect::<Vec<_>>(),
        [RecordKind::GateOpenedBound, RecordKind::GateDecisionBound]
    );
    let journal = FjallJournal::open(temp.path(), None)?;
    for event in &events {
        journal.append_strict(event)?;
    }
    drop(journal);
    let reopened = FjallJournal::open(temp.path(), None)?;
    let persisted = reopened.events_for_run(run)?;
    assert_eq!(persisted, events);
    assert_eq!(
        persisted
            .get(1)
            .ok_or("kind 55 event missing")?
            .record_kind(),
        RecordKind::GateDecisionBound
    );
    Ok(())
}
