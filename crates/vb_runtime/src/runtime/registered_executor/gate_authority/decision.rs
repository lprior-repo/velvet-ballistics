use super::{ExecutorError, GateOpening, RegisteredExecutionError};
use vb_core::manual_gate::{BoundGateOutcomeRecord, GateDecisionInput, GateOutcomeRecord};

pub(super) fn verify_policy(
    record: &BoundGateOutcomeRecord,
    opening: GateOpening<'_>,
) -> Result<(), RegisteredExecutionError> {
    let (gate, requester, opened_at_ms) = opening;
    let input = decision_input(&record.record)?;
    let trust = record.operator_trust.ok_or(ExecutorError::Unauthorized)?;
    let expected = crate::manual_gate::evaluate_bound_decision(
        gate,
        opened_at_ms,
        record.opening_seq,
        requester,
        trust,
        &input,
    )
    .map_err(|_| ExecutorError::Unauthorized)?;
    if &expected != record {
        return Err(ExecutorError::Unauthorized.into());
    }
    Ok(())
}

fn decision_input(
    record: &GateOutcomeRecord,
) -> Result<GateDecisionInput, RegisteredExecutionError> {
    Ok(GateDecisionInput {
        gate: record.gate,
        decision: record.decision.ok_or(ExecutorError::Unauthorized)?,
        approver: record.approver.clone().ok_or(ExecutorError::Unauthorized)?,
        approver_role: record
            .approver_role
            .clone()
            .ok_or(ExecutorError::Unauthorized)?,
        requester: record.requester.clone(),
        reason: record.reason.clone(),
        evidence_shown: record.evidence_shown,
        evidence_bundle_digest: record.evidence_bundle_digest,
        delegated_to: record.delegated_to.clone(),
        break_glass_justification: record.break_glass_justification.clone(),
        decided_at_ms: record.decided_at_ms,
    })
}
