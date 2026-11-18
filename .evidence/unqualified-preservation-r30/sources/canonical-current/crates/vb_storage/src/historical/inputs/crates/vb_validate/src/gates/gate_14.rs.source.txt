#![forbid(unsafe_code)]

use crate::{ValidationError, ValidationResult};
use vb_core::workflow::WorkflowParts;

/// Checks executable assignment and known operand kinds through the core analysis.
pub fn validate_gate_14_slot_type_consistency(parts: &WorkflowParts) -> ValidationResult<()> {
    vb_core::workflow::validate_slot_flow(parts).map_err(|error| {
        if error.reason == vb_core::workflow::SlotFlowFailure::ConflictingConstantWrites
            && let Some(slot) = error.slot
        {
            return ValidationError::SlotTypeInconsistency {
                slot: slot.as_usize(),
            };
        }
        ValidationError::TypeMismatch {
            expected: "valid compiled slot flow".to_owned(),
            found: error.to_string(),
        }
    })
}
