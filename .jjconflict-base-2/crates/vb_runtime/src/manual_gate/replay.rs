use super::errors::GateError;
use crate::shard::types::OpenGate;
use indexmap::IndexMap;
use vb_core::ids::RunId;
use vb_core::manual_gate::{
    ApproverId, GateDecision, GateDigest, GateOutcome, GateOutcomeRecord, ManualGate,
};

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
