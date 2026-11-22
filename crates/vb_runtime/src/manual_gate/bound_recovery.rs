//! Recover bound manual gates from durable record kinds 54/55.
//!
//! Recovery reconstructs only state carried by the journal. It never
//! synthesizes a decision or a receipt, and a malformed or mismatched stream
//! fails closed so a recovered run does not drive past an undecided gate.
//! Full-history replay and recovery share `BoundLedger`, so they admit and
//! reject the same records.

use super::bound::BoundGateReplay;
use super::bound_open::{check_admitted_decision, check_bound_open};
use super::bound_receipt::verify_bound_receipt;
use super::errors::GateError;
use crate::shard::types::BoundOpenGate;
use indexmap::IndexMap;
use std::collections::BTreeSet;
use vb_core::ids::RunId;
use vb_core::manual_gate::{
    ApproverId, BoundGateOutcomeRecord, BoundManualGate, GateDigest, GateOutcome,
};

/// Reconstructs open bound gates from journal events in order.
///
/// A decision whose opening is absent from the recovered stream fails
/// closed: its gate definition is not in the evidence. A duplicate open
/// while a gate is still open fails. A closing decision removes the gate so
/// a later open with the same id is a legitimate reuse.
pub(crate) fn reconstruct_bound_open_gates(
    events: &[vb_storage::JournalEvent],
) -> Result<Vec<BoundOpenGate>, GateError> {
    Ok(walk_bound_history(events, |_| {})?.into_open())
}

/// Admits every bound event in stream order through `BoundLedger`, calling
/// `on_decision` for each admitted decision. Full-history replay and recovery
/// both use this walk so they admit and reject the same records.
pub(super) fn walk_bound_history(
    events: &[vb_storage::JournalEvent],
    mut on_decision: impl FnMut(&BoundGateOutcomeRecord),
) -> Result<BoundLedger, GateError> {
    let mut ledger = BoundLedger::default();
    for event in events {
        match event {
            vb_storage::JournalEvent::GateOpenedBoundEvent {
                gate,
                seq,
                opened_at_ms,
                requester,
                ..
            } => ledger.open_gate(gate, seq.get(), *opened_at_ms, requester)?,
            vb_storage::JournalEvent::GateDecisionBoundEvent { record, .. } => {
                ledger.admit_decision(record)?;
                on_decision(record);
            }
            vb_storage::JournalEvent::RunFailedEvent { run, .. } => ledger.close_failed_run(*run),
            _ => {}
        }
    }
    Ok(ledger)
}

/// Open-gate and approval-receipt ledger shared by replay and recovery.
#[derive(Default)]
pub(super) struct BoundLedger {
    open: IndexMap<u64, BoundOpenGate>,
    closed: BTreeSet<u64>,
    seen_receipts: IndexMap<GateDigest, u64>,
}

impl BoundLedger {
    pub(super) fn open_gate(
        &mut self,
        gate: &BoundManualGate,
        opening_seq: u64,
        opened_at_ms: u64,
        requester: &ApproverId,
    ) -> Result<(), GateError> {
        check_bound_open(gate, opened_at_ms, requester)?;
        let gate_id = gate.gate.id.get();
        if self.open.contains_key(&gate_id) {
            return Err(GateError::ReplayDuplicateOpen { gate: gate_id });
        }
        self.closed.remove(&gate_id);
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

    pub(super) fn admit_decision(
        &mut self,
        record: &BoundGateOutcomeRecord,
    ) -> Result<(), GateError> {
        if !record.is_structurally_valid() {
            return Err(GateError::InvalidRecord);
        }
        let gate_id = record.record.gate.get();
        let Some(existing) = self.open.get(&gate_id) else {
            return Err(if self.closed.contains(&gate_id) {
                GateError::ReplayDecisionAfterClose { gate: gate_id }
            } else {
                GateError::ReplayDecisionWithoutOpenGate { gate: gate_id }
            });
        };
        validate_bound_replay(
            record,
            &existing.gate,
            &existing.requester,
            existing.opened_at_ms,
            existing.opening_seq,
        )?;
        if let Some(delegate) = &existing.delegated_to
            && record.record.outcome != GateOutcome::Escalated
            && record.record.approver.as_ref() != Some(delegate)
        {
            return Err(GateError::ReplayWrongDelegate { gate: gate_id });
        }
        if let Some(receipt) = &record.bound_receipt
            && self.seen_receipts.insert(receipt.digest, gate_id).is_some()
        {
            return Err(GateError::ReplayDuplicateBoundReceipt { gate: gate_id });
        }
        apply_bound_outcome(&mut self.open, gate_id, record);
        if !self.open.contains_key(&gate_id) {
            self.closed.insert(gate_id);
        }
        Ok(())
    }

    /// Closes every open bound gate of a failed run. The run's gates can no
    /// longer be decided, so later decisions for them are rejected as closed.
    pub(super) fn close_failed_run(&mut self, run: RunId) {
        let gate_ids: Vec<u64> = self
            .open
            .iter()
            .filter(|(_, open)| open.gate.gate.run == run)
            .map(|(gate_id, _)| *gate_id)
            .collect();
        for gate_id in gate_ids {
            self.open.shift_remove(&gate_id);
            self.closed.insert(gate_id);
        }
    }

    pub(super) fn into_open(self) -> Vec<BoundOpenGate> {
        self.open.into_values().collect()
    }
}

fn apply_bound_outcome(
    open: &mut IndexMap<u64, BoundOpenGate>,
    gate_id: u64,
    record: &BoundGateOutcomeRecord,
) {
    if bound_outcome_is_terminal(record.record.outcome) {
        open.shift_remove(&gate_id);
    } else if record.record.outcome == GateOutcome::Delegated
        && let Some(gate) = open.get_mut(&gate_id)
    {
        gate.delegated_to = record.record.delegated_to.clone();
    }
}

fn bound_outcome_is_terminal(outcome: GateOutcome) -> bool {
    matches!(
        outcome,
        GateOutcome::Approved
            | GateOutcome::BreakGlassApproved
            | GateOutcome::Denied
            | GateOutcome::Escalated
    )
}

fn validate_bound_replay(
    record: &BoundGateOutcomeRecord,
    gate: &BoundManualGate,
    requester: &ApproverId,
    opened_at_ms: u64,
    opening_seq: u64,
) -> Result<(), GateError> {
    check_admitted_decision(record, gate, requester, opened_at_ms, opening_seq)?;
    if let Some(receipt) = &record.bound_receipt
        && !verify_bound_receipt(receipt, gate)?
    {
        return Err(GateError::BoundReceiptMismatch);
    }
    Ok(())
}

pub(super) fn bound_replay(record: &BoundGateOutcomeRecord) -> BoundGateReplay {
    BoundGateReplay {
        gate: record.record.gate.get(),
        run: record.record.run,
        outcome: record.record.outcome,
        decision: record.record.decision,
        bound_receipt_digest: record.bound_receipt.as_ref().map(|receipt| receipt.digest),
        escalated: record.record.outcome == GateOutcome::Escalated,
    }
}
