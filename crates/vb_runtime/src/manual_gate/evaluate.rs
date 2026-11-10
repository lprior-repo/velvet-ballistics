use super::errors::GateError;
use super::receipt::compute_receipt_digest;
use vb_core::manual_gate::{
    ApprovalReceipt, ApproverId, GateDecision, GateDecisionInput, GateOutcome, GateOutcomeRecord,
    ManualGate, RiskClass,
};

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
