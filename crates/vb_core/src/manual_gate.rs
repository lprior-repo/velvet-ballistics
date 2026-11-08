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

use crate::ids::{ActionId, RunId};
use serde::{Deserialize, Serialize};

/// Maximum length in bytes for an approver identity or role name.
pub const MAX_APPROVER_TEXT_LEN: usize = 128;

/// 32-byte gate digest, in the style of [`crate::policy::PolicyDigest`].
/// Callers supply the actual digest of the artifact they computed; never
/// fabricated or default-filled with zeros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct GateDigest([u8; 32]);

impl GateDigest {
    /// Creates a digest from verified bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Numeric gate identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct GateId(u64);

impl GateId {
    /// Creates a gate identifier from a validated integer.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw identifier value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Risk class of the action behind a gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum RiskClass {
    /// Low-risk action.
    Low = 0,
    /// Medium-risk action.
    Medium = 1,
    /// High-risk action; separation of duties always applies.
    High = 2,
}

/// Separation-of-duties rule carried by a gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SeparationOfDuties {
    /// No separation requirement beyond the risk-class rule.
    Disabled = 0,
    /// Approver must differ from the requester at any risk class.
    DistinctFromRequester = 1,
}

/// Structured decision options (MASTER §77.15). Closed: exactly five.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum GateDecision {
    /// Approve the gated action.
    Approve = 0,
    /// Deny the gated action.
    Deny = 1,
    /// Ask for more evidence before deciding.
    RequestMoreEvidence = 2,
    /// Hand the gate to another approver.
    Delegate = 3,
    /// Emergency approval outside the normal role, with justification.
    BreakGlass = 4,
}

/// Durable outcome of a gate evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum GateOutcome {
    /// Action approved.
    Approved = 0,
    /// Action denied.
    Denied = 1,
    /// More evidence requested; the gate stays open.
    MoreEvidenceRequested = 2,
    /// Gate delegated to another approver.
    Delegated = 3,
    /// Action approved via break-glass.
    BreakGlassApproved = 4,
    /// Gate timed out and escalated under the escalation policy.
    Escalated = 5,
}

impl GateOutcome {
    /// Outcome implied by an operator decision, or `None` for escalations,
    /// which are enforcement outcomes rather than operator decisions.
    #[must_use]
    pub const fn from_decision(decision: GateDecision) -> Self {
        match decision {
            GateDecision::Approve => Self::Approved,
            GateDecision::Deny => Self::Denied,
            GateDecision::RequestMoreEvidence => Self::MoreEvidenceRequested,
            GateDecision::Delegate => Self::Delegated,
            GateDecision::BreakGlass => Self::BreakGlassApproved,
        }
    }
}

/// Approver role required by a gate, e.g. `deploy-approver`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproverRole {
    /// Role name; empty and over-long names are rejected by [`ApproverRole::new`].
    pub name: String,
}

impl ApproverRole {
    /// Creates a role, or `None` when the name is empty or over-long.
    #[must_use]
    pub fn new(name: String) -> Option<Self> {
        if name.is_empty() || name.len() > MAX_APPROVER_TEXT_LEN {
            None
        } else {
            Some(Self { name })
        }
    }

    /// Returns the role name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns true when the name is non-empty and bounded.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.name.is_empty() && self.name.len() <= MAX_APPROVER_TEXT_LEN
    }
}

/// Identity of a requester or approver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproverId {
    /// Identity string; empty and over-long values are rejected by [`ApproverId::new`].
    pub value: String,
}

impl ApproverId {
    /// Creates an identity, or `None` when the value is empty or over-long.
    #[must_use]
    pub fn new(value: String) -> Option<Self> {
        if value.is_empty() || value.len() > MAX_APPROVER_TEXT_LEN {
            None
        } else {
            Some(Self { value })
        }
    }

    /// Returns the identity string.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns true when the value is non-empty and bounded.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.value.is_empty() && self.value.len() <= MAX_APPROVER_TEXT_LEN
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
