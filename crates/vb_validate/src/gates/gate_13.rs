#![forbid(unsafe_code)]

use crate::ValidationResult;
use vb_core::workflow::WorkflowParts;

/// Rejects dependency cycles unless executable flow proves their reads initialized.
pub fn validate_gate_13_no_slot_cycles(parts: &WorkflowParts) -> ValidationResult<()> {
    crate::gate_13_cycles::validate_gate_13_no_slot_cycles(parts)
}
