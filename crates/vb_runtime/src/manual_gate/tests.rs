use super::*;
use vb_core::ids::{ActionId, RunId};
use vb_core::manual_gate::{
    ApproverId, ApproverRole, BoundManualGate, GateBinding, GateDecision, GateDecisionInput,
    GateDigest, GateId, GateOutcome, ManualGate, OperatorTrust, RiskClass, SeparationOfDuties,
};

fn digest(byte: u8) -> GateDigest {
    GateDigest::from_bytes([byte; 32])
}

fn role() -> ApproverRole {
    ApproverRole::new("deploy-approver".to_string()).unwrap()
}

fn person(value: &str) -> ApproverId {
    ApproverId::new(value.to_string()).unwrap()
}

fn gate() -> ManualGate {
    ManualGate {
        id: GateId::new(7),
        run: RunId::new(3),
        action: ActionId::new(9),
        risk: RiskClass::High,
        compensatable: false,
        required_role: role(),
        separation_of_duties: SeparationOfDuties::DistinctFromRequester,
        evidence_bundle_digest: digest(1),
        prompt_digest: digest(2),
        timeout_ms: 1_000,
        escalation_policy_digest: digest(3),
        break_glass_allowed: true,
        approval_reason_required: true,
    }
}

fn bound_gate() -> BoundManualGate {
    BoundManualGate {
        gate: gate(),
        binding: GateBinding {
            artifact_digest: digest(4),
            policy_digest: digest(5),
            input_digest: digest(6),
            attempt: 1,
        },
    }
}

fn input(decision: GateDecision) -> GateDecisionInput {
    GateDecisionInput {
        gate: GateId::new(7),
        decision,
        approver: person("alice"),
        approver_role: role(),
        requester: person("bob"),
        reason: Some("checked the rollout plan".to_string()),
        evidence_shown: true,
        evidence_bundle_digest: digest(1),
        delegated_to: None,
        break_glass_justification: None,
        decided_at_ms: 500,
    }
}

mod legacy_decisions {
    use super::*;
    include!("tests_legacy_decisions.rs");
}

mod legacy_replay {
    use super::*;
    include!("tests_legacy_replay.rs");
}

mod legacy_recovery {
    use super::*;
    include!("tests_legacy_recovery.rs");
}

mod bound_receipts {
    use super::*;
    include!("tests_bound_receipts.rs");
}

mod bound_replay {
    use super::*;
    include!("tests_bound_replay.rs");
}
