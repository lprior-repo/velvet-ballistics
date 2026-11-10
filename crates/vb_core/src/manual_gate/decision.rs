use super::{ApproverId, ApproverRole, GateDecision, GateDigest, GateId};
use serde::{Deserialize, Serialize};

/// Operator submission to a gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateDecisionInput {
    /// Gate being decided.
    pub gate: GateId,
    /// Submitted decision.
    pub decision: GateDecision,
    /// Deciding operator.
    pub approver: ApproverId,
    /// Role the approver claims.
    pub approver_role: ApproverRole,
    /// Operator who requested the gated action.
    pub requester: ApproverId,
    /// Approval reason; required when the gate demands one.
    pub reason: Option<String>,
    /// Whether the evidence bundle was displayed to the approver.
    pub evidence_shown: bool,
    /// Digest of the evidence bundle actually shown.
    pub evidence_bundle_digest: GateDigest,
    /// Delegate target; required for [`GateDecision::Delegate`].
    pub delegated_to: Option<ApproverId>,
    /// Break-glass justification; required for [`GateDecision::BreakGlass`].
    pub break_glass_justification: Option<String>,
    /// Decision time in milliseconds since the UNIX epoch.
    pub decided_at_ms: u64,
}
