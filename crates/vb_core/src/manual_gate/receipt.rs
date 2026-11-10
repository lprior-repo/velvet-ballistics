use super::{ApproverId, GateDecision, GateDigest, GateId};
use crate::ids::RunId;
use serde::{Deserialize, Serialize};

/// Approval receipt: digest-bound proof of an approval decision.
///
/// The receipt carries identities and digests only. It contains no
/// capability, scope, or resource field, so by construction it cannot
/// broaden artifact capabilities. A capability grant that cites this
/// receipt must be issued and journaled separately.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalReceipt {
    /// Gate the receipt approves.
    pub gate: GateId,
    /// Run the receipt approves.
    pub run: RunId,
    /// Approval decision: approve or break-glass only.
    pub decision: GateDecision,
    /// Approving operator.
    pub approver: ApproverId,
    /// Digest of the evidence bundle bound to the approval.
    pub evidence_bundle_digest: GateDigest,
    /// Digest of the prompt bound to the approval.
    pub prompt_digest: GateDigest,
    /// Approval time in milliseconds since the UNIX epoch.
    pub decided_at_ms: u64,
    /// Receipt digest binding every field above; computed by the runtime
    /// evaluator and consumed by the later capability-grant batch.
    pub digest: GateDigest,
}

impl ApprovalReceipt {
    /// Binds a receipt, or `None` when the decision is not an approval
    /// decision (only approve and break-glass produce receipts).
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn bind(
        gate: GateId,
        run: RunId,
        decision: GateDecision,
        approver: ApproverId,
        evidence_bundle_digest: GateDigest,
        prompt_digest: GateDigest,
        decided_at_ms: u64,
        digest: GateDigest,
    ) -> Option<Self> {
        match decision {
            GateDecision::Approve | GateDecision::BreakGlass => Some(Self {
                gate,
                run,
                decision,
                approver,
                evidence_bundle_digest,
                prompt_digest,
                decided_at_ms,
                digest,
            }),
            GateDecision::Deny | GateDecision::RequestMoreEvidence | GateDecision::Delegate => None,
        }
    }

    /// Returns the receipt digest consumed by capability grants.
    #[must_use]
    pub const fn digest(&self) -> GateDigest {
        self.digest
    }
}
