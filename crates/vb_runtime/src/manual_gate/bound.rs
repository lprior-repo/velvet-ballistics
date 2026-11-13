use super::errors::GateError;
use super::evaluate::{evaluate_decision, evaluate_timeout};
use super::receipt::verify_receipt;
use indexmap::IndexMap;
use vb_core::ids::RunId;
use vb_core::manual_gate::{
    ApproverId, BoundApprovalReceipt, BoundGateOutcomeRecord, BoundManualGate, GateDecision,
    GateDigest, GateOutcome, OperatorTrust, RiskClass, SeparationOfDuties,
};

/// Computes the V2 digest over every accountable bound-receipt field except
/// the digest itself. Domain separation is `vb-gate-receipt-v2`.
pub fn compute_bound_receipt_digest(
    receipt: &BoundApprovalReceipt,
) -> Result<GateDigest, GateError> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"vb-gate-receipt-v2");
    hasher.update(&receipt.receipt.gate.get().to_le_bytes());
    hasher.update(&receipt.receipt.run.get().to_le_bytes());
    hasher.update(&receipt.action.get().to_le_bytes());
    hasher.update(&[decision_byte(receipt.receipt.decision)]);
    hasher.update(&[risk_byte(receipt.risk)]);
    hasher.update(&[separation_byte(receipt.separation)]);
    update_text(&mut hasher, receipt.receipt.approver.value())?;
    update_text(&mut hasher, receipt.approver_role.name())?;
    update_text(&mut hasher, receipt.requester.value())?;
    hasher.update(&receipt.receipt.evidence_bundle_digest.as_bytes());
    hasher.update(&receipt.receipt.prompt_digest.as_bytes());
    hasher.update(&receipt.binding.artifact_digest.as_bytes());
    hasher.update(&receipt.binding.policy_digest.as_bytes());
    hasher.update(&receipt.binding.input_digest.as_bytes());
    hasher.update(&receipt.binding.attempt.to_le_bytes());
    hasher.update(&[trust_byte(receipt.operator_trust)]);
    hasher.update(&receipt.receipt.decided_at_ms.to_le_bytes());
    Ok(GateDigest::from_bytes(*hasher.finalize().as_bytes()))
}

fn update_text(hasher: &mut blake3::Hasher, text: &str) -> Result<(), GateError> {
    let len = u64::try_from(text.len()).map_err(|_| GateError::ReceiptDigestEncoding)?;
    hasher.update(&len.to_le_bytes());
    hasher.update(text.as_bytes());
    Ok(())
}

const fn decision_byte(decision: GateDecision) -> u8 {
    match decision {
        GateDecision::Approve => 0,
        GateDecision::Deny => 1,
        GateDecision::RequestMoreEvidence => 2,
        GateDecision::Delegate => 3,
        GateDecision::BreakGlass => 4,
    }
}

const fn risk_byte(risk: RiskClass) -> u8 {
    match risk {
        RiskClass::Low => 0,
        RiskClass::Medium => 1,
        RiskClass::High => 2,
    }
}

const fn separation_byte(separation: SeparationOfDuties) -> u8 {
    match separation {
        SeparationOfDuties::Disabled => 0,
        SeparationOfDuties::DistinctFromRequester => 1,
    }
}

const fn trust_byte(trust: OperatorTrust) -> u8 {
    match trust {
        OperatorTrust::LocalSocketClaim => 0,
        OperatorTrust::InProcessCaller => 1,
    }
}

/// Verifies a bound receipt against its bound gate: every bound field must
/// match the gate and the V2 digest must recompute. The embedded V1 receipt is
/// also verified so an old-digest tamper cannot hide under the new wrapper.
pub fn verify_bound_receipt(
    receipt: &BoundApprovalReceipt,
    gate: &BoundManualGate,
) -> Result<bool, GateError> {
    if !gate.is_structurally_valid()
        || receipt.receipt.gate != gate.gate.id
        || receipt.receipt.run != gate.gate.run
        || receipt.action != gate.gate.action
        || receipt.approver_role != gate.gate.required_role
        || receipt.risk != gate.gate.risk
        || receipt.separation != gate.gate.separation_of_duties
        || receipt.binding != gate.binding
        || receipt.receipt.evidence_bundle_digest != gate.gate.evidence_bundle_digest
        || receipt.receipt.prompt_digest != gate.gate.prompt_digest
        || !verify_receipt(&receipt.receipt, &gate.gate)?
    {
        return Ok(false);
    }
    Ok(compute_bound_receipt_digest(receipt)? == receipt.digest)
}

/// Evaluates a decision against a bound gate: the unchanged V1 evaluation
/// runs first, then the bound receipt and trust provenance are attached.
pub fn evaluate_bound_decision(
    gate: &BoundManualGate,
    opened_at_ms: u64,
    requester: &ApproverId,
    operator_trust: OperatorTrust,
    input: &vb_core::manual_gate::GateDecisionInput,
) -> Result<BoundGateOutcomeRecord, GateError> {
    if !gate.is_structurally_valid() {
        return Err(GateError::InvalidGate);
    }
    let record = evaluate_decision(&gate.gate, opened_at_ms, requester, input)?;
    let bound_receipt = match &record.receipt {
        Some(receipt) => {
            let mut bound = BoundApprovalReceipt {
                receipt: receipt.clone(),
                action: gate.gate.action,
                requester: requester.clone(),
                approver_role: gate.gate.required_role.clone(),
                risk: gate.gate.risk,
                separation: gate.gate.separation_of_duties,
                binding: gate.binding,
                operator_trust,
                digest: GateDigest::ZERO,
            };
            bound.digest = compute_bound_receipt_digest(&bound)?;
            Some(bound)
        }
        None => None,
    };
    let outcome = BoundGateOutcomeRecord {
        record,
        binding: gate.binding,
        operator_trust: Some(operator_trust),
        bound_receipt,
    };
    if !outcome.is_structurally_valid() {
        return Err(GateError::InvalidRecord);
    }
    Ok(outcome)
}

/// Produces the bound escalation record for a timed-out bound gate.
pub fn evaluate_bound_timeout(
    gate: &BoundManualGate,
    requester: &ApproverId,
    opened_at_ms: u64,
    now_ms: u64,
) -> Result<BoundGateOutcomeRecord, GateError> {
    if !gate.is_structurally_valid() {
        return Err(GateError::InvalidGate);
    }
    let record = evaluate_timeout(&gate.gate, requester, opened_at_ms, now_ms)?;
    let outcome = BoundGateOutcomeRecord {
        record,
        binding: gate.binding,
        operator_trust: None,
        bound_receipt: None,
    };
    if !outcome.is_structurally_valid() {
        return Err(GateError::InvalidRecord);
    }
    Ok(outcome)
}

/// Reconstructed bound outcome: replay output only, never a capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundGateReplay {
    /// Gate the replayed outcome belongs to.
    pub gate: u64,
    /// Run the replayed outcome belongs to.
    pub run: RunId,
    /// Replayed outcome.
    pub outcome: GateOutcome,
    /// Replayed operator decision, if any.
    pub decision: Option<GateDecision>,
    /// V2 receipt digest of the historical bound approval, if any.
    pub bound_receipt_digest: Option<GateDigest>,
    /// True when the replayed outcome is an escalation.
    pub escalated: bool,
}

/// Reconstructs one bound outcome against its bound gate.
pub fn replay_bound_gate_outcome(
    record: &BoundGateOutcomeRecord,
    gate: &BoundManualGate,
) -> Result<BoundGateReplay, GateError> {
    if !record.is_structurally_valid() || !record.matches_bound_gate(gate) {
        return Err(GateError::InvalidRecord);
    }
    let bound_receipt_digest = match &record.bound_receipt {
        Some(receipt) => {
            if !verify_bound_receipt(receipt, gate)? {
                return Err(GateError::BoundReceiptMismatch);
            }
            Some(receipt.digest)
        }
        None => None,
    };
    Ok(BoundGateReplay {
        gate: record.record.gate.get(),
        run: record.record.run,
        outcome: record.record.outcome,
        decision: record.record.decision,
        bound_receipt_digest,
        escalated: record.record.outcome == GateOutcome::Escalated,
    })
}

/// Replays bound decision events only. Old 44 records are ignored here and
/// continue to replay through the legacy path without current-policy checks.
pub fn replay_bound_gate_history(
    events: &[vb_storage::JournalEvent],
) -> Result<Vec<BoundGateReplay>, GateError> {
    let mut replays = Vec::new();
    let mut seen_receipts = IndexMap::new();
    let mut opened: IndexMap<u64, (BoundManualGate, ApproverId)> = IndexMap::new();
    for event in events {
        match event {
            vb_storage::JournalEvent::GateOpenedBoundEvent {
                gate, requester, ..
            } => {
                let gate_id = gate.gate.id.get();
                if opened
                    .insert(gate_id, (gate.clone(), requester.clone()))
                    .is_some()
                {
                    return Err(GateError::ReplayDuplicateOpen { gate: gate_id });
                }
            }
            vb_storage::JournalEvent::GateDecisionBoundEvent { record, .. } => {
                if !record.is_structurally_valid() {
                    return Err(GateError::InvalidRecord);
                }
                let gate_id = record.record.gate.get();
                let bound_receipt_digest = match &record.bound_receipt {
                    Some(receipt) => {
                        if compute_bound_receipt_digest(receipt)? != receipt.digest {
                            return Err(GateError::BoundReceiptMismatch);
                        }
                        if seen_receipts
                            .insert(receipt.digest, record.record.gate)
                            .is_some()
                        {
                            return Err(GateError::ReplayDuplicateBoundReceipt {
                                gate: record.record.gate.get(),
                            });
                        }
                        Some(receipt.digest)
                    }
                    None => None,
                };
                let Some((gate, requester)) = opened.get(&gate_id) else {
                    // The opening is outside the recovered window; history
                    // replay ignores the decision rather than guessing.
                    continue;
                };
                if !record.matches_bound_gate(gate) || record.record.requester != *requester {
                    return Err(GateError::ReplayBoundGateMismatch { gate: gate_id });
                }
                if let Some(receipt) = &record.bound_receipt
                    && !verify_bound_receipt(receipt, gate)?
                {
                    return Err(GateError::BoundReceiptMismatch);
                }
                replays.push(BoundGateReplay {
                    gate: record.record.gate.get(),
                    run: record.record.run,
                    outcome: record.record.outcome,
                    decision: record.record.decision,
                    bound_receipt_digest,
                    escalated: record.record.outcome == GateOutcome::Escalated,
                });
                if matches!(
                    record.record.outcome,
                    GateOutcome::Approved
                        | GateOutcome::BreakGlassApproved
                        | GateOutcome::Denied
                        | GateOutcome::Escalated
                ) {
                    opened.shift_remove(&gate_id);
                }
            }
            _ => {}
        }
    }
    Ok(replays)
}

/// Builds a bound gate-opened journal event.
pub fn bound_gate_opened_event(
    gate: BoundManualGate,
    opened_at_ms: u64,
    requester: ApproverId,
    seq: vb_storage::EventSeq,
) -> vb_storage::JournalEvent {
    vb_storage::JournalEvent::GateOpenedBoundEvent {
        run: gate.gate.run,
        seq,
        gate,
        opened_at_ms,
        requester,
    }
}

/// Builds a bound gate-decision journal event.
pub fn bound_gate_decision_event(
    run: RunId,
    seq: vb_storage::EventSeq,
    record: BoundGateOutcomeRecord,
) -> vb_storage::JournalEvent {
    vb_storage::JournalEvent::GateDecisionBoundEvent {
        run,
        seq,
        record: Box::new(record),
    }
}
