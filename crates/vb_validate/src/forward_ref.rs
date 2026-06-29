#![forbid(unsafe_code)]
//! Adversarial reference validation tests.
//!
//! Tests that verify the reference validation system resists bypass attacks
//! through undeclared references, future references, runtime references, etc.

#[cfg(test)]
mod tests {
    use crate::ValidationError;
    use crate::ref_validate::{
        RefTables, WorkflowRefs, validate_references, validate_single_reference,
    };

    fn make_tables(
        inputs: &[&str],
        vars: &[&str],
        secrets: &[&str],
        step_ids: &[&str],
    ) -> RefTables {
        let input_names = inputs
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let var_names = vars.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        let secret_names = secrets
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let step_names = step_ids
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        RefTables::from_slices(&input_names, &var_names, &secret_names, &step_names)
    }

    #[test]
    fn adversarial_undeclared_secret_reference_is_rejected_as_unknown() {
        let workflow = WorkflowRefs {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            step_ids: vec!["done".to_owned()],
            references: vec!["$secrets.api_key".to_owned()],
        };
        let result = validate_references(&workflow);
        assert_eq!(
            result,
            Err(ValidationError::UnknownReference {
                reference: "$secrets.api_key".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_future_reference_to_existing_step_is_rejected() {
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
    fn adversarial_future_reference_to_nonexistent_step_is_rejected_as_unknown() {
        let workflow = WorkflowRefs {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            step_ids: vec!["build".to_owned()],
            references: vec!["$steps.ghost.output".to_owned()],
        };
        let result = validate_references(&workflow);
        assert_eq!(
            result,
            Err(ValidationError::UnknownReference {
                reference: "$steps.ghost.output".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_runtime_reference_via_dollar_runtime_is_rejected() {
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
    fn adversarial_bare_dollar_now_is_rejected_as_runtime() {
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
    fn adversarial_bare_dollar_random_is_rejected_as_runtime() {
        let workflow = WorkflowRefs {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            step_ids: vec![],
            references: vec!["$random".to_owned()],
        };
        let result = validate_references(&workflow);
        assert_eq!(result, Err(ValidationError::DirectRuntimeReference));
    }

    #[test]
    fn adversarial_unknown_root_dollar_env_is_rejected() {
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
    fn adversarial_mixed_valid_and_invalid_references_fails_on_first_invalid() {
        let workflow = WorkflowRefs {
            inputs: vec!["user".to_owned()],
            vars: vec!["count".to_owned()],
            secrets: vec!["token".to_owned()],
            step_ids: vec!["done".to_owned()],
            references: vec![
                "$input.user".to_owned(),
                "$vars.count".to_owned(),
                "$secrets.token".to_owned(),
                "$input.ghost".to_owned(),
            ],
        };
        let result = validate_references(&workflow);
        assert_eq!(
            result,
            Err(ValidationError::UnknownReference {
                reference: "$input.ghost".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_step_reference_to_existing_step_without_dot_suffix_is_rejected() {
        let tables = make_tables(&[], &[], &[], &["build"]);
        let result = validate_single_reference("$steps.build", &tables);
        assert_eq!(
            result,
            Err(ValidationError::FutureReference {
                reference: "$steps.build".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_non_dollar_prefixed_reference_passes_silently() {
        let tables = make_tables(&[], &[], &[], &[]);
        let result = validate_single_reference("just_text", &tables);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn adversarial_declared_secret_reference_is_accepted() {
        let workflow = WorkflowRefs {
            inputs: vec![],
            vars: vec![],
            secrets: vec!["api_key".to_owned()],
            step_ids: vec!["done".to_owned()],
            references: vec!["$secrets.api_key".to_owned()],
        };
        let result = validate_references(&workflow);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn adversarial_var_reference_using_vars_root_is_accepted() {
        let tables = make_tables(&[], &["count"], &[], &[]);
        let result = validate_single_reference("$vars.count", &tables);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn adversarial_var_reference_using_var_singular_root_is_accepted() {
        let tables = make_tables(&[], &["count"], &[], &[]);
        let result = validate_single_reference("$var.count", &tables);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn adversarial_step_reference_to_nonexistent_step_without_dot_is_unknown() {
        let tables = make_tables(&[], &[], &[], &["build"]);
        let result = validate_single_reference("$steps.ghost", &tables);
        assert_eq!(
            result,
            Err(ValidationError::UnknownReference {
                reference: "$steps.ghost".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_unknown_bare_dollar_word_is_rejected() {
        let tables = make_tables(&[], &[], &[], &[]);
        let result = validate_single_reference("$something", &tables);
        assert_eq!(
            result,
            Err(ValidationError::UnknownReference {
                reference: "$something".to_owned(),
            })
        );
    }
}
