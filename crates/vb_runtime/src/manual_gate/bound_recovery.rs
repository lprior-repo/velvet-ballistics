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
    let mut open: IndexMap<u64, BoundOpenGate> = IndexMap::new();
    let mut seen_receipts: IndexMap<vb_core::manual_gate::GateDigest, u64> = IndexMap::new();
    for event in events {
        match event {
            vb_storage::JournalEvent::GateOpenedBoundEvent {
                gate,
                seq,
                opened_at_ms,
                requester,
                ..
            } => {
                if !gate.is_structurally_valid() || !requester.is_valid() {
                    return Err(GateError::ReplayInvalidGate);
                }
                let gate_id = gate.gate.id.get();
                if open.contains_key(&gate_id) {
                    return Err(GateError::ReplayDuplicateOpen { gate: gate_id });
                }
                open.insert(
                    gate_id,
                    BoundOpenGate {
                        gate: gate.clone(),
                        opened_at_ms: *opened_at_ms,
                        opening_seq: seq.get(),
                        requester: requester.clone(),
                        delegated_to: None,
                    },
                );
            }
            vb_storage::JournalEvent::GateDecisionBoundEvent { record, .. } => {
                if !record.is_structurally_valid() {
                    return Err(GateError::InvalidRecord);
                }
                let gate_id = record.record.gate.get();
                if let Some(receipt) = &record.bound_receipt {
                    if compute_bound_receipt_digest(receipt)? != receipt.digest {
                        return Err(GateError::BoundReceiptMismatch);
                    }
                    if seen_receipts.insert(receipt.digest, gate_id).is_some() {
                        return Err(GateError::ReplayDuplicateBoundReceipt { gate: gate_id });
                    }
                }
                let Some(existing) = open.get(&gate_id) else {
                    continue;
                };
                if !record.matches_bound_gate(&existing.gate)
                    || record.record.requester != existing.requester
                    || record.record.opened_at_ms != existing.opened_at_ms
                    || record.opening_seq != existing.opening_seq
                    || record.record.decided_at_ms < existing.opened_at_ms
                {
                    return Err(GateError::ReplayBoundGateMismatch { gate: gate_id });
                }
                if let Some(receipt) = &record.bound_receipt {
                    if !verify_bound_receipt(receipt, &existing.gate)? {
                        return Err(GateError::BoundReceiptMismatch);
                    }
                }
                match record.record.outcome {
                    GateOutcome::Approved
                    | GateOutcome::BreakGlassApproved
                    | GateOutcome::Denied
                    | GateOutcome::Escalated => {
                        open.shift_remove(&gate_id);
                    }
                    GateOutcome::Delegated => {
                        if let Some(open_gate) = open.get_mut(&gate_id) {
                            open_gate.delegated_to = record.record.delegated_to.clone();
                        }
                    }
                    GateOutcome::MoreEvidenceRequested => {}
                }
            }
            _ => {}
        }
    }
    Ok(open.into_values().collect())
}
