//! Basic reference validation tests.
//!
//! Unit tests for reference validation covering declared references,
//! unknown references, runtime references, and step references.

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
    fn accepts_declared_input_reference() {
        let tables = make_tables(&["user"], &[], &[], &[]);
        assert_eq!(validate_single_reference("$input.user", &tables), Ok(()));
    }

    #[test]
    fn rejects_unknown_input_reference() {
        let tables = make_tables(&["user"], &[], &[], &[]);
        assert!(matches!(
            validate_single_reference("$input.missing", &tables),
            Err(ValidationError::UnknownReference { .. })
        ));
    }

    #[test]
    fn accepts_declared_var_reference() {
        let tables = make_tables(&[], &["count"], &[], &[]);
        assert_eq!(validate_single_reference("$vars.count", &tables), Ok(()));
    }

    #[test]
    fn accepts_declared_secret_reference() {
        let tables = make_tables(&[], &[], &["token"], &[]);
        assert_eq!(validate_single_reference("$secrets.token", &tables), Ok(()));
    }

    #[test]
    fn rejects_runtime_reference() {
        let tables = make_tables(&[], &[], &[], &[]);
        assert!(matches!(
            validate_single_reference("$runtime.now", &tables),
            Err(ValidationError::DirectRuntimeReference)
        ));
    }

    #[test]
    fn rejects_bare_now_reference() {
        let tables = make_tables(&[], &[], &[], &[]);
        assert!(matches!(
            validate_single_reference("$now", &tables),
            Err(ValidationError::DirectRuntimeReference)
        ));
    }

    #[test]
    fn rejects_bare_random_reference() {
        let tables = make_tables(&[], &[], &[], &[]);
        assert!(matches!(
            validate_single_reference("$random", &tables),
            Err(ValidationError::DirectRuntimeReference)
        ));
    }

    #[test]
    fn rejects_unknown_root() {
        let tables = make_tables(&[], &[], &[], &[]);
        assert!(matches!(
            validate_single_reference("$env.HOME", &tables),
            Err(ValidationError::UnknownReference { .. })
        ));
    }

    #[test]
    fn rejects_step_reference_as_future() {
        let tables = make_tables(&[], &[], &[], &["build"]);
        assert!(matches!(
            validate_single_reference("$steps.build.result", &tables),
            Err(ValidationError::FutureReference { .. })
        ));
    }

    #[test]
    fn rejects_unknown_step_reference() {
        let tables = make_tables(&[], &[], &[], &["build"]);
        assert!(matches!(
            validate_single_reference("$steps.missing.result", &tables),
            Err(ValidationError::UnknownReference { .. })
        ));
    }

    #[test]
    fn full_validation_accepts_valid_workflow() {
        let workflow = WorkflowRefs {
            inputs: vec!["user".to_owned()],
            vars: vec!["count".to_owned()],
            secrets: vec!["token".to_owned()],
            step_ids: vec!["step1".to_owned(), "done".to_owned()],
            references: vec![
                "$input.user".to_owned(),
                "$vars.count".to_owned(),
                "$secrets.token".to_owned(),
            ],
        };
        assert_eq!(validate_references(&workflow), Ok(()));
    }

    #[test]
    fn full_validation_rejects_unknown_reference() {
        let workflow = WorkflowRefs {
            inputs: vec!["user".to_owned()],
            vars: vec![],
            secrets: vec![],
            step_ids: vec!["done".to_owned()],
            references: vec!["$input.user".to_owned(), "$input.missing".to_owned()],
        };
        assert!(matches!(
            validate_references(&workflow),
            Err(ValidationError::UnknownReference { .. })
        ));
    }
}
