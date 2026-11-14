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
