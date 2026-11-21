use super::bound_receipt::compute_bound_receipt_digest;
use super::bound_recovery::{bound_replay, walk_bound_history};
use super::errors::GateError;
use super::evaluate::{evaluate_decision, evaluate_timeout};
use vb_core::ids::RunId;
use vb_core::manual_gate::{
    ApproverId, BoundApprovalReceipt, BoundGateOutcomeRecord, BoundManualGate, GateDecision,
    GateDigest, GateOutcome, OpeningSeq, OperatorTrust,
};

/// Evaluates a decision against a bound gate: the unchanged V1 evaluation
/// runs first, then the bound receipt and trust provenance are attached.
pub fn evaluate_bound_decision(
    gate: &BoundManualGate,
    opened_at_ms: u64,
    opening_seq: u64,
    requester: &ApproverId,
    operator_trust: OperatorTrust,
    input: &vb_core::manual_gate::GateDecisionInput,
) -> Result<BoundGateOutcomeRecord, GateError> {
    validate_bound_gate(gate)?;
    let record = evaluate_decision(&gate.gate, opened_at_ms, requester, input)?;
    let receipt = bound_receipt(gate, opening_seq, requester, operator_trust, input, &record)?;
    validate_bound_outcome(bound_outcome(
        gate,
        opening_seq,
        operator_trust,
        record,
        receipt,
    ))
}

fn validate_bound_gate(gate: &BoundManualGate) -> Result<(), GateError> {
    if !gate.is_structurally_valid() {
        return Err(GateError::InvalidGate);
    }
    Ok(())
}

fn bound_receipt(
    gate: &BoundManualGate,
    opening_seq: u64,
    requester: &ApproverId,
    operator_trust: OperatorTrust,
    input: &vb_core::manual_gate::GateDecisionInput,
    record: &vb_core::manual_gate::GateOutcomeRecord,
) -> Result<Option<BoundApprovalReceipt>, GateError> {
    let Some(receipt) = &record.receipt else {
        return Ok(None);
    };
    let mut bound = BoundApprovalReceipt {
        receipt: receipt.clone(),
        action: gate.gate.action,
        requester: requester.clone(),
        approver_role: input.approver_role.clone(),
        risk: gate.gate.risk,
        separation: gate.gate.separation_of_duties,
        binding: gate.binding,
        opening_seq: OpeningSeq::new(opening_seq),
        operator_trust,
        digest: GateDigest::ZERO,
    };
    bound.digest = compute_bound_receipt_digest(&bound)?;
    Ok(Some(bound))
}

fn bound_outcome(
    gate: &BoundManualGate,
    opening_seq: u64,
    operator_trust: OperatorTrust,
    record: vb_core::manual_gate::GateOutcomeRecord,
    bound_receipt: Option<BoundApprovalReceipt>,
) -> BoundGateOutcomeRecord {
    BoundGateOutcomeRecord {
        record,
        binding: gate.binding,
        opening_seq: OpeningSeq::new(opening_seq),
        operator_trust: Some(operator_trust),
        bound_receipt,
    }
}

fn validate_bound_outcome(
    outcome: BoundGateOutcomeRecord,
) -> Result<BoundGateOutcomeRecord, GateError> {
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
    opening_seq: u64,
    now_ms: u64,
) -> Result<BoundGateOutcomeRecord, GateError> {
    validate_bound_gate(gate)?;
    let record = evaluate_timeout(&gate.gate, requester, opened_at_ms, now_ms)?;
    validate_bound_outcome(BoundGateOutcomeRecord {
        record,
        binding: gate.binding,
        opening_seq: OpeningSeq::new(opening_seq),
        operator_trust: None,
        bound_receipt: None,
    })
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

/// Replays bound decision events only. Old 44 records are ignored here and
/// continue to replay through the legacy path without current-policy checks.
///
/// A decision with no durable opening in this stream is an
/// error: full history must account for every bound outcome it contains.
pub fn replay_bound_gate_history(
    events: &[vb_storage::JournalEvent],
) -> Result<Vec<BoundGateReplay>, GateError> {
    let mut replays = Vec::new();
    walk_bound_history(events, |record| replays.push(bound_replay(record)))?;
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
