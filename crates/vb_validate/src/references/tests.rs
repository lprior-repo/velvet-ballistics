// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
use crate::references::{
    RefTables, WorkflowRefs, parse_step_reference, validate_references, validate_single_reference,
    validate_single_reference_with_context,
};
#[cfg(test)]
use crate::ValidationError;

fn make_tables(inputs: &[&str], vars: &[&str], secrets: &[&str], step_ids: &[&str]) -> RefTables {
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
        step_ids: step_ids
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>(),
        step_ids_set: string_set(
            &step_ids
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        ),
    }
}

// Private helper used by tests
fn string_set(names: &[String]) -> std::collections::HashSet<String> {
    names.iter().cloned().collect()
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
fn step_reference_allowed_without_context() {
    // Step references are now allowed at workflow level (no context)
    let tables = make_tables(&[], &[], &[], &["build"]);
    assert_eq!(
        validate_single_reference("$steps.build.result", &tables),
        Ok(())
    );
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

// ---------------------------------------------------------------------------
// BDD exact-assertion tests
// ---------------------------------------------------------------------------

#[test]
fn validate_references_accepts_valid_forward_references() {
    // Given a workflow with declared inputs, vars, secrets
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
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_references_accepts_step_reference_at_workflow_level() {
    // Given a workflow referencing a declared step
    // Step references are now allowed at workflow level (no step context)
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        step_ids: vec!["build".to_owned()],
        references: vec!["$steps.build.result".to_owned()],
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns Ok (step reference allowed without context)
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_references_rejects_unknown_input_reference_exact() {
    // Given a workflow referencing an undeclared input
    let workflow = WorkflowRefs {
        inputs: vec!["user".to_owned()],
        vars: vec![],
        secrets: vec![],
        step_ids: vec![],
        references: vec!["$input.nonexistent".to_owned()],
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns UnknownReference with exact reference
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$input.nonexistent".to_owned(),
        })
    );
}

#[test]
fn validate_references_rejects_runtime_reference_exact() {
    // Given a workflow referencing $runtime.something
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        step_ids: vec![],
        references: vec!["$runtime.memory".to_owned()],
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns DirectRuntimeReference
    assert_eq!(result, Err(ValidationError::DirectRuntimeReference));
}

#[test]
fn validate_references_rejects_bare_now_exact() {
    // Given a workflow referencing $now
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        step_ids: vec![],
        references: vec!["$now".to_owned()],
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns DirectRuntimeReference
    assert_eq!(result, Err(ValidationError::DirectRuntimeReference));
}

#[test]
fn validate_references_rejects_unknown_root_exact() {
    // Given a workflow referencing an unknown root like $env.HOME
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        step_ids: vec![],
        references: vec!["$env.HOME".to_owned()],
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns UnknownReference with exact reference
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$env.HOME".to_owned(),
        })
    );
}

#[test]
fn validate_references_accepts_var_alias() {
    // Given a workflow with declared var and a reference using "var" (alias for "vars")
    let tables = make_tables(&[], &["count"], &[], &[]);
    // When validate_single_reference is called with "$var.count"
    let result = validate_single_reference("$var.count", &tables);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

// ---------------------------------------------------------------------------
// Adversarial BDD tests: validation bypass attacks
// ---------------------------------------------------------------------------

#[test]
fn adversarial_undeclared_secret_reference_is_rejected_as_unknown() {
    // Given a workflow referencing $secrets.api_key when "api_key" is NOT declared
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![], // No secrets declared!
        step_ids: vec!["done".to_owned()],
        references: vec!["$secrets.api_key".to_owned()],
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns UnknownReference (E0201) -- secret name not declared
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$secrets.api_key".to_owned(),
        })
    );
}

#[test]
fn adversarial_step_reference_allowed_at_workflow_level() {
    // Step references are now allowed at workflow level (no step context)
    // This is the intended new behavior for vb-xi2f.7
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        step_ids: vec!["build".to_owned()],
        references: vec!["$steps.build.result".to_owned()],
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns Ok (step reference allowed without context)
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_future_reference_to_nonexistent_step_is_rejected_as_unknown() {
    // Given a workflow referencing $steps.ghost where "ghost" is NOT declared
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        step_ids: vec!["build".to_owned()],
        references: vec!["$steps.ghost.output".to_owned()],
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns UnknownReference (E0201) -- step does not exist
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$steps.ghost.output".to_owned(),
        })
    );
}

#[test]
fn adversarial_runtime_reference_via_dollar_runtime_is_rejected() {
    // Given a workflow referencing $runtime.something
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        step_ids: vec![],
        references: vec!["$runtime.memory".to_owned()],
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns DirectRuntimeReference (E0204)
    assert_eq!(result, Err(ValidationError::DirectRuntimeReference));
}

#[test]
fn adversarial_bare_dollar_now_is_rejected_as_runtime() {
    // Given a workflow with a bare $now reference
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        step_ids: vec![],
        references: vec!["$now".to_owned()],
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns DirectRuntimeReference (E0204)
    assert_eq!(result, Err(ValidationError::DirectRuntimeReference));
}

#[test]
fn adversarial_bare_dollar_random_is_rejected_as_runtime() {
    // Given a workflow with a bare $random reference
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        step_ids: vec![],
        references: vec!["$random".to_owned()],
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns DirectRuntimeReference (E0204)
    assert_eq!(result, Err(ValidationError::DirectRuntimeReference));
}

#[test]
fn adversarial_unknown_root_dollar_env_is_rejected() {
    // Given a workflow referencing $env.HOME
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        step_ids: vec![],
        references: vec!["$env.HOME".to_owned()],
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns UnknownReference (E0201)
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$env.HOME".to_owned(),
        })
    );
}

#[test]
fn adversarial_mixed_valid_and_invalid_references_fails_on_first_invalid() {
    // Given a workflow with valid refs then an invalid one
    let workflow = WorkflowRefs {
        inputs: vec!["user".to_owned()],
        vars: vec!["count".to_owned()],
        secrets: vec!["token".to_owned()],
        step_ids: vec!["done".to_owned()],
        references: vec![
            "$input.user".to_owned(),
            "$vars.count".to_owned(),
            "$secrets.token".to_owned(),
            "$input.ghost".to_owned(), // invalid -- not declared
        ],
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns UnknownReference for the invalid one
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$input.ghost".to_owned(),
        })
    );
}

#[test]
fn adversarial_step_reference_without_field_allowed_without_context() {
    // Given a workflow referencing $steps.build (bare step, no dot suffix)
    // Without step context, step references are now allowed
    let tables = make_tables(&[], &[], &[], &["build"]);
    // When validate_single_reference is called with "$steps.build"
    let result = validate_single_reference("$steps.build", &tables);
    // Then "build" IS in step_ids, so Ok (allowed without context)
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_non_dollar_prefixed_reference_passes_silently() {
    // Given a non-dollar reference like "just_text"
    let tables = make_tables(&[], &[], &[], &[]);
    // When validate_single_reference is called
    let result = validate_single_reference("just_text", &tables);
    // Then it returns Ok -- non-dollar refs are not validated here
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_declared_secret_reference_is_accepted() {
    // Given a workflow where secret is properly declared and referenced
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec!["api_key".to_owned()],
        step_ids: vec!["done".to_owned()],
        references: vec!["$secrets.api_key".to_owned()],
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns Ok -- secret is declared
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_var_reference_using_vars_root_is_accepted() {
    // Given a declared var and reference using "$vars.count"
    let tables = make_tables(&[], &["count"], &[], &[]);
    // When validate_single_reference is called
    let result = validate_single_reference("$vars.count", &tables);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_var_reference_using_var_singular_root_is_accepted() {
    // Given a declared var and reference using "$var.count" (singular alias)
    let tables = make_tables(&[], &["count"], &[], &[]);
    // When validate_single_reference is called
    let result = validate_single_reference("$var.count", &tables);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_step_reference_to_nonexistent_step_without_dot_is_unknown() {
    // Given a step reference "$steps.ghost" where "ghost" is not declared
    let tables = make_tables(&[], &[], &[], &["build"]);
    // When validate_single_reference is called
    let result = validate_single_reference("$steps.ghost", &tables);
    // Then it returns UnknownReference (E0201) -- ghost not in step_ids
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$steps.ghost".to_owned(),
        })
    );
}

#[test]
fn adversarial_unknown_bare_dollar_word_is_rejected() {
    // Given a bare reference "$something" (no dot)
    let tables = make_tables(&[], &[], &[], &[]);
    // When validate_single_reference is called
    let result = validate_single_reference("$something", &tables);
    // Then it returns UnknownReference (E0201) -- not "now" or "random"
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$something".to_owned(),
        })
    );
}

// ---------------------------------------------------------------------------
// Tests for prior-step validation with context
// ---------------------------------------------------------------------------

#[test]
fn parse_step_reference_parses_valid_step_reference() {
    assert_eq!(
        parse_step_reference("$steps.build.output"),
        Some(("build", "output"))
    );
    assert_eq!(
        parse_step_reference("$step.build.output"),
        Some(("build", "output"))
    );
    assert_eq!(parse_step_reference("$steps.build"), None); // missing field
    assert_eq!(parse_step_reference("$input.user"), None); // not a step reference
    assert_eq!(parse_step_reference("steps.build.output"), None); // missing $
}

#[test]
fn prior_step_reference_allowed_with_context() {
    // Given step_ids ["step1", "step2", "step3"] and current step index 2
    let tables = make_tables(&[], &[], &[], &["step1", "step2", "step3"]);
    // When validating a reference to step1 (index 0) from step3 (index 2)
    let result = validate_single_reference_with_context("$steps.step1.output", &tables, Some(2));
    // Then it succeeds (prior step reference)
    assert_eq!(result, Ok(()));
}

#[test]
fn future_step_reference_rejected_with_context() {
    // Given step_ids ["step1", "step2", "step3"] and current step index 1
    let tables = make_tables(&[], &[], &[], &["step1", "step2", "step3"]);
    // When validating a reference to step3 (index 2) from step2 (index 1)
    let result = validate_single_reference_with_context("$steps.step3.output", &tables, Some(1));
    // Then it fails (future step reference)
    assert!(matches!(
        result,
        Err(ValidationError::FutureReference { .. })
    ));
}

#[test]
fn same_step_reference_rejected_with_context() {
    // Given step_ids ["step1", "step2", "step3"] and current step index 1
    let tables = make_tables(&[], &[], &[], &["step1", "step2", "step3"]);
    // When validating a reference to step2 (index 1) from step2 (index 1)
    let result = validate_single_reference_with_context("$steps.step2.output", &tables, Some(1));
    // Then it fails (same-step reference)
    assert!(matches!(
        result,
        Err(ValidationError::FutureReference { .. })
    ));
}

#[test]
fn step_reference_allowed_without_context_via_workflow_validation() {
    // Given step_ids ["build"]
    let tables = make_tables(&[], &[], &[], &["build"]);
    // When validating without step context via workflow validation
    let result = validate_single_reference("$steps.build.output", &tables);
    // Then it succeeds (no context means allow)
    assert_eq!(result, Ok(()));
}
