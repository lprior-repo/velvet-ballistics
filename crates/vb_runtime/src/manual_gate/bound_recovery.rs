//! Recover bound manual gates from durable record kinds 54/55.
//!
//! Recovery reconstructs only state carried by the journal. It never
//! synthesizes a decision or a receipt, and a malformed or mismatched stream
//! fails closed so a recovered run does not drive past an undecided gate.

use super::bound_receipt::{compute_bound_receipt_digest, verify_bound_receipt};
use super::errors::GateError;
use crate::shard::types::BoundOpenGate;
use indexmap::IndexMap;
use vb_core::manual_gate::GateOutcome;

/// Reconstructs open bound gates from journal events in order.
///
/// A decision whose opening is outside the recovered window is ignored:
/// its gate definition is not in the evidence. A duplicate open while a
/// gate is still open fails. A closing decision removes the gate so a
/// later open with the same id is a legitimate reuse.
pub(crate) fn reconstruct_bound_open_gates(
    events: &[vb_storage::JournalEvent],
) -> Result<Vec<BoundOpenGate>, GateError> {
    let mut state = BoundRecoveryState::default();
    for event in events {
        state.apply(event)?;
    }
    Ok(state.open.into_values().collect())
}

#[derive(Default)]
struct BoundRecoveryState {
    open: IndexMap<u64, BoundOpenGate>,
    seen_receipts: IndexMap<vb_core::manual_gate::GateDigest, u64>,
}

impl BoundRecoveryState {
    fn apply(&mut self, event: &vb_storage::JournalEvent) -> Result<(), GateError> {
        match event {
            vb_storage::JournalEvent::GateOpenedBoundEvent {
                gate,
                seq,
                opened_at_ms,
                requester,
                ..
            } => self.open_gate(gate, seq.get(), *opened_at_ms, requester),
            vb_storage::JournalEvent::GateDecisionBoundEvent { record, .. } => {
                self.apply_decision(record)
            }
            _ => Ok(()),
        }
    }

    fn open_gate(
        &mut self,
        gate: &vb_core::manual_gate::BoundManualGate,
        opening_seq: u64,
        opened_at_ms: u64,
        requester: &vb_core::manual_gate::ApproverId,
    ) -> Result<(), GateError> {
        if !gate.is_structurally_valid() || !requester.is_valid() {
            return Err(GateError::ReplayInvalidGate);
        }
        let gate_id = gate.gate.id.get();
        if self.open.contains_key(&gate_id) {
            return Err(GateError::ReplayDuplicateOpen { gate: gate_id });
        }
        self.open.insert(
            gate_id,
            BoundOpenGate {
                gate: gate.clone(),
                opened_at_ms,
                opening_seq,
                requester: requester.clone(),
                delegated_to: None,
            },
        );
        Ok(())
    }

    fn apply_decision(
        &mut self,
        record: &vb_core::manual_gate::BoundGateOutcomeRecord,
    ) -> Result<(), GateError> {
        if !record.is_structurally_valid() {
            return Err(GateError::InvalidRecord);
        }
        self.validate_receipt(record)?;
        let gate_id = record.record.gate.get();
        let Some(existing) = self.open.get(&gate_id) else {
            return Ok(());
        };
        validate_recovered_decision(record, existing, gate_id)?;
        apply_bound_outcome(&mut self.open, gate_id, record);
        Ok(())
    }

    fn validate_receipt(
        &mut self,
        record: &vb_core::manual_gate::BoundGateOutcomeRecord,
    ) -> Result<(), GateError> {
        let Some(receipt) = &record.bound_receipt else {
            return Ok(());
        };
        let gate_id = record.record.gate.get();
        if compute_bound_receipt_digest(receipt)? != receipt.digest {
            return Err(GateError::BoundReceiptMismatch);
        }
        if self.seen_receipts.insert(receipt.digest, gate_id).is_some() {
            return Err(GateError::ReplayDuplicateBoundReceipt { gate: gate_id });
        }
        Ok(())
    }
}

fn validate_recovered_decision(
    record: &vb_core::manual_gate::BoundGateOutcomeRecord,
    existing: &BoundOpenGate,
    gate_id: u64,
) -> Result<(), GateError> {
    if !record.matches_bound_gate(&existing.gate)
        || record.record.requester != existing.requester
        || record.record.opened_at_ms != existing.opened_at_ms
        || record.opening_seq != existing.opening_seq
        || record.record.decided_at_ms < existing.opened_at_ms
    {
        return Err(GateError::ReplayBoundGateMismatch { gate: gate_id });
    }
    if let Some(receipt) = &record.bound_receipt
        && !verify_bound_receipt(receipt, &existing.gate)?
    {
        return Err(GateError::BoundReceiptMismatch);
    }
    Ok(())
}

fn apply_bound_outcome(
    open: &mut IndexMap<u64, BoundOpenGate>,
    gate_id: u64,
    record: &vb_core::manual_gate::BoundGateOutcomeRecord,
) {
    match record.record.outcome {
        GateOutcome::Approved
        | GateOutcome::BreakGlassApproved
        | GateOutcome::Denied
        | GateOutcome::Escalated => {
            open.shift_remove(&gate_id);
        }
        GateOutcome::Delegated => {
            if let Some(gate) = open.get_mut(&gate_id) {
                gate.delegated_to = record.record.delegated_to.clone();
            }
        }
        GateOutcome::MoreEvidenceRequested => {}
    }
}
pub(super) fn validate_bound_replay(
    record: &vb_core::manual_gate::BoundGateOutcomeRecord,
    gate: &vb_core::manual_gate::BoundManualGate,
    requester: &vb_core::manual_gate::ApproverId,
    opened_at_ms: u64,
    opening_seq: u64,
    gate_id: u64,
) -> Result<(), GateError> {
    if !record.matches_bound_gate(gate)
        || record.record.requester != *requester
        || record.record.opened_at_ms != opened_at_ms
        || record.opening_seq != opening_seq
        || record.record.decided_at_ms < opened_at_ms
    {
        return Err(GateError::ReplayBoundGateMismatch { gate: gate_id });
    }
    if let Some(receipt) = &record.bound_receipt
        && !verify_bound_receipt(receipt, gate)?
    {
        return Err(GateError::BoundReceiptMismatch);
    }
    Ok(())
}

pub(super) fn bound_replay(
    record: &vb_core::manual_gate::BoundGateOutcomeRecord,
    bound_receipt_digest: Option<vb_core::manual_gate::GateDigest>,
) -> super::bound::BoundGateReplay {
    super::bound::BoundGateReplay {
        gate: record.record.gate.get(),
        run: record.record.run,
        outcome: record.record.outcome,
        decision: record.record.decision,
        bound_receipt_digest,
        escalated: record.record.outcome == GateOutcome::Escalated,
    }
}

pub(super) fn bound_outcome_is_terminal(outcome: GateOutcome) -> bool {
    matches!(
        outcome,
        GateOutcome::Approved
            | GateOutcome::BreakGlassApproved
            | GateOutcome::Denied
            | GateOutcome::Escalated
    )
}
