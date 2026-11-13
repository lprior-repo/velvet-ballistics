use super::{
    ApprovalReceipt, ApproverId, ApproverRole, GateBinding, GateDigest, GateOutcomeRecord,
    ManualGate, OperatorTrust, RiskClass, SeparationOfDuties,
};
use crate::ids::ActionId;
use serde::{Deserialize, Serialize};

/// A `ManualGate` plus the run binding required for full accountability.
///
/// The embedded `ManualGate` keeps its original layout and semantics. The
/// wrapper is the only way to construct a gate that the bound approval path
/// accepts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundManualGate {
    /// Unchanged V1 gate definition.
    pub gate: ManualGate,
    /// Artifact/policy/input/attempt binding.
    pub binding: GateBinding,
}

impl BoundManualGate {
    /// Returns true when the V1 gate is valid and the binding is real.
    #[must_use]
    pub fn is_structurally_valid(&self) -> bool {
        self.gate.is_structurally_valid() && self.binding.is_valid()
    }
}

/// Additive approval receipt that binds the V1 receipt plus every remaining
/// accountable field. It carries no capability, scope, or resource data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundApprovalReceipt {
    /// Unchanged V1 receipt; its V1 digest still verifies its old fields.
    pub receipt: ApprovalReceipt,
    /// Gated action.
    pub action: ActionId,
    /// Operator who requested the gated action, from the durable opening.
    pub requester: ApproverId,
    /// Role the approver claimed.
    pub approver_role: ApproverRole,
    /// Risk class of the gated action.
    pub risk: RiskClass,
    /// Separation-of-duties rule applied to the gate.
    pub separation: SeparationOfDuties,
    /// Artifact/policy/input/attempt binding of the approved run.
    pub binding: GateBinding,
    /// Provenance of the claimed approver identity.
    pub operator_trust: OperatorTrust,
    /// V2 digest over every field above except this digest.
    pub digest: GateDigest,
}

impl BoundApprovalReceipt {
    /// Returns the V2 digest consumed by a bound capability grant.
    #[must_use]
    pub const fn digest(&self) -> GateDigest {
        self.digest
    }
}

/// Additive durable record pairing the unchanged V1 outcome with its binding,
/// trust provenance, and bound approval receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundGateOutcomeRecord {
    /// Unchanged V1 outcome record, including its V1 receipt when approving.
    pub record: GateOutcomeRecord,
    /// Artifact/policy/input/attempt binding of the run.
    pub binding: GateBinding,
    /// Provenance of the claimed approver identity; `None` only for escalation.
    pub operator_trust: Option<OperatorTrust>,
    /// Bound receipt; present exactly for approve/break-glass.
    pub bound_receipt: Option<BoundApprovalReceipt>,
}

impl BoundGateOutcomeRecord {
    /// Checks the V1 record, the binding, trust alignment, and bound-receipt
    /// equality. Escalations carry no trust claim and no bound receipt.
    #[must_use]
    pub fn is_structurally_valid(&self) -> bool {
        if !self.binding.is_valid() || !self.record.is_structurally_valid() {
            return false;
        }
        let approval = matches!(
            self.record.outcome,
            super::GateOutcome::Approved | super::GateOutcome::BreakGlassApproved
        );
        let escalated = self.record.outcome == super::GateOutcome::Escalated;
        if escalated {
            if self.operator_trust.is_some() || self.bound_receipt.is_some() {
                return false;
            }
        } else if self.operator_trust.is_none() {
            return false;
        }
        match (&self.bound_receipt, approval) {
            (Some(bound), true) => self.bound_receipt_matches(bound),
            (None, false) => true,
            _ => false,
        }
    }

    fn bound_receipt_matches(&self, bound: &BoundApprovalReceipt) -> bool {
        self.record.receipt.as_ref() == Some(&bound.receipt)
            && self.record.decision == Some(bound.receipt.decision)
            && self.record.gate == bound.receipt.gate
            && self.record.run == bound.receipt.run
            && self.record.action == bound.action
            && self.record.approver.as_ref() == Some(&bound.receipt.approver)
            && self.record.requester == bound.requester
            && self.record.approver_role.as_ref() == Some(&bound.approver_role)
            && self.binding == bound.binding
            && self.operator_trust == Some(bound.operator_trust)
            && self.record.evidence_bundle_digest == bound.receipt.evidence_bundle_digest
            && self.record.prompt_digest == bound.receipt.prompt_digest
            && self.record.decided_at_ms == bound.receipt.decided_at_ms
    }

    /// Checks the bound record against its bound gate.
    #[must_use]
    pub fn matches_bound_gate(&self, gate: &BoundManualGate) -> bool {
        self.record.matches_gate(&gate.gate) && self.binding == gate.binding
    }
}
