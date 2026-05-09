#![forbid(unsafe_code)]
//! Type checking for workflow documents.

#![allow(unreachable_pub)]
//!
//! Validates that typed values satisfy step requirements (e.g., Choose
//! requires a boolean condition).

use crate::ValidationResult;

use crate::fact_table::{Facts, require_boolean, resolve_value, write_slot};
use crate::type_sigs::{StepKind, ValueFact, WorkflowTypes};

/// Validates types and taint for an entire workflow.
pub fn validate_types(workflow: &WorkflowTypes) -> ValidationResult<()> {
    let facts = Facts::build(workflow);
    let mut slots = vec![None::<ValueFact>; workflow.steps.len()];
    validate_step_types(workflow, &facts, &mut slots)
}

fn validate_step_types(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ValidationError;
    use crate::type_sigs::{StepTypes, TypedValue, ValueType};

    fn make_workflow(steps: Vec<StepTypes>) -> WorkflowTypes {
        WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps,
            resource_contract: crate::type_sigs::ResourceLimits::default(),
        }
    }

    fn save_step(id: &str, value: TypedValue) -> StepTypes {
        StepTypes {
            id: id.to_owned(),
            kind: StepKind::Save { value },
        }
    }

    fn choose_step(id: &str, condition: TypedValue) -> StepTypes {
        StepTypes {
            id: id.to_owned(),
            kind: StepKind::Choose { condition },
        }
    }

    // -- Pass cases --

    #[test]
    fn accepts_boolean_choose_literal() {
        let wf = make_workflow(vec![choose_step(
            "route",
            TypedValue::Literal(ValueType::Boolean),
        )]);
        assert_eq!(validate_types(&wf), Ok(()));
    }

    #[test]
    fn accepts_boolean_choose_via_slot() {
        let wf = make_workflow(vec![
            save_step("flag", TypedValue::Literal(ValueType::Boolean)),
            choose_step("route", TypedValue::Slot(0)),
        ]);
        assert_eq!(validate_types(&wf), Ok(()));
    }

    #[test]
    fn accepts_any_type_choose() {
        let wf = make_workflow(vec![
            save_step("val", TypedValue::Literal(ValueType::Any)),
            choose_step("route", TypedValue::Slot(0)),
        ]);
        assert_eq!(validate_types(&wf), Ok(()));
    }

    #[test]
    fn accepts_empty_workflow() {
        let wf = make_workflow(vec![]);
        assert_eq!(validate_types(&wf), Ok(()));
    }

    #[test]
    fn accepts_save_and_finish_no_choose() {
        let wf = make_workflow(vec![save_step(
            "val",
            TypedValue::Literal(ValueType::Number),
        )]);
        assert_eq!(validate_types(&wf), Ok(()));
    }

    // -- Fail cases --

    #[test]
    fn rejects_number_choose() {
        let wf = make_workflow(vec![
            save_step("flag", TypedValue::Literal(ValueType::Number)),
            choose_step("route", TypedValue::Slot(0)),
        ]);
        assert!(matches!(
            validate_types(&wf),
            Err(ValidationError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn rejects_text_choose() {
        let wf = make_workflow(vec![choose_step(
            "route",
            TypedValue::Literal(ValueType::Text),
        )]);
        assert_eq!(
            validate_types(&wf),
            Err(ValidationError::TypeMismatch {
                expected: "boolean".to_owned(),
                found: "text".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_null_choose() {
        let wf = make_workflow(vec![
            save_step("val", TypedValue::Literal(ValueType::Null)),
            choose_step("route", TypedValue::Slot(0)),
        ]);
        assert!(matches!(
            validate_types(&wf),
            Err(ValidationError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn rejects_object_choose() {
        let wf = make_workflow(vec![choose_step(
            "route",
            TypedValue::Literal(ValueType::Object),
        )]);
        assert_eq!(
            validate_types(&wf),
            Err(ValidationError::TypeMismatch {
                expected: "boolean".to_owned(),
                found: "object".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_list_choose() {
        let wf = make_workflow(vec![choose_step(
            "route",
            TypedValue::Literal(ValueType::List),
        )]);
        assert_eq!(
            validate_types(&wf),
            Err(ValidationError::TypeMismatch {
                expected: "boolean".to_owned(),
                found: "list".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_number_choose_exact_message() {
        let wf = make_workflow(vec![
            save_step("val", TypedValue::Literal(ValueType::Number)),
            choose_step("route", TypedValue::Slot(0)),
        ]);
        assert_eq!(
            validate_types(&wf),
            Err(ValidationError::TypeMismatch {
                expected: "boolean".to_owned(),
                found: "number".to_owned(),
            })
        );
    }
}
