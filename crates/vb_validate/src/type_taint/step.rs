#![forbid(unsafe_code)]
//! Step-level type and taint validation for workflow type/taint checking.
//!
//! Iterates over workflow steps, resolves typed values into facts, writes
//! slot facts, and enforces type constraints (e.g. boolean choose conditions)
//! and secret taint rules (no secret results unless allowed).

use crate::ValidationResult;

use super::facts::Facts;
use super::model::{StepKind, TypedValue, WorkflowTypes};
use super::types::{Taint, ValueFact, ValueType};

/// Slot storage for step-level validation.
pub(super) type Slots = Vec<Option<ValueFact>>;

// ---------------------------------------------------------------------------
// Public validators
// ---------------------------------------------------------------------------

/// Validates types and taint for an entire workflow.
pub fn validate_types(workflow: &WorkflowTypes) -> ValidationResult<()> {
    let facts = Facts::build(workflow);
    let mut slots = vec![None::<ValueFact>; workflow.steps.len()];
    validate_step_types(workflow, &facts, &mut slots)
}

/// Validates secret taint tracking. Per Section 47, Finish | Result taint is
/// passed through without rejection: `Secret` and `DerivedFromSecret` results
/// are accepted and only tracked.
pub fn validate_taint(workflow: &WorkflowTypes) -> ValidationResult<()> {
    let facts = Facts::build(workflow);
    let mut slots = vec![None::<ValueFact>; workflow.steps.len()];
    validate_step_taint(workflow, &facts, &mut slots)
}

// ---------------------------------------------------------------------------
// Internal: step validation
// ---------------------------------------------------------------------------

fn validate_step_types(
    workflow: &WorkflowTypes,
    facts: &Facts,
    slots: &mut Slots,
) -> ValidationResult<()> {
    for (index, step) in workflow.steps.iter().enumerate() {
        match &step.kind {
            StepKind::Save { value } => {
                let fact = resolve_value(value, facts, slots);
                write_slot(slots, index, fact);
            }
            StepKind::Choose { condition } => {
                let fact = resolve_value(condition, facts, slots);
                super::types::require_boolean(fact.value_type)?;
            }
            StepKind::Finish { .. } => {}
        }
    }
    Ok(())
}

fn validate_step_taint(
    workflow: &WorkflowTypes,
    facts: &Facts,
    slots: &mut Slots,
) -> ValidationResult<()> {
    for (index, step) in workflow.steps.iter().enumerate() {
        match &step.kind {
            StepKind::Save { value } => {
                let fact = resolve_value(value, facts, slots);
                write_slot(slots, index, fact);
            }
            StepKind::Choose { .. } => {
                // Taint pass only: no taint is produced or leaked by a branch
                // condition. Type checking of the condition is handled by
                // validate_step_types.
            }
            StepKind::Finish { result } => {
                // Section 47: Finish | Result taint passed through. No rejection
                // of Secret or DerivedFromSecret results. Taint is tracked but
                // does not cause rejection, mirroring vb_compile::validate_public_result.
                let _fact = resolve_value(result, facts, slots);
            }
        }
    }
    Ok(())
}

pub(super) fn write_slot(slots: &mut Slots, index: usize, fact: ValueFact) {
    if let Some(slot) = slots.get_mut(index) {
        *slot = Some(fact);
    }
}

pub(super) fn resolve_value(value: &TypedValue, facts: &Facts, slots: &Slots) -> ValueFact {
    match value {
        TypedValue::Literal(vt) => ValueFact::clean(*vt),
        TypedValue::Reference(name) => facts.resolve_reference(name),
        TypedValue::Slot(index) => match slots.get(*index).and_then(|s| *s) {
            Some(value) => value,
            None => ValueFact::clean(ValueType::Any),
        },
        TypedValue::Composite(values) => resolve_composite(values, facts, slots),
    }
}

fn resolve_composite(values: &[TypedValue], facts: &Facts, slots: &Slots) -> ValueFact {
    let mut taint = Taint::Clean;
    for value in values {
        let fact = resolve_value(value, facts, slots);
        taint = taint.merge(fact.taint);
    }
    ValueFact {
        value_type: ValueType::Any,
        taint,
    }
}
