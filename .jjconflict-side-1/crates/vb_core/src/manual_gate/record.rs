use super::{
    ApprovalReceipt, ApproverId, ApproverRole, GateDecision, GateDigest, GateId, GateOutcome,
    ManualGate,
};
use crate::ids::{ActionId, RunId};
use serde::{Deserialize, Serialize};

/// Durable record of a gate outcome, journaled by storage and replayed
/// without external effects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateOutcomeRecord {
    /// Gate the record belongs to.
    pub gate: GateId,
    /// Run the record belongs to.
    pub run: RunId,
    /// Gated action.
    pub action: ActionId,
    /// Operator decision; `None` only for escalations, which no operator made.
    pub decision: Option<GateDecision>,
    /// Outcome of the evaluation.
    pub outcome: GateOutcome,
    /// Deciding operator; `None` only for escalations.
    pub approver: Option<ApproverId>,
    /// Role the approver claimed; `None` only for escalations.
    pub approver_role: Option<ApproverRole>,
    /// Operator who requested the gated action.
    pub requester: ApproverId,
    /// Approval reason, when supplied.
    pub reason: Option<String>,
    /// Whether the evidence bundle was displayed to the approver.
    pub evidence_shown: bool,
    /// Digest of the evidence bundle bound to the outcome.
    pub evidence_bundle_digest: GateDigest,
    /// Digest of the prompt bound to the outcome.
    pub prompt_digest: GateDigest,
    /// Digest of the escalation policy bound to the outcome.
    pub escalation_policy_digest: GateDigest,
    /// Delegate target; present only for delegations.
    pub delegated_to: Option<ApproverId>,
    /// True only for break-glass approvals.
    pub break_glass: bool,
    /// Break-glass justification; present only for break-glass approvals.
    pub break_glass_justification: Option<String>,
    /// Gate opening time in milliseconds since the UNIX epoch.
    pub opened_at_ms: u64,
    /// Outcome time in milliseconds since the UNIX epoch.
    pub decided_at_ms: u64,
    /// Approval receipt; present only for approvals.
    pub receipt: Option<ApprovalReceipt>,
}

impl GateOutcomeRecord {
    /// Checks the structural invariants of the record: decision/outcome
    /// correspondence, receipt presence exactly for approvals, delegation
    /// fields exactly for delegations, and break-glass fields exactly for
    /// break-glass approvals.
    #[must_use]
    pub fn is_structurally_valid(&self) -> bool {
        let decision_matches = match (self.decision, self.outcome) {
            (Some(decision), outcome) => GateOutcome::from_decision(decision) == outcome,
            (None, GateOutcome::Escalated) => true,
            (None, _) => false,
        };
        if !decision_matches {
            return false;
        }
        let receipt_ok = match self.outcome {
            GateOutcome::Approved | GateOutcome::BreakGlassApproved => self.receipt.is_some(),
            _ => self.receipt.is_none(),
        };
        if !receipt_ok {
            return false;
        }
        let delegation_ok = match self.outcome {
            GateOutcome::Delegated => self.delegated_to.is_some(),
            _ => self.delegated_to.is_none(),
        };
        if !delegation_ok {
            return false;
        }
        let break_glass_ok = match self.outcome {
            GateOutcome::BreakGlassApproved => {
                self.break_glass
                    && self
                        .break_glass_justification
                        .as_ref()
                        .is_some_and(|text| !text.is_empty())
            }
            _ => !self.break_glass && self.break_glass_justification.is_none(),
        };
        if !break_glass_ok {
            return false;
        }
        match self.outcome {
            GateOutcome::Escalated => self.approver.is_none() && self.approver_role.is_none(),
            _ => self.approver.is_some() && self.approver_role.is_some(),
        }
    }

    /// Checks the record against its gate: identifiers, action, and the
    /// evidence, prompt, and escalation digests must all match.
    #[must_use]
    pub fn matches_gate(&self, gate: &ManualGate) -> bool {
        self.gate == gate.id
            && self.run == gate.run
            && self.action == gate.action
            && self.evidence_bundle_digest == gate.evidence_bundle_digest
            && self.prompt_digest == gate.prompt_digest
            && self.escalation_policy_digest == gate.escalation_policy_digest
    }
}
