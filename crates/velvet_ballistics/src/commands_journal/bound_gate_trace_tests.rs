use super::{TraceStatus, trace_one};
use vb_core::ids::{ActionId, RunId};
use vb_core::manual_gate::{
    ApproverId, ApproverRole, BoundGateOutcomeRecord, BoundManualGate, GateBinding, GateDecision,
    GateDigest, GateId, GateOutcome, GateOutcomeRecord, ManualGate, OperatorTrust, RiskClass,
    SeparationOfDuties,
};
use vb_storage::{EventSeq, JournalEvent};

fn gate() -> BoundManualGate {
    let digest = |byte| GateDigest::from_bytes([byte; 32]);
    BoundManualGate {
        gate: ManualGate {
            id: GateId::new(7),
            run: RunId::new(4),
            action: ActionId::new(9),
            risk: RiskClass::High,
            compensatable: false,
            required_role: ApproverRole {
                name: String::from("deploy-approver"),
            },
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
    }
}

fn approved(gate: &BoundManualGate) -> BoundGateOutcomeRecord {
    let record = GateOutcomeRecord {
        gate: gate.gate.id,
        run: gate.gate.run,
        action: gate.gate.action,
        decision: Some(GateDecision::Approve),
        outcome: GateOutcome::Approved,
        approver: Some(ApproverId {
            value: String::from("alice"),
        }),
        approver_role: Some(gate.gate.required_role.clone()),
        requester: ApproverId {
            value: String::from("bob"),
        },
        reason: Some(String::from("reviewed")),
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
    };
    BoundGateOutcomeRecord {
        record,
        binding: gate.binding,
        opening_seq: 0,
        operator_trust: Some(OperatorTrust::LocalSocketClaim),
        bound_receipt: None,
    }
}

#[test]
fn bound_gate_trace_maps_kind_54_to_waiting_answer() {
    let gate = gate();
    let event = JournalEvent::GateOpenedBoundEvent {
        run: gate.gate.run,
        seq: EventSeq::new(10),
        gate,
        opened_at_ms: 100,
        requester: ApproverId {
            value: String::from("bob"),
        },
    };
    let trace = trace_one(3, &event);
    assert_eq!(trace.event_type, "GateOpenedBound");
    assert_eq!(trace.status, Some(TraceStatus::WaitingAnswer));
    assert_eq!(trace.action, Some(9));
    assert_eq!(trace.seq, 10);
}

#[test]
fn bound_gate_trace_maps_kind_55_approval_to_completed() {
    let gate = gate();
    let event = JournalEvent::GateDecisionBoundEvent {
        run: gate.gate.run,
        seq: EventSeq::new(11),
        record: Box::new(approved(&gate)),
    };
    let trace = trace_one(4, &event);
    assert_eq!(trace.event_type, "GateDecisionBound");
    assert_eq!(trace.status, Some(TraceStatus::Completed));
    assert_eq!(trace.action, Some(9));
    assert_eq!(trace.seq, 11);
}
