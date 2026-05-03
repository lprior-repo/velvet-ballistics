//! Taint propagation validation for workflow documents.
//!
//! Tracks secret-derived data through steps and rejects any workflow
//! that would leak secrets into results (SECRET_RESULT_LEAK).

use crate::ValidationResult;

use super::fact_table::{resolve_value, write_slot, Facts};
use super::type_sigs::{StepKind, Taint, ValueFact, WorkflowTypes};

/// Validates secret taint tracking; rejects secret data leaking into results.
pub fn validate_taint(workflow: &WorkflowTypes) -> ValidationResult<()> {
    let facts = Facts::build(workflow);
    let mut slots = vec![None::<ValueFact>; workflow.steps.len()];
    validate_step_taint(workflow, &facts, &mut slots)
}

fn validate_step_taint(
    workflow: &WorkflowTypes,
    facts: &Facts,
    slots: &mut [Option<ValueFact>],
) -> ValidationResult<()> {
    for (index, step) in workflow.steps.iter().enumerate() {
        match &step.kind {
            StepKind::Save { value } => {
                let fact = resolve_value(value, facts, slots);
                write_slot(slots, index, fact);
            }
            StepKind::Choose { condition } => {
                let fact = resolve_value(condition, facts, slots);
                // Type checking for choose is done by validate_types
                // Here we only track taint; secret in a choose is allowed
                let _ = fact;
            }
            StepKind::Finish { result } => {
                let fact = resolve_value(result, facts, slots);
                if fact.taint == Taint::Secret {
                    return Err(crate::ValidationError::SecretResultLeak);
                }
            }
        }
    }
    Ok(())
}
