use super::{ApproverRole, GateDigest, GateId, RiskClass, SeparationOfDuties};
use crate::ids::{ActionId, RunId};
use serde::{Deserialize, Serialize};

/// Manual gate definition (MASTER §77.15 required fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualGate {
    /// Gate identifier.
    pub id: GateId,
    /// Run the gate belongs to.
    pub run: RunId,
    /// Gated action.
    pub action: ActionId,
    /// Risk class of the gated action.
    pub risk: RiskClass,
    /// Whether a compensation protocol exists for the gated action.
    pub compensatable: bool,
    /// Role required to approve.
    pub required_role: ApproverRole,
    /// Separation-of-duties rule.
    pub separation_of_duties: SeparationOfDuties,
    /// Digest of the evidence bundle the approver must be shown.
    pub evidence_bundle_digest: GateDigest,
    /// Digest of the prompt presented to the approver.
    pub prompt_digest: GateDigest,
    /// Decision window in milliseconds from gate opening.
    pub timeout_ms: u64,
    /// Digest of the escalation policy applied on timeout.
    pub escalation_policy_digest: GateDigest,
    /// Whether break-glass decisions are permitted.
    pub break_glass_allowed: bool,
    /// Whether approval requires a non-empty reason.
    pub approval_reason_required: bool,
}

impl ManualGate {
    /// Deadline in milliseconds for a gate opened at `opened_at_ms`, or
    /// `None` when the addition overflows.
    #[must_use]
    pub const fn deadline_ms(&self, opened_at_ms: u64) -> Option<u64> {
        match opened_at_ms.checked_add(self.timeout_ms) {
            Some(deadline) => Some(deadline),
            None => None,
        }
    }

    /// Returns true when separation of duties applies to an approval:
    /// always for high-risk gates, otherwise only when the gate rule
    /// requires a distinct approver.
    #[must_use]
    pub const fn separation_applies(&self) -> bool {
        match self.risk {
            RiskClass::High => true,
            RiskClass::Low | RiskClass::Medium => {
                matches!(
                    self.separation_of_duties,
                    SeparationOfDuties::DistinctFromRequester
                )
            }
        }
    }

    /// Returns true when approval requires the evidence to have been
    /// displayed: high-risk, non-compensatable actions only.
    #[must_use]
    pub const fn evidence_display_required(&self) -> bool {
        matches!(self.risk, RiskClass::High) && !self.compensatable
    }

    /// Checks the structural invariants of the gate: non-zero gate, run,
    /// and action identifiers, and a valid required role. Zero identifiers
    /// are the null/placeholder values and can never name a real gate.
    #[must_use]
    pub fn is_structurally_valid(&self) -> bool {
        self.id.get() != 0
            && self.run.get() != 0
            && self.action.get() != 0
            && self.required_role.is_valid()
    }
}
