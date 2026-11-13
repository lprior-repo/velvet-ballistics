use super::bound_receipt::{compute_bound_receipt_digest, verify_bound_receipt};
use super::errors::GateError;
use super::evaluate::{evaluate_decision, evaluate_timeout};
use indexmap::IndexMap;
use vb_core::ids::RunId;
use vb_core::manual_gate::{
    ApproverId, BoundApprovalReceipt, BoundGateOutcomeRecord, BoundManualGate, GateDecision,
    GateDigest, GateOutcome, OperatorTrust,
};

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
                approver_role: input.approver_role.clone(),
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
                if !gate.is_structurally_valid() || !requester.is_valid() {
                    return Err(GateError::ReplayInvalidGate);
                }
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
