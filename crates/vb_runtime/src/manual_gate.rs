#![forbid(unsafe_code)]
//! Manual gate evaluation and enforcement (MASTER §77.15, V1-MIN).
//!
//! Pure functions over the value types in [`vb_core::manual_gate`]:
//! decision evaluation, timeout/escalation enforcement, receipt digest
//! binding, and replay reconstruction. Nothing here performs an external
//! effect, mints a capability, or touches the journal directly; callers
//! journal the returned [`GateOutcomeRecord`] as a
//! `JournalEvent::GateDecisionEvent`.
//!
//! Enforcement summary:
//! - The submitted evidence digest must equal the gate's evidence digest.
//! - High-risk, non-compensatable approvals require evidence display.
//! - Separation of duties (requester != approver) always applies to
//!   high-risk approvals and, when the gate rule requires it, to every
//!   approval. It also applies to break-glass approvals at high risk.
//! - Approval requires the gate's approver role and, when the gate
//!   demands it, a non-empty reason.
//! - Break-glass waives the role check (that is the point of break-glass)
//!   but requires the gate to allow it, a non-empty justification, and
//!   leaves a full audit trail in the record.
//! - Decisions after the deadline are rejected; [`evaluate_timeout`]
//!   produces the escalation record instead.
//! - Replay reconstructs history from records only. It never synthesizes
//!   a receipt and never authorizes a live effect.

use vb_core::ids::RunId;
use vb_core::manual_gate::{
    ApprovalReceipt, ApproverId, GateDecision, GateDecisionInput, GateDigest, GateOutcome,
    GateOutcomeRecord, ManualGate, RiskClass,
};

use crate::shard::types::OpenGate;
use indexmap::IndexMap;

/// Typed manual-gate evaluation errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GateError {
    /// Submitted gate identifier does not match the gate.
    #[error("gate id mismatch: submitted {submitted} does not match gate {expected}")]
    GateIdMismatch {
        /// Gate identifier from the gate definition.
        expected: u64,
        /// Gate identifier from the submission.
        submitted: u64,
    },
    /// Approver role does not match the gate's required role.
    #[error("approver role mismatch: required {required} but approver holds {actual}")]
    WrongRole {
        /// Required role name.
        required: String,
        /// Approver's claimed role name.
        actual: String,
    },
    /// Evidence bundle shown to the approver is not the gate's bundle.
    #[error("evidence bundle digest does not match the gate evidence bundle")]
    EvidenceDigestMismatch,
    /// High-risk, non-compensatable approval without evidence display.
    #[error("evidence must be displayed before approving a high-risk non-compensatable action")]
    EvidenceNotShown,
    /// Requester and approver are the same operator where separation
    /// of duties applies.
    #[error("separation of duties violated: requester and approver must differ")]
    SeparationOfDutiesViolation,
    /// Approval without a reason where the gate requires one.
    #[error("approval reason is required by this gate")]
    ReasonRequired,
    /// Break-glass decision on a gate that does not allow it.
    #[error("break-glass is not allowed by this gate")]
    BreakGlassNotAllowed,
    /// Break-glass decision without a justification.
    #[error("break-glass justification is required")]
    BreakGlassJustificationRequired,
    /// Delegation without a target.
    #[error("delegation requires a target approver")]
    DelegateTargetMissing,
    /// Delegation target is the delegating approver.
    #[error("delegation target must differ from the delegating approver")]
    DelegateTargetIsApprover,
    /// Decision submitted after the gate deadline.
    #[error("decision at {decided_at_ms} is after the gate deadline {deadline_ms}")]
    DecisionAfterTimeout {
        /// Gate deadline in milliseconds.
        deadline_ms: u64,
        /// Submitted decision time in milliseconds.
        decided_at_ms: u64,
    },
    /// Timeout evaluation before the deadline.
    #[error("gate has not timed out yet")]
    NotTimedOut,
    /// Deadline arithmetic overflowed.
    #[error("gate deadline arithmetic overflowed")]
    DeadlineOverflow,
    /// Approver identity is empty or over-long.
    #[error("approver identity is invalid")]
    InvalidApprover,
    /// Requester identity is empty or over-long.
    #[error("requester identity is invalid")]
    InvalidRequester,
    /// Approver role is empty or over-long.
    #[error("approver role is invalid")]
    InvalidRole,
    /// Receipt digest encoding failed (approver length does not fit u64).
    #[error("receipt digest encoding failed")]
    ReceiptDigestEncoding,
    /// Record fails structural validation.
    #[error("gate outcome record is structurally invalid")]
    InvalidRecord,
    /// Record does not match its gate.
    #[error("gate outcome record does not match its gate")]
    RecordGateMismatch,
    /// Replay found a receipt on a non-approval outcome.
    #[error("replay record carries a receipt without an approval outcome")]
    ReplayReceiptMismatch,
    /// Replay found a gate opened twice without a closing decision.
    #[error("replay opened gate {gate} twice without a closing decision")]
    ReplayDuplicateOpen {
        /// Gate identifier opened twice.
        gate: u64,
    },
    /// Replay found a gate-opened event carrying a structurally invalid gate.
    #[error("replay gate-opened event carries a structurally invalid gate")]
    ReplayInvalidGate,
}

/// Canonical decision byte for receipt digest encoding.
fn decision_byte(decision: GateDecision) -> u8 {
    match decision {
        GateDecision::Approve => 0,
        GateDecision::Deny => 1,
        GateDecision::RequestMoreEvidence => 2,
        GateDecision::Delegate => 3,
        GateDecision::BreakGlass => 4,
    }
}

/// Computes the receipt digest binding gate, run, decision, approver,
/// evidence digest, prompt digest, and decision time, with domain
/// separation. The same inputs always produce the same digest; any
/// field change produces a different digest.
pub fn compute_receipt_digest(
    gate: u64,
    run: RunId,
    decision: GateDecision,
    approver: &ApproverId,
    evidence_bundle_digest: GateDigest,
    prompt_digest: GateDigest,
    decided_at_ms: u64,
) -> Result<GateDigest, GateError> {
    let approver_len =
        u64::try_from(approver.value().len()).map_err(|_| GateError::ReceiptDigestEncoding)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"vb-gate-receipt-v1");
    hasher.update(&gate.to_le_bytes());
    hasher.update(&run.get().to_le_bytes());
    hasher.update(&[decision_byte(decision)]);
    hasher.update(&approver_len.to_le_bytes());
    hasher.update(approver.value().as_bytes());
    hasher.update(&evidence_bundle_digest.as_bytes());
    hasher.update(&prompt_digest.as_bytes());
    hasher.update(&decided_at_ms.to_le_bytes());
    Ok(GateDigest::from_bytes(*hasher.finalize().as_bytes()))
}

/// Verifies a receipt against its gate: the receipt's evidence and prompt
/// digests must match the gate, and its digest must equal the digest
/// recomputed from the receipt's own bound fields.
pub fn verify_receipt(receipt: &ApprovalReceipt, gate: &ManualGate) -> Result<bool, GateError> {
    if receipt.gate != gate.id
        || receipt.run != gate.run
        || receipt.evidence_bundle_digest != gate.evidence_bundle_digest
        || receipt.prompt_digest != gate.prompt_digest
    {
        return Ok(false);
    }
    let recomputed = compute_receipt_digest(
        receipt.gate.get(),
        receipt.run,
        receipt.decision,
        &receipt.approver,
        receipt.evidence_bundle_digest,
        receipt.prompt_digest,
        receipt.decided_at_ms,
    )?;
    Ok(recomputed == receipt.digest)
}

fn reason_present(reason: &Option<String>) -> bool {
    reason.as_ref().is_some_and(|text| !text.trim().is_empty())
}

/// Evaluates an operator decision against a gate opened at
/// `opened_at_ms`. Returns the durable outcome record, journaled by the
/// caller. Approval decisions carry a digest-bound receipt; no other
/// decision does.
pub fn evaluate_decision(
    gate: &ManualGate,
    opened_at_ms: u64,
    input: &GateDecisionInput,
) -> Result<GateOutcomeRecord, GateError> {
    if !input.approver.is_valid() {
        return Err(GateError::InvalidApprover);
    }
    if !input.requester.is_valid() {
        return Err(GateError::InvalidRequester);
    }
    if !input.approver_role.is_valid() {
        return Err(GateError::InvalidRole);
    }
    if input.gate != gate.id {
        return Err(GateError::GateIdMismatch {
            expected: gate.id.get(),
            submitted: input.gate.get(),
        });
    }
    let deadline = gate
        .deadline_ms(opened_at_ms)
        .ok_or(GateError::DeadlineOverflow)?;
    if input.decided_at_ms > deadline {
        return Err(GateError::DecisionAfterTimeout {
            deadline_ms: deadline,
            decided_at_ms: input.decided_at_ms,
        });
    }
    if input.evidence_bundle_digest != gate.evidence_bundle_digest {
        return Err(GateError::EvidenceDigestMismatch);
    }

    let mut record = GateOutcomeRecord {
        gate: gate.id,
        run: gate.run,
        action: gate.action,
        decision: Some(input.decision),
        outcome: GateOutcome::from_decision(input.decision),
        approver: Some(input.approver.clone()),
        approver_role: Some(input.approver_role.clone()),
        requester: input.requester.clone(),
        reason: input.reason.clone(),
        evidence_shown: input.evidence_shown,
        evidence_bundle_digest: gate.evidence_bundle_digest,
        prompt_digest: gate.prompt_digest,
        escalation_policy_digest: gate.escalation_policy_digest,
        delegated_to: None,
        break_glass: false,
        break_glass_justification: None,
        opened_at_ms,
        decided_at_ms: input.decided_at_ms,
        receipt: None,
    };

    match input.decision {
        GateDecision::Approve => {
            check_role(gate, input)?;
            if gate.approval_reason_required && !reason_present(&input.reason) {
                return Err(GateError::ReasonRequired);
            }
            if gate.evidence_display_required() && !input.evidence_shown {
                return Err(GateError::EvidenceNotShown);
            }
            check_separation(gate, input)?;
            let digest = compute_receipt_digest(
                gate.id.get(),
                gate.run,
                GateDecision::Approve,
                &input.approver,
                gate.evidence_bundle_digest,
                gate.prompt_digest,
                input.decided_at_ms,
            )?;
            record.receipt = ApprovalReceipt::bind(
                gate.id,
                gate.run,
                GateDecision::Approve,
                input.approver.clone(),
                gate.evidence_bundle_digest,
                gate.prompt_digest,
                input.decided_at_ms,
                digest,
            );
        }
        GateDecision::Deny | GateDecision::RequestMoreEvidence => {
            check_role(gate, input)?;
        }
        GateDecision::Delegate => {
            check_role(gate, input)?;
            let target = input
                .delegated_to
                .clone()
                .ok_or(GateError::DelegateTargetMissing)?;
            if target == input.approver {
                return Err(GateError::DelegateTargetIsApprover);
            }
            record.delegated_to = Some(target);
        }
        GateDecision::BreakGlass => {
            if !gate.break_glass_allowed {
                return Err(GateError::BreakGlassNotAllowed);
            }
            if !reason_present(&input.break_glass_justification) {
                return Err(GateError::BreakGlassJustificationRequired);
            }
            // Role check waived: break-glass exists for operators outside
            // the normal role. Separation of duties still applies at
            // high risk, and the justification is journaled.
            if matches!(gate.risk, RiskClass::High) && input.requester == input.approver {
                return Err(GateError::SeparationOfDutiesViolation);
            }
            record.break_glass = true;
            record.break_glass_justification = input.break_glass_justification.clone();
            let digest = compute_receipt_digest(
                gate.id.get(),
                gate.run,
                GateDecision::BreakGlass,
                &input.approver,
                gate.evidence_bundle_digest,
                gate.prompt_digest,
                input.decided_at_ms,
            )?;
            record.receipt = ApprovalReceipt::bind(
                gate.id,
                gate.run,
                GateDecision::BreakGlass,
                input.approver.clone(),
                gate.evidence_bundle_digest,
                gate.prompt_digest,
                input.decided_at_ms,
                digest,
            );
        }
    }

    if !record.is_structurally_valid() {
        return Err(GateError::InvalidRecord);
    }
    Ok(record)
}

fn check_role(gate: &ManualGate, input: &GateDecisionInput) -> Result<(), GateError> {
    if input.approver_role != gate.required_role {
        return Err(GateError::WrongRole {
            required: gate.required_role.name().to_string(),
            actual: input.approver_role.name().to_string(),
        });
    }
    Ok(())
}

fn check_separation(gate: &ManualGate, input: &GateDecisionInput) -> Result<(), GateError> {
    if gate.separation_applies() && input.requester == input.approver {
        return Err(GateError::SeparationOfDutiesViolation);
    }
    Ok(())
}

/// Produces the escalation record for a gate whose deadline passed at
/// `now_ms`, or [`GateError::NotTimedOut`] while the gate is still open.
/// The escalation carries the gate's escalation policy digest and the
/// original requester; it carries no decision, no approver, and no receipt.
pub fn evaluate_timeout(
    gate: &ManualGate,
    requester: &ApproverId,
    opened_at_ms: u64,
    now_ms: u64,
) -> Result<GateOutcomeRecord, GateError> {
    let deadline = gate
        .deadline_ms(opened_at_ms)
        .ok_or(GateError::DeadlineOverflow)?;
    if now_ms <= deadline {
        return Err(GateError::NotTimedOut);
    }
    Ok(GateOutcomeRecord {
        gate: gate.id,
        run: gate.run,
        action: gate.action,
        decision: None,
        outcome: GateOutcome::Escalated,
        approver: None,
        approver_role: None,
        requester: requester.clone(),
        reason: None,
        evidence_shown: false,
        evidence_bundle_digest: gate.evidence_bundle_digest,
        prompt_digest: gate.prompt_digest,
        escalation_policy_digest: gate.escalation_policy_digest,
        delegated_to: None,
        break_glass: false,
        break_glass_justification: None,
        opened_at_ms,
        decided_at_ms: now_ms,
        receipt: None,
    })
}

/// Reconstructed historical gate outcome: replay output only. It carries
/// a receipt digest when the historical outcome was an approval, and
/// never a receipt itself, so replay cannot mint capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateReplay {
    /// Gate the replayed outcome belongs to.
    pub gate: u64,
    /// Run the replayed outcome belongs to.
    pub run: RunId,
    /// Replayed outcome.
    pub outcome: GateOutcome,
    /// Replayed operator decision, if any.
    pub decision: Option<GateDecision>,
    /// Receipt digest of the historical approval, if any.
    pub receipt_digest: Option<GateDigest>,
    /// True when the replayed outcome is an escalation.
    pub escalated: bool,
}

/// Reconstructs a single historical outcome against its gate: structural
/// validation plus digest binding. No external effect is performed.
pub fn replay_gate_outcome(
    record: &GateOutcomeRecord,
    gate: &ManualGate,
) -> Result<GateReplay, GateError> {
    if !record.is_structurally_valid() {
        return Err(GateError::InvalidRecord);
    }
    if !record.matches_gate(gate) {
        return Err(GateError::RecordGateMismatch);
    }
    let receipt_digest = match (&record.receipt, record.outcome) {
        (Some(receipt), GateOutcome::Approved | GateOutcome::BreakGlassApproved) => {
            Some(receipt.digest)
        }
        (None, _) => None,
        (Some(_), _) => return Err(GateError::ReplayReceiptMismatch),
    };
    Ok(GateReplay {
        gate: record.gate.get(),
        run: record.run,
        outcome: record.outcome,
        decision: record.decision,
        receipt_digest,
        escalated: record.outcome == GateOutcome::Escalated,
    })
}

/// Reconstructs gate history from journal events in order: every
/// `GateDecisionEvent` becomes its replay, structurally validated.
/// Non-gate events are ignored. Replay performs no external effect.
pub fn replay_gate_history(
    events: &[vb_storage::JournalEvent],
) -> Result<Vec<GateReplay>, GateError> {
    let mut replays = Vec::new();
    for event in events {
        if let vb_storage::JournalEvent::GateDecisionEvent { record, .. } = event {
            if !record.is_structurally_valid() {
                return Err(GateError::InvalidRecord);
            }
            let receipt_digest = match (&record.receipt, record.outcome) {
                (Some(receipt), GateOutcome::Approved | GateOutcome::BreakGlassApproved) => {
                    Some(receipt.digest)
                }
                (None, _) => None,
                (Some(_), _) => return Err(GateError::ReplayReceiptMismatch),
            };
            replays.push(GateReplay {
                gate: record.gate.get(),
                run: record.run,
                outcome: record.outcome,
                decision: record.decision,
                receipt_digest,
                escalated: record.outcome == GateOutcome::Escalated,
            });
        }
    }
    Ok(replays)
}

/// Builds the journal event carrying a gate outcome record. The caller
/// supplies the run and per-run storage sequence; the record's own run
/// must match, enforced by the storage wire codec on serialization.
pub fn gate_decision_event(
    run: RunId,
    seq: vb_storage::EventSeq,
    record: GateOutcomeRecord,
) -> vb_storage::JournalEvent {
    vb_storage::JournalEvent::GateDecisionEvent {
        run,
        seq,
        record: Box::new(record),
    }
}

/// Builds the journal event carrying a gate opening. The caller supplies
/// the run and per-run storage sequence; the gate's own run must match,
/// enforced by the storage wire codec on serialization.
pub fn gate_opened_event(
    gate: ManualGate,
    opened_at_ms: u64,
    requester: ApproverId,
    seq: vb_storage::EventSeq,
) -> vb_storage::JournalEvent {
    vb_storage::JournalEvent::GateOpenedEvent {
        run: gate.run,
        seq,
        gate,
        opened_at_ms,
        requester,
    }
}

/// Reconstructs the currently open gates from journal events in order.
///
/// A `GateOpenedEvent` opens its gate; a terminal `GateDecisionEvent`
/// (approve, deny, break-glass, escalate) closes it; more-evidence
/// leaves it open and delegation updates its delegate. Decisions for a
/// gate with no opening in this event stream are ignored: the gate was
/// opened before the recovered window and its definition is not in this
/// evidence. A second opening of a gate that is already open, or an
/// opening carrying a structurally invalid gate, fails the reconstruction:
/// the journal is inconsistent and recovery must not silently drop a gate.
/// Replay performs no external effect; the returned gates are in opening
/// order.
///
/// The runtime recovery path calls this over the full per-run journal and
/// threads the result through `ResumableRecoveryProduct` into
/// `RecoverRunCommand`, so a crash after opening leaves the gate open
/// across recovery instead of silently resuming the gated run.
pub(crate) fn reconstruct_open_gates(
    events: &[vb_storage::JournalEvent],
) -> Result<Vec<OpenGate>, GateError> {
    let open = events.iter().try_fold(
        IndexMap::new(),
        |mut open: IndexMap<vb_core::manual_gate::GateId, OpenGate>,
         event: &vb_storage::JournalEvent| {
            match event {
                vb_storage::JournalEvent::GateOpenedEvent {
                    gate,
                    opened_at_ms,
                    requester,
                    ..
                } => {
                    if !gate.is_structurally_valid() || !requester.is_valid() {
                        return Err(GateError::ReplayInvalidGate);
                    }
                    if open.contains_key(&gate.id) {
                        return Err(GateError::ReplayDuplicateOpen {
                            gate: gate.id.get(),
                        });
                    }
                    open.insert(
                        gate.id,
                        OpenGate {
                            gate: gate.clone(),
                            opened_at_ms: *opened_at_ms,
                            requester: requester.clone(),
                            delegated_to: None,
                        },
                    );
                    Ok(open)
                }
                vb_storage::JournalEvent::GateDecisionEvent { record, .. } => {
                    if !record.is_structurally_valid() {
                        return Err(GateError::InvalidRecord);
                    }
                    match record.outcome {
                        GateOutcome::Approved
                        | GateOutcome::Denied
                        | GateOutcome::BreakGlassApproved
                        | GateOutcome::Escalated => {
                            open.shift_remove(&record.gate);
                        }
                        GateOutcome::Delegated => {
                            if let Some(open_gate) = open.get_mut(&record.gate) {
                                open_gate.delegated_to = record.delegated_to.clone();
                            }
                        }
                        GateOutcome::MoreEvidenceRequested => {}
                    }
                    Ok(open)
                }
                _ => Ok(open),
            }
        },
    )?;
    Ok(open.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::ActionId;
    use vb_core::manual_gate::{ApproverRole, GateId, ManualGate, RiskClass, SeparationOfDuties};

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

    #[test]
    fn approve_happy_path_yields_receipt_bound_to_gate() {
        let gate = gate();
        let record = evaluate_decision(&gate, 0, &input(GateDecision::Approve)).unwrap();
        assert_eq!(record.outcome, GateOutcome::Approved);
        assert_eq!(record.decision, Some(GateDecision::Approve));
        let receipt = record.receipt.as_ref().unwrap();
        assert_eq!(receipt.gate, gate.id);
        assert_eq!(receipt.run, gate.run);
        assert_eq!(receipt.evidence_bundle_digest, gate.evidence_bundle_digest);
        assert_eq!(receipt.prompt_digest, gate.prompt_digest);
        assert!(verify_receipt(receipt, &gate).unwrap());
        assert!(record.is_structurally_valid());
        assert!(record.matches_gate(&gate));
    }

    #[test]
    fn deny_and_more_evidence_carry_no_receipt() {
        let gate = gate();
        for (decision, outcome) in [
            (GateDecision::Deny, GateOutcome::Denied),
            (
                GateDecision::RequestMoreEvidence,
                GateOutcome::MoreEvidenceRequested,
            ),
        ] {
            let record = evaluate_decision(&gate, 0, &input(decision)).unwrap();
            assert_eq!(record.outcome, outcome);
            assert!(record.receipt.is_none());
            assert!(record.is_structurally_valid());
        }
    }

    #[test]
    fn wrong_role_is_rejected() {
        let gate = gate();
        let mut bad = input(GateDecision::Approve);
        bad.approver_role = ApproverRole::new("viewer".to_string()).unwrap();
        let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
        assert!(matches!(err, GateError::WrongRole { .. }));
    }

    #[test]
    fn separation_of_duties_rejects_self_approval_for_high_risk() {
        let gate = gate();
        let mut bad = input(GateDecision::Approve);
        bad.approver = person("bob");
        let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
        assert_eq!(err, GateError::SeparationOfDutiesViolation);
    }

    #[test]
    fn separation_rule_extends_to_low_risk_when_gate_requires_it() {
        let mut gate = gate();
        gate.risk = RiskClass::Low;
        let mut bad = input(GateDecision::Approve);
        bad.approver = person("bob");
        let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
        assert_eq!(err, GateError::SeparationOfDutiesViolation);

        gate.separation_of_duties = SeparationOfDuties::Disabled;
        let ok = evaluate_decision(&gate, 0, &input(GateDecision::Approve)).unwrap();
        assert_eq!(ok.outcome, GateOutcome::Approved);
    }

    #[test]
    fn evidence_digest_mismatch_is_rejected() {
        let gate = gate();
        let mut bad = input(GateDecision::Approve);
        bad.evidence_bundle_digest = digest(99);
        let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
        assert_eq!(err, GateError::EvidenceDigestMismatch);
    }

    #[test]
    fn evidence_not_shown_blocks_high_risk_non_compensatable_approval() {
        let gate = gate();
        let mut bad = input(GateDecision::Approve);
        bad.evidence_shown = false;
        let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
        assert_eq!(err, GateError::EvidenceNotShown);

        let mut compensatable = gate.clone();
        compensatable.compensatable = true;
        let ok = evaluate_decision(&compensatable, 0, &bad).unwrap();
        assert_eq!(ok.outcome, GateOutcome::Approved);
    }

    #[test]
    fn approval_reason_required_is_enforced() {
        let gate = gate();
        let mut bad = input(GateDecision::Approve);
        bad.reason = Some("   ".to_string());
        let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
        assert_eq!(err, GateError::ReasonRequired);
        bad.reason = None;
        let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
        assert_eq!(err, GateError::ReasonRequired);
    }

    #[test]
    fn delegate_requires_target_distinct_from_approver() {
        let gate = gate();
        let missing = input(GateDecision::Delegate);
        let err = evaluate_decision(&gate, 0, &missing).unwrap_err();
        assert_eq!(err, GateError::DelegateTargetMissing);

        let mut self_target = input(GateDecision::Delegate);
        self_target.delegated_to = Some(person("alice"));
        let err = evaluate_decision(&gate, 0, &self_target).unwrap_err();
        assert_eq!(err, GateError::DelegateTargetIsApprover);

        let mut ok_input = input(GateDecision::Delegate);
        ok_input.delegated_to = Some(person("carol"));
        let record = evaluate_decision(&gate, 0, &ok_input).unwrap();
        assert_eq!(record.outcome, GateOutcome::Delegated);
        assert_eq!(record.delegated_to, Some(person("carol")));
        assert!(record.receipt.is_none());
    }

    #[test]
    fn break_glass_requires_permission_and_justification_and_audits() {
        let gate = gate();
        let mut no_justification = input(GateDecision::BreakGlass);
        let err = evaluate_decision(&gate, 0, &no_justification).unwrap_err();
        assert_eq!(err, GateError::BreakGlassJustificationRequired);

        no_justification.break_glass_justification = Some("prod is down".to_string());
        // Break-glass waives the role check: a viewer may break glass.
        no_justification.approver_role = ApproverRole::new("viewer".to_string()).unwrap();
        let record = evaluate_decision(&gate, 0, &no_justification).unwrap();
        assert_eq!(record.outcome, GateOutcome::BreakGlassApproved);
        assert!(record.break_glass);
        assert_eq!(
            record.break_glass_justification,
            Some("prod is down".to_string())
        );
        assert!(record.receipt.is_some());

        let mut disallowed = gate.clone();
        disallowed.break_glass_allowed = false;
        let err = evaluate_decision(&disallowed, 0, &no_justification).unwrap_err();
        assert_eq!(err, GateError::BreakGlassNotAllowed);
    }

    #[test]
    fn break_glass_still_enforces_separation_for_high_risk() {
        let gate = gate();
        let mut bad = input(GateDecision::BreakGlass);
        bad.break_glass_justification = Some("prod is down".to_string());
        bad.approver = person("bob");
        let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
        assert_eq!(err, GateError::SeparationOfDutiesViolation);
    }

    #[test]
    fn decision_after_deadline_is_rejected_and_timeout_escalates() {
        let gate = gate();
        let mut late = input(GateDecision::Approve);
        late.decided_at_ms = 1_001;
        let err = evaluate_decision(&gate, 0, &late).unwrap_err();
        assert!(matches!(err, GateError::DecisionAfterTimeout { .. }));

        // Exactly at the deadline is still on time.
        late.decided_at_ms = 1_000;
        assert!(evaluate_decision(&gate, 0, &late).is_ok());

        // Timeout evaluation before the deadline fails closed.
        let err = evaluate_timeout(&gate, &person("bob"), 0, 1_000).unwrap_err();
        assert_eq!(err, GateError::NotTimedOut);

        let record = evaluate_timeout(&gate, &person("bob"), 0, 1_001).unwrap();
        assert_eq!(record.outcome, GateOutcome::Escalated);
        assert_eq!(record.decision, None);
        assert!(record.receipt.is_none());
        assert_eq!(
            record.escalation_policy_digest,
            gate.escalation_policy_digest
        );
        assert!(record.is_structurally_valid());
    }

    #[test]
    fn receipt_digest_is_deterministic_and_sensitive() {
        let gate = gate();
        let first = compute_receipt_digest(
            gate.id.get(),
            gate.run,
            GateDecision::Approve,
            &person("alice"),
            gate.evidence_bundle_digest,
            gate.prompt_digest,
            500,
        )
        .unwrap();
        let again = compute_receipt_digest(
            gate.id.get(),
            gate.run,
            GateDecision::Approve,
            &person("alice"),
            gate.evidence_bundle_digest,
            gate.prompt_digest,
            500,
        )
        .unwrap();
        assert_eq!(first, again);
        let changed_evidence = compute_receipt_digest(
            gate.id.get(),
            gate.run,
            GateDecision::Approve,
            &person("alice"),
            digest(99),
            gate.prompt_digest,
            500,
        )
        .unwrap();
        assert_ne!(first, changed_evidence);
        let changed_time = compute_receipt_digest(
            gate.id.get(),
            gate.run,
            GateDecision::Approve,
            &person("alice"),
            gate.evidence_bundle_digest,
            gate.prompt_digest,
            501,
        )
        .unwrap();
        assert_ne!(first, changed_time);
    }

    #[test]
    fn receipt_verification_fails_on_tampered_digest() {
        let gate = gate();
        let record = evaluate_decision(&gate, 0, &input(GateDecision::Approve)).unwrap();
        let mut receipt = record.receipt.unwrap();
        assert!(verify_receipt(&receipt, &gate).unwrap());
        receipt.digest = digest(77);
        assert!(!verify_receipt(&receipt, &gate).unwrap());
        receipt.digest = compute_receipt_digest(
            gate.id.get(),
            gate.run,
            GateDecision::Approve,
            &receipt.approver,
            digest(99),
            gate.prompt_digest,
            receipt.decided_at_ms,
        )
        .unwrap();
        // Digest recomputed over tampered evidence no longer matches gate binding.
        receipt.evidence_bundle_digest = digest(99);
        assert!(!verify_receipt(&receipt, &gate).unwrap());
    }

    #[test]
    fn replay_reconstructs_history_without_effects() {
        let gate = gate();
        let approved = evaluate_decision(&gate, 0, &input(GateDecision::Approve)).unwrap();
        let denied = evaluate_decision(&gate, 0, &input(GateDecision::Deny)).unwrap();
        let escalated = evaluate_timeout(&gate, &person("bob"), 0, 2_000).unwrap();

        let replay = replay_gate_outcome(&approved, &gate).unwrap();
        assert_eq!(replay.outcome, GateOutcome::Approved);
        assert_eq!(
            replay.receipt_digest,
            approved.receipt.as_ref().map(|r| r.digest)
        );
        assert!(!replay.escalated);

        let denied_replay = replay_gate_outcome(&denied, &gate).unwrap();
        assert_eq!(denied_replay.receipt_digest, None);

        let escalated_replay = replay_gate_outcome(&escalated, &gate).unwrap();
        assert!(escalated_replay.escalated);
        assert_eq!(escalated_replay.decision, None);

        // Replay rejects a record bound to different evidence.
        let mut tampered = approved.clone();
        tampered.evidence_bundle_digest = digest(99);
        let err = replay_gate_outcome(&tampered, &gate).unwrap_err();
        assert_eq!(err, GateError::RecordGateMismatch);

        // History replay over journal events preserves order and outcomes.
        let events = vec![
            gate_decision_event(gate.run, vb_storage::EventSeq::new(1), approved),
            gate_decision_event(gate.run, vb_storage::EventSeq::new(2), denied),
            gate_decision_event(gate.run, vb_storage::EventSeq::new(3), escalated),
        ];
        let history = replay_gate_history(&events).unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].outcome, GateOutcome::Approved);
        assert_eq!(history[1].outcome, GateOutcome::Denied);
        assert_eq!(history[2].outcome, GateOutcome::Escalated);
    }

    #[test]
    fn journal_event_round_trips_through_postcard() {
        let gate = gate();
        let record = evaluate_decision(&gate, 0, &input(GateDecision::Approve)).unwrap();
        let event = gate_decision_event(gate.run, vb_storage::EventSeq::new(1), record.clone());
        assert_eq!(event.run_id(), gate.run);
        assert_eq!(event.seq(), vb_storage::EventSeq::new(1));
        assert_eq!(event.record_kind(), vb_storage::RecordKind::GateDecision);
        let bytes = postcard::to_allocvec(&event).unwrap();
        let decoded: vb_storage::JournalEvent = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, event);
        // Replay from the decoded event reproduces the same outcome.
        let history = replay_gate_history(std::slice::from_ref(&decoded)).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].outcome, GateOutcome::Approved);
        assert_eq!(history[0].receipt_digest, record.receipt.map(|r| r.digest));
    }

    #[test]
    fn gate_opened_event_round_trips_through_postcard() {
        let gate = gate();
        let event = gate_opened_event(
            gate.clone(),
            100,
            person("bob"),
            vb_storage::EventSeq::new(1),
        );
        assert_eq!(event.run_id(), gate.run);
        assert_eq!(event.seq(), vb_storage::EventSeq::new(1));
        assert_eq!(event.record_kind(), vb_storage::RecordKind::GateOpened);
        let bytes = postcard::to_allocvec(&event).unwrap();
        let decoded: vb_storage::JournalEvent = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, event);
        let open = reconstruct_open_gates(std::slice::from_ref(&decoded)).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].gate, gate);
        assert_eq!(open[0].opened_at_ms, 100);
        assert_eq!(open[0].requester, person("bob"));
        assert_eq!(open[0].delegated_to, None);
    }

    #[test]
    fn reconstruct_open_gates_folds_open_and_close_in_order() {
        let gate = gate();
        let mut other = gate.clone();
        other.id = GateId::new(8);
        let opened =
            gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
        let other_opened = gate_opened_event(
            other.clone(),
            10,
            person("bob"),
            vb_storage::EventSeq::new(2),
        );
        let denied = gate_decision_event(
            gate.run,
            vb_storage::EventSeq::new(3),
            evaluate_decision(&gate, 0, &input(GateDecision::Deny)).unwrap(),
        );

        let open = reconstruct_open_gates(&[opened.clone(), other_opened.clone()]).unwrap();
        assert_eq!(open.len(), 2);
        assert_eq!(open[0].gate, gate);
        assert_eq!(open[1].gate, other);

        let open = reconstruct_open_gates(&[opened, other_opened, denied]).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].gate, other);

        for decision in [
            GateDecision::Approve,
            GateDecision::BreakGlass,
            GateDecision::Deny,
        ] {
            let mut decision_input = input(decision);
            if decision == GateDecision::BreakGlass {
                decision_input.break_glass_justification = Some("prod is down".to_string());
            }
            let opened =
                gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
            let decided = gate_decision_event(
                gate.run,
                vb_storage::EventSeq::new(2),
                evaluate_decision(&gate, 0, &decision_input).unwrap(),
            );
            assert!(
                reconstruct_open_gates(&[opened, decided])
                    .unwrap()
                    .is_empty(),
                "{decision:?} must close the gate"
            );
        }
        let opened =
            gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
        let escalated = gate_decision_event(
            gate.run,
            vb_storage::EventSeq::new(2),
            evaluate_timeout(&gate, &person("bob"), 0, 2_000).unwrap(),
        );
        assert!(
            reconstruct_open_gates(&[opened, escalated])
                .unwrap()
                .is_empty(),
            "escalation must close the gate"
        );
    }

    #[test]
    fn reconstruct_open_gates_keeps_gate_on_more_evidence_and_delegation() {
        let gate = gate();
        let opened =
            gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
        let more = gate_decision_event(
            gate.run,
            vb_storage::EventSeq::new(2),
            evaluate_decision(&gate, 0, &input(GateDecision::RequestMoreEvidence)).unwrap(),
        );
        let open = reconstruct_open_gates(&[opened.clone(), more]).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].delegated_to, None);

        let mut delegate_input = input(GateDecision::Delegate);
        delegate_input.delegated_to = Some(person("carol"));
        let delegated = gate_decision_event(
            gate.run,
            vb_storage::EventSeq::new(3),
            evaluate_decision(&gate, 0, &delegate_input).unwrap(),
        );
        let open = reconstruct_open_gates(&[opened, delegated]).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].delegated_to, Some(person("carol")));
    }

    #[test]
    fn reconstruct_open_gates_rejects_duplicate_open_and_invalid_gate() {
        let gate = gate();
        let opened =
            gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
        let again = gate_opened_event(gate.clone(), 5, person("bob"), vb_storage::EventSeq::new(2));
        let err = reconstruct_open_gates(&[opened, again]).unwrap_err();
        assert_eq!(err, GateError::ReplayDuplicateOpen { gate: 7 });

        let mut invalid = gate.clone();
        invalid.id = GateId::new(0);
        let bad = gate_opened_event(invalid, 0, person("bob"), vb_storage::EventSeq::new(1));
        let err = reconstruct_open_gates(std::slice::from_ref(&bad)).unwrap_err();
        assert_eq!(err, GateError::ReplayInvalidGate);

        let bad_requester = gate_opened_event(
            gate.clone(),
            0,
            ApproverId {
                value: String::new(),
            },
            vb_storage::EventSeq::new(1),
        );
        let err = reconstruct_open_gates(std::slice::from_ref(&bad_requester)).unwrap_err();
        assert_eq!(err, GateError::ReplayInvalidGate);
    }

    #[test]
    fn reconstruct_open_gates_ignores_decisions_for_unknown_gates() {
        let gate = gate();
        let denied = gate_decision_event(
            gate.run,
            vb_storage::EventSeq::new(1),
            evaluate_decision(&gate, 0, &input(GateDecision::Deny)).unwrap(),
        );
        assert!(
            reconstruct_open_gates(std::slice::from_ref(&denied))
                .unwrap()
                .is_empty(),
            "a decision for a gate opened before the window is not an error"
        );

        let opened =
            gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
        let denied = gate_decision_event(
            gate.run,
            vb_storage::EventSeq::new(2),
            evaluate_decision(&gate, 0, &input(GateDecision::Deny)).unwrap(),
        );
        let reopened = gate_opened_event(
            gate.clone(),
            3_000,
            person("bob"),
            vb_storage::EventSeq::new(3),
        );
        let open = reconstruct_open_gates(&[opened, denied, reopened]).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].opened_at_ms, 3_000);
    }

    #[test]
    fn reconstruct_open_gates_rejects_invalid_decision_record() {
        let gate = gate();
        let opened =
            gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
        let mut denied = evaluate_decision(&gate, 0, &input(GateDecision::Deny)).unwrap();
        denied.receipt = evaluate_decision(&gate, 0, &input(GateDecision::Approve))
            .unwrap()
            .receipt;
        assert!(!denied.is_structurally_valid());
        let bad = gate_decision_event(gate.run, vb_storage::EventSeq::new(2), denied);
        let err = reconstruct_open_gates(&[opened, bad]).unwrap_err();
        assert_eq!(err, GateError::InvalidRecord);
    }

    #[test]
    fn gate_id_mismatch_and_invalid_identities_are_rejected() {
        let gate = gate();
        let mut wrong_gate = input(GateDecision::Approve);
        wrong_gate.gate = GateId::new(8);
        let err = evaluate_decision(&gate, 0, &wrong_gate).unwrap_err();
        assert!(matches!(err, GateError::GateIdMismatch { .. }));

        let mut bad_approver = input(GateDecision::Approve);
        bad_approver.approver = ApproverId {
            value: String::new(),
        };
        let err = evaluate_decision(&gate, 0, &bad_approver).unwrap_err();
        assert_eq!(err, GateError::InvalidApprover);
    }
}
