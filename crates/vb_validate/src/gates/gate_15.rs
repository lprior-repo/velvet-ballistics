#![forbid(unsafe_code)]

use crate::ValidationResult;
use vb_core::workflow::WorkflowParts;

// ---------------------------------------------------------------------------
// Gate 15: Determinism proof
// ---------------------------------------------------------------------------

/// Validates that every suspension point (Do/Ask node) has a valid resume
/// target: either terminal (next == None) or a forward node index within
/// bounds. This is a bounded structural check using actual IR guarantees
/// (node index bounds and forward progression) without claiming durability
/// or replay proof.
///
/// Gate 15 (suspension resume): Do/Ask nodes are suspension points whose
/// completion events/tickets create separate boundaries. Adjacent suspension
/// points are valid — each suspends independently, receives its own completion
/// event, and resumes at its own target. The check validates that resume
/// targets are well-formed: within node count bounds and strictly forward
/// (no self or backward unmarked loop escape).
pub fn validate_gate_15_determinism_proof(parts: &WorkflowParts) -> ValidationResult<()> {
    crate::gate_15_resume::validate_resume_targets(parts)
}
