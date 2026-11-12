#![forbid(unsafe_code)]
//! Manual gate value types (MASTER §77.15, V1-MIN).
//!
//! Pure value types with no runtime state or execution logic. A manual gate
//! is a safety control carrying authority, evidence, and accountability:
//! rubber-stamp prompts are rejected by the evaluator in `vb_runtime`.
//!
//! The durable outcome of a gate is [`GateOutcomeRecord`]. An approval
//! receipt ([`ApprovalReceipt`]) binds a decision to its gate, run,
//! approver, evidence bundle digest, and prompt digest. A receipt carries
//! no capability data by construction: it can never broaden artifact
//! capabilities. Broadening capabilities requires a separate journaled
//! grant (a later batch consumes [`ApprovalReceipt::digest`]).

mod approver;
mod decision;
mod gate;
mod identifiers;
mod kinds;
mod receipt;
mod record;

pub use approver::{ApproverId, ApproverRole, MAX_APPROVER_TEXT_LEN};
pub use decision::GateDecisionInput;
pub use gate::ManualGate;
pub use identifiers::{GateDigest, GateId};
pub use kinds::{GateDecision, GateOutcome, RiskClass, SeparationOfDuties};
pub use receipt::ApprovalReceipt;
pub use record::GateOutcomeRecord;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ActionId, RunId};

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

    #[test]
    fn digest_round_trips_bytes() {
        let bytes = [9_u8; 32];
        assert_eq!(GateDigest::from_bytes(bytes).as_bytes(), bytes);
    }

    #[test]
    fn gate_id_round_trips_value() {
        assert_eq!(GateId::new(42).get(), 42);
    }

    #[test]
    fn deadline_checked_addition_overflows_to_none() {
        let gate = gate();
        assert_eq!(gate.deadline_ms(u64::MAX), None);
        assert_eq!(gate.deadline_ms(500), Some(1_500));
    }

    #[test]
    fn separation_applies_always_for_high_risk() {
        let mut gate = gate();
        gate.separation_of_duties = SeparationOfDuties::Disabled;
        assert!(gate.separation_applies());
        gate.risk = RiskClass::Low;
        assert!(!gate.separation_applies());
        gate.separation_of_duties = SeparationOfDuties::DistinctFromRequester;
        assert!(gate.separation_applies());
    }

    #[test]
    fn evidence_display_required_only_for_high_non_compensatable() {
        let mut gate = gate();
        assert!(gate.evidence_display_required());
        gate.compensatable = true;
        assert!(!gate.evidence_display_required());
        gate.compensatable = false;
        gate.risk = RiskClass::Medium;
        assert!(!gate.evidence_display_required());
    }

    #[test]
    fn approver_text_validation_rejects_empty_and_overlong() {
        assert!(ApproverRole::new(String::new()).is_none());
        assert!(ApproverId::new(String::new()).is_none());
        let long = "x".repeat(MAX_APPROVER_TEXT_LEN + 1);
        assert!(ApproverRole::new(long.clone()).is_none());
        assert!(ApproverId::new(long).is_none());
        assert!(role().is_valid());
        assert!(person("alice").is_valid());
    }

    #[test]
    fn receipt_bind_rejects_non_approval_decisions() {
        for decision in [
            GateDecision::Deny,
            GateDecision::RequestMoreEvidence,
            GateDecision::Delegate,
        ] {
            assert!(
                ApprovalReceipt::bind(
                    GateId::new(7),
                    RunId::new(3),
                    decision,
                    person("alice"),
                    digest(1),
                    digest(2),
                    100,
                    digest(4),
                )
                .is_none(),
                "decision {decision:?} must not produce a receipt"
            );
        }
        for decision in [GateDecision::Approve, GateDecision::BreakGlass] {
            assert!(
                ApprovalReceipt::bind(
                    GateId::new(7),
                    RunId::new(3),
                    decision,
                    person("alice"),
                    digest(1),
                    digest(2),
                    100,
                    digest(4),
                )
                .is_some(),
                "decision {decision:?} must produce a receipt"
            );
        }
    }

    #[test]
    fn outcome_from_decision_covers_all_five_decisions() {
        assert_eq!(
            GateOutcome::from_decision(GateDecision::Approve),
            GateOutcome::Approved
        );
        assert_eq!(
            GateOutcome::from_decision(GateDecision::Deny),
            GateOutcome::Denied
        );
        assert_eq!(
            GateOutcome::from_decision(GateDecision::RequestMoreEvidence),
            GateOutcome::MoreEvidenceRequested
        );
        assert_eq!(
            GateOutcome::from_decision(GateDecision::Delegate),
            GateOutcome::Delegated
        );
        assert_eq!(
            GateOutcome::from_decision(GateDecision::BreakGlass),
            GateOutcome::BreakGlassApproved
        );
    }

    #[test]
    fn record_structural_validation_accepts_approval_and_rejects_mismatch() {
        let gate = gate();
        let receipt = ApprovalReceipt::bind(
            gate.id,
            gate.run,
            GateDecision::Approve,
            person("alice"),
            gate.evidence_bundle_digest,
            gate.prompt_digest,
            100,
            digest(4),
        );
        let record = GateOutcomeRecord {
            gate: gate.id,
            run: gate.run,
            action: gate.action,
            decision: Some(GateDecision::Approve),
            outcome: GateOutcome::Approved,
            approver: Some(person("alice")),
            approver_role: Some(role()),
            requester: person("bob"),
            reason: Some("checked".to_string()),
            evidence_shown: true,
            evidence_bundle_digest: gate.evidence_bundle_digest,
            prompt_digest: gate.prompt_digest,
            escalation_policy_digest: gate.escalation_policy_digest,
            delegated_to: None,
            break_glass: false,
            break_glass_justification: None,
            opened_at_ms: 0,
            decided_at_ms: 100,
            receipt,
        };
        assert!(record.is_structurally_valid());
        assert!(record.matches_gate(&gate));

        let mut denied_without_receipt = record.clone();
        denied_without_receipt.outcome = GateOutcome::Denied;
        denied_without_receipt.decision = Some(GateDecision::Deny);
        assert!(!denied_without_receipt.is_structurally_valid());

        let mut wrong_digest = record.clone();
        wrong_digest.evidence_bundle_digest = digest(99);
        assert!(!wrong_digest.matches_gate(&gate));
    }
}
