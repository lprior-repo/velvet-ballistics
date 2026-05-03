//! Taint propagation validation for workflow documents.

#![allow(unreachable_pub)]
//!
//! Tracks secret-derived data through steps and rejects any workflow
//! that would leak secrets into results (SECRET_RESULT_LEAK).

use crate::ValidationResult;

use crate::fact_table::{resolve_value, write_slot, Facts};
use crate::type_sigs::{StepKind, Taint, ValueFact, WorkflowTypes};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type_sigs::{InputDecl, StepTypes, TypedValue, ValueType};
    use crate::ValidationError;

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

    fn finish_step(id: &str, result: TypedValue) -> StepTypes {
        StepTypes {
            id: id.to_owned(),
            kind: StepKind::Finish { result },
        }
    }

    // -- Pass cases --

    #[test]
    fn accepts_clean_finish() {
        let wf = make_workflow(vec![finish_step("done", TypedValue::Literal(ValueType::Number))]);
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    #[test]
    fn accepts_clean_input_finish() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$input.user".into()),
        )]);
        wf.inputs.push(InputDecl {
            name: "user".to_owned(),
            schema_type: ValueType::Text,
            is_secret: false,
        });
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    #[test]
    fn accepts_clean_var_finish() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$vars.label".into()),
        )]);
        wf.vars.push(("label".to_owned(), ValueType::Boolean));
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    #[test]
    fn accepts_secret_in_choose_not_leak() {
        let mut wf = make_workflow(vec![
            save_step("val", TypedValue::Reference("$secrets.token".into())),
            choose_step("route", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("token".to_owned());
        // Secret in a choose condition does not constitute a leak
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    #[test]
    fn accepts_empty_workflow() {
        let wf = make_workflow(vec![]);
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    #[test]
    fn accepts_save_clean_then_finish_slot() {
        let wf = make_workflow(vec![
            save_step("cap", TypedValue::Literal(ValueType::Number)),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    // -- Fail cases --

    #[test]
    fn rejects_secret_finish_direct() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$secrets.token".into()),
        )]);
        wf.secrets.push("token".to_owned());
        assert!(matches!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak)
        ));
    }

    #[test]
    fn rejects_secret_finish_via_slot() {
        let mut wf = make_workflow(vec![
            save_step("cap", TypedValue::Reference("$secrets.token".into())),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("token".to_owned());
        assert!(matches!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak)
        ));
    }

    #[test]
    fn rejects_secret_input_finish() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$input.key".into()),
        )]);
        wf.inputs.push(InputDecl {
            name: "key".to_owned(),
            schema_type: ValueType::Text,
            is_secret: true,
        });
        assert!(matches!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak)
        ));
    }

    #[test]
    fn rejects_two_step_secret_indirection() {
        let mut wf = make_workflow(vec![
            save_step("cap", TypedValue::Reference("$secrets.token".into())),
            save_step("relay", TypedValue::Slot(0)),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("token".to_owned());
        assert!(matches!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak)
        ));
    }

    #[test]
    fn rejects_nested_composite_secret() {
        let mut wf = make_workflow(vec![
            save_step(
                "cap",
                TypedValue::Composite(vec![TypedValue::Reference("$secrets.token".into())]),
            ),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("token".to_owned());
        assert!(matches!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak)
        ));
    }

    #[test]
    fn rejects_deeply_nested_composite_secret() {
        let mut wf = make_workflow(vec![
            save_step(
                "nested",
                TypedValue::Composite(vec![
                    TypedValue::Literal(ValueType::Number),
                    TypedValue::Composite(vec![
                        TypedValue::Literal(ValueType::Text),
                        TypedValue::Reference("$secrets.deep_secret".into()),
                    ]),
                ]),
            ),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("deep_secret".to_owned());
        assert!(matches!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak)
        ));
    }

    #[test]
    fn rejects_mixed_clean_secret_composite() {
        let mut wf = make_workflow(vec![
            save_step(
                "mixed",
                TypedValue::Composite(vec![
                    TypedValue::Literal(ValueType::Number),
                    TypedValue::Reference("$secrets.password".into()),
                ]),
            ),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("password".to_owned());
        assert!(matches!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak)
        ));
    }
}
