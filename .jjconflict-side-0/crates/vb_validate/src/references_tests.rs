#![forbid(unsafe_code)]
//! BDD exact-assertion tests for reference validation.
//!
//! These tests verify exact error type and message fidelity for
//! the reference validation system.

#[cfg(test)]
mod tests {
    use crate::ref_validate::{RefTables, WorkflowRefs, string_set, validate_references, validate_single_reference};
    use crate::ValidationError;

    fn make_tables(
        inputs: &[&str],
        vars: &[&str],
        secrets: &[&str],
        step_ids: &[&str],
    ) -> RefTables {
        RefTables {
            inputs: string_set(
                &inputs
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
            ),
            vars: string_set(&vars.iter().map(|s| s.to_string()).collect::<Vec<String>>()),
            secrets: string_set(
                &secrets
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
            ),
            step_ids: string_set(
                &step_ids
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
            ),
        }
    }

    #[test]
    fn validate_references_accepts_valid_forward_references() {
        let workflow = WorkflowRefs {
            inputs: vec!["user".to_owned()],
            vars: vec!["count".to_owned()],
            secrets: vec!["token".to_owned()],
            step_ids: vec!["done".to_owned()],
            references: vec![
                "$input.user".to_owned(),
                "$vars.count".to_owned(),
                "$secrets.token".to_owned(),
            ],
        };
        let result = validate_references(&workflow);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_references_rejects_backward_then_reference_exact() {
        let workflow = WorkflowRefs {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            step_ids: vec!["build".to_owned()],
            references: vec!["$steps.build.result".to_owned()],
        };
        let result = validate_references(&workflow);
        assert_eq!(
            result,
            Err(ValidationError::FutureReference {
                reference: "$steps.build.result".to_owned(),
            })
        );
    }

    #[test]
    fn validate_references_rejects_unknown_input_reference_exact() {
        let workflow = WorkflowRefs {
            inputs: vec!["user".to_owned()],
            vars: vec![],
            secrets: vec![],
            step_ids: vec![],
            references: vec!["$input.nonexistent".to_owned()],
        };
        let result = validate_references(&workflow);
        assert_eq!(
            result,
            Err(ValidationError::UnknownReference {
                reference: "$input.nonexistent".to_owned(),
            })
        );
    }

    #[test]
    fn validate_references_rejects_runtime_reference_exact() {
        let workflow = WorkflowRefs {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            step_ids: vec![],
            references: vec!["$runtime.memory".to_owned()],
        };
        let result = validate_references(&workflow);
        assert_eq!(result, Err(ValidationError::DirectRuntimeReference));
    }

    #[test]
    fn validate_references_rejects_bare_now_exact() {
        let workflow = WorkflowRefs {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            step_ids: vec![],
            references: vec!["$now".to_owned()],
        };
        let result = validate_references(&workflow);
        assert_eq!(result, Err(ValidationError::DirectRuntimeReference));
    }

    #[test]
    fn validate_references_rejects_unknown_root_exact() {
        let workflow = WorkflowRefs {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            step_ids: vec![],
            references: vec!["$env.HOME".to_owned()],
        };
        let result = validate_references(&workflow);
        assert_eq!(
            result,
            Err(ValidationError::UnknownReference {
                reference: "$env.HOME".to_owned(),
            })
        );
    }

    #[test]
    fn validate_references_accepts_var_alias() {
        let tables = make_tables(&[], &["count"], &[], &[]);
        let result = validate_single_reference("$var.count", &tables);
        assert_eq!(result, Ok(()));
    }
}
