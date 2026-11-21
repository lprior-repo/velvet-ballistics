// Bound manual gate submission flow (MASTER §77.15 additive path).
//
// Mirrors `gate_submission.rs` for the versioned bound representation. The
// legacy registry and the bound registry are disjoint: neither path can
// approve a gate opened through the other, so a legacy gate cannot silently
// gain binding authority and a bound gate cannot lose it.

include!("gate_submission_bound/open.rs");
include!("gate_submission_bound/decision.rs");
include!("gate_submission_bound/binding.rs");

fn binding_error(gate: &BoundManualGate, field: &'static str) -> RuntimeError {
    RuntimeError::GateEvaluationFailed {
        source: crate::manual_gate::GateError::BoundGateBindingMismatch {
            gate: gate.gate.id.get(),
            field,
        },
    }
}

pub(crate) fn validate_bound_open(
    gate: &BoundManualGate,
    opened_at_ms: u64,
    requester: &ApproverId,
) -> RuntimeResult<()> {
    crate::manual_gate::check_bound_open(gate, opened_at_ms, requester).map_err(|source| {
        match source {
            crate::manual_gate::GateError::InvalidGate
            | crate::manual_gate::GateError::InvalidRequester => {
                RuntimeError::InvalidGateOpenedRecord
            }
            source => RuntimeError::GateEvaluationFailed { source },
        }
    })
}
