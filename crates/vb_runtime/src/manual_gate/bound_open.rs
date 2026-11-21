//! Single admission rule for opening a bound manual gate.
//!
//! Live open, replay, and recovery all call this, so a gate that the live
//! path rejects is also rejected when its journal is replayed.

use super::errors::GateError;
use vb_core::manual_gate::{
    ApproverId, BoundGateOutcomeRecord, BoundManualGate, GateOutcome, ManualGate,
};

pub(crate) fn check_bound_open(
    gate: &BoundManualGate,
    opened_at_ms: u64,
    requester: &ApproverId,
) -> Result<(), GateError> {
    if !gate.is_structurally_valid() {
        return Err(GateError::InvalidGate);
    }
    if !requester.is_valid() {
        return Err(GateError::InvalidRequester);
    }
    gate_deadline(&gate.gate, opened_at_ms)?;
    Ok(())
}

/// Whether a journaled bound decision was recorded against this exact opening.
pub(crate) fn decision_matches_opening(
    record: &BoundGateOutcomeRecord,
    requester: &ApproverId,
    opened_at_ms: u64,
    opening_seq: u64,
) -> bool {
    record.record.requester == *requester
        && record.record.opened_at_ms == opened_at_ms
        && record.opening_seq.get() == opening_seq
}

/// Deadline for a gate opened at `opened_at_ms`; overflow means the window is unrepresentable.
pub(crate) fn gate_deadline(gate: &ManualGate, opened_at_ms: u64) -> Result<u64, GateError> {
    gate.deadline_ms(opened_at_ms)
        .ok_or(GateError::DeadlineOverflow)
}

/// Admits a decision time against the gate window; shared by live evaluation and recovery.
pub(crate) fn check_decision_window(
    gate: &ManualGate,
    opened_at_ms: u64,
    decided_at_ms: u64,
) -> Result<(), GateError> {
    let deadline = gate_deadline(gate, opened_at_ms)?;
    if decided_at_ms < opened_at_ms {
        return Err(GateError::DecisionBeforeOpen {
            opened_at_ms,
            decided_at_ms,
        });
    }
    if decided_at_ms > deadline {
        return Err(GateError::DecisionAfterTimeout {
            deadline_ms: deadline,
            decided_at_ms,
        });
    }
    Ok(())
}

/// Live-only: a decision processed after the real deadline is rejected even if its stamped time is in-window.
pub(crate) fn check_live_processing_window(
    gate: &ManualGate,
    opened_at_ms: u64,
    processed_at_ms: u64,
) -> Result<(), GateError> {
    let deadline_ms = gate_deadline(gate, opened_at_ms)?;
    if processed_at_ms > deadline_ms {
        return Err(GateError::DecisionAfterTimeout {
            deadline_ms,
            decided_at_ms: processed_at_ms,
        });
    }
    Ok(())
}

/// Admits a timeout escalation time: strictly after the deadline, as live `evaluate_timeout` does.
pub(crate) fn check_escalation_window(
    gate: &ManualGate,
    opened_at_ms: u64,
    now_ms: u64,
) -> Result<(), GateError> {
    if now_ms <= gate_deadline(gate, opened_at_ms)? {
        return Err(GateError::NotTimedOut);
    }
    Ok(())
}

/// Admits a journaled outcome's timing by its kind: escalations are stamped at timeout,
/// every other outcome must fall inside the decision window.
pub(crate) fn check_recorded_window(
    gate: &ManualGate,
    opened_at_ms: u64,
    outcome: GateOutcome,
    decided_at_ms: u64,
) -> Result<(), GateError> {
    match outcome {
        GateOutcome::Escalated => check_escalation_window(gate, opened_at_ms, decided_at_ms),
        GateOutcome::Approved
        | GateOutcome::BreakGlassApproved
        | GateOutcome::Denied
        | GateOutcome::Delegated
        | GateOutcome::MoreEvidenceRequested => {
            check_decision_window(gate, opened_at_ms, decided_at_ms)
        }
    }
}

/// Admission rule for a historical bound decision against its opening.
///
/// Shared by the registered-executor authority check and journal replay so
/// both accept exactly the same decision records.
pub(crate) fn check_admitted_decision(
    record: &BoundGateOutcomeRecord,
    gate: &BoundManualGate,
    requester: &ApproverId,
    opened_at_ms: u64,
    opening_seq: u64,
) -> Result<(), GateError> {
    if !record.matches_bound_gate(gate)
        || !decision_matches_opening(record, requester, opened_at_ms, opening_seq)
    {
        return Err(GateError::ReplayBoundGateMismatch {
            gate: gate.gate.id.get(),
        });
    }
    check_recorded_window(
        &gate.gate,
        opened_at_ms,
        record.record.outcome,
        record.record.decided_at_ms,
    )
}
