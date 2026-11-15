#![forbid(unsafe_code)]
use super::event_to_json;
use serde_json::json;
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
fn open_and_decided_gate_records_preserve_bindings_and_operator_fields() {
    let gate = gate();
    let record = approved(&gate);
    let run = gate.gate.run;
    let seq = EventSeq::new(19);
    let requester = ApproverId {
        value: "requesting operator".into(),
    };
    let cases = [
        (
            JournalEvent::GateOpenedEvent {
                run,
                seq,
                gate: gate.gate.clone(),
                opened_at_ms: 123,
                requester: requester.clone(),
            },
            json!({"type":"GateOpenedEvent","run":4,"seq":19,"gate":format!("{:?}",gate.gate),"opened_at_ms":123,"requester":format!("{requester:?}")}),
        ),
        (
            JournalEvent::GateOpenedBoundEvent {
                run,
                seq,
                gate: gate.clone(),
                opened_at_ms: 345,
                requester: requester.clone(),
            },
            json!({"type":"GateOpenedBoundEvent","run":4,"seq":19,"gate":format!("{gate:?}"),"opened_at_ms":345,"requester":format!("{requester:?}")}),
        ),
        (
            JournalEvent::GateDecisionEvent {
                run,
                seq,
                record: Box::new(record.record.clone()),
            },
            json!({"type":"GateDecisionEvent","run":4,"seq":19,"record":format!("{:?}",record.record)}),
        ),
        (
            JournalEvent::GateDecisionBoundEvent {
                run,
                seq,
                record: Box::new(record.clone()),
            },
            json!({"type":"GateDecisionBoundEvent","run":4,"seq":19,"record":format!("{record:?}")}),
        ),
    ];
    for (event, expected) in cases {
        assert_eq!(event_to_json(&event), expected);
    }
}
