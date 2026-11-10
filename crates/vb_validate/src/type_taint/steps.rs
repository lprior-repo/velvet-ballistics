use super::facts::Facts;
use super::{StepKind, Taint, TypedValue, ValueFact, ValueType, WorkflowTypes};
use crate::{ValidationError, ValidationResult};

// ---------------------------------------------------------------------------
// Internal: step validation
// ---------------------------------------------------------------------------

pub(crate) fn validate_step_types(
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
                require_boolean(fact.value_type)?;
            }
            StepKind::Finish { .. } => {}
        }
    }
    Ok(())
}

pub(crate) fn validate_step_taint(
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
            StepKind::Choose { .. } => {
                // Taint pass only: no taint is produced or leaked by a branch
                // condition. Type checking of the condition is handled by
                // validate_step_types.
            }
            StepKind::Finish { result } => {
                // Section 47: No rejection of Secret or DerivedFromSecret results
                // Taint is tracked but does not cause rejection in Finish
                let _fact = resolve_value(result, facts, slots);
            }
        }
    }
    Ok(())
}

fn write_slot(slots: &mut [Option<ValueFact>], index: usize, fact: ValueFact) {
    if let Some(slot) = slots.get_mut(index) {
        *slot = Some(fact);
    }
}

fn resolve_value(value: &TypedValue, facts: &Facts, slots: &[Option<ValueFact>]) -> ValueFact {
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

fn resolve_composite(
    values: &[TypedValue],
    facts: &Facts,
    slots: &[Option<ValueFact>],
) -> ValueFact {
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

fn require_boolean(actual: ValueType) -> ValidationResult<()> {
    if matches!(actual, ValueType::Boolean | ValueType::Any) {
        Ok(())
    } else {
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: actual.as_str().to_owned(),
        })
    }
}
