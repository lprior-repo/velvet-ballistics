// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
use crate::ValidationError;
#[cfg(test)]
use crate::references::{
    RefTables, WorkflowRefs, parse_step_reference, validate_references, validate_single_reference,
    validate_single_reference_in_on_error, validate_single_reference_in_repeat,
    validate_single_reference_with_context,
};

fn make_tables(inputs: &[&str], vars: &[&str], secrets: &[&str], step_ids: &[&str]) -> RefTables {
    make_tables_with_loop_vars(inputs, vars, secrets, step_ids, &[])
}

fn make_tables_with_loop_vars(
    inputs: &[&str],
    vars: &[&str],
    secrets: &[&str],
    step_ids: &[&str],
    loop_variable_names: &[&str],
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
        loop_variable_names: string_set(
            &loop_variable_names
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
    let result = validate_single_reference_with_context(
        "$steps.step1.output",
        &tables,
        Some(2),
        false,
        false,
    );
    // Then it succeeds (prior step reference)
    assert_eq!(result, Ok(()));
}

#[test]
fn future_step_reference_rejected_with_context() {
    // Given step_ids ["step1", "step2", "step3"] and current step index 1
    let tables = make_tables(&[], &[], &[], &["step1", "step2", "step3"]);
    // When validating a reference to step3 (index 2) from step2 (index 1)
    let result = validate_single_reference_with_context(
        "$steps.step3.output",
        &tables,
        Some(1),
        false,
        false,
    );
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
    let result = validate_single_reference_with_context(
        "$steps.step2.output",
        &tables,
        Some(1),
        false,
        false,
    );
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

// ---------------------------------------------------------------------------
// Tests for direct step-ID root routing (vb-ref001)
// ---------------------------------------------------------------------------
//
// A reference like `$build_result.output` uses a declared step ID as the root
// instead of the required `$steps.<step_id>.<field>` prefix. The validator
// must surface `DirectStepReference` so the user can be told to add the
// `steps.` prefix, not the generic `UnknownReference`.

#[test]
fn direct_step_id_root_emits_direct_step_reference() {
    // Given step_ids ["build_result"] and a reference using the step ID
    // directly as the root
    let tables = make_tables(&[], &[], &[], &["build_result"]);
    // When validating the bare step reference
    let result = validate_single_reference("$build_result.output", &tables);
    // Then it returns DirectStepReference (NOT UnknownReference)
    assert_eq!(
        result,
        Err(ValidationError::DirectStepReference {
            step: "build_result".to_owned(),
        })
    );
}

#[test]
fn direct_step_id_root_is_not_classified_as_unknown() {
    // Given step_ids ["build_result"]
    let tables = make_tables(&[], &[], &[], &["build_result"]);
    // When validating the bare step reference
    let result = validate_single_reference("$build_result.output", &tables);
    // Then it is not UnknownReference
    assert!(!matches!(
        result,
        Err(ValidationError::UnknownReference { .. })
    ));
}

#[test]
fn direct_step_id_root_via_full_validation_emits_direct_step_reference() {
    // Given a workflow with a step ID and a reference using it as root
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        step_ids: vec!["build_result".to_owned()],
        references: vec!["$build_result.output".to_owned()],
        ..Default::default()
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns DirectStepReference with the step ID
    assert_eq!(
        result,
        Err(ValidationError::DirectStepReference {
            step: "build_result".to_owned(),
        })
    );
}

#[test]
fn direct_step_id_root_does_not_shadow_unknown_reference() {
    // Given step_ids ["build_result"] and a reference whose root is
    // neither a step ID nor any known namespace
    let tables = make_tables(&["user"], &[], &[], &["build_result"]);
    // When validating "$ghost.output" -- "ghost" is not declared anywhere
    let result = validate_single_reference("$ghost.output", &tables);
    // Then it still returns UnknownReference (regression check: routing
    // for step IDs must not steal the unknown-reference case)
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$ghost.output".to_owned(),
        })
    );
}

#[test]
fn direct_step_id_root_message_mentions_steps_prefix() {
    // Given step_ids ["build_result"]
    let tables = make_tables(&[], &[], &[], &["build_result"]);
    // When validating the bare step reference
    let result = validate_single_reference("$build_result.output", &tables);
    // Then it is DirectStepReference and its Display message includes
    // both "$steps" and the step ID
    let msg = match result {
        Err(ValidationError::DirectStepReference { step }) => {
            format!(
                "DIRECT_STEP_REFERENCE: step references must use the `$steps.X` prefix (found `${step}`)"
            )
        }
        other => panic!("expected DirectStepReference, got {other:?}"),
    };
    assert!(
        msg.contains("$steps"),
        "diagnostic message should mention the `$steps` prefix; got: {msg}"
    );
    assert!(
        msg.contains("build_result"),
        "diagnostic message should include the step ID; got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Tests for `$total` scope guard (vb-ref004)
// ---------------------------------------------------------------------------
//
// The `total` root is the runtime binding populated with the iteration
// count inside a `repeat` body. The validator must allow `$total.<field>`
// when the reference is validated from inside a `repeat` body and reject
// it with `ScopeGuardViolation` otherwise.

#[test]
fn total_root_inside_repeat_scope_is_accepted() {
    // Given tables with no declared step/var/input named "total"
    let tables = make_tables(&[], &[], &[], &[]);
    // When validating `$total.count` from inside a repeat body
    let result = validate_single_reference_in_repeat("$total.count", &tables);
    // Then it returns Ok (the runtime populates the binding)
    assert_eq!(result, Ok(()));
}

#[test]
fn total_root_outside_repeat_scope_is_scope_guard_violation() {
    // Given tables with no declared step/var/input named "total"
    let tables = make_tables(&[], &[], &[], &[]);
    // When validating `$total.count` at workflow level (no repeat scope)
    let result = validate_single_reference("$total.count", &tables);
    // Then it returns ScopeGuardViolation, NOT UnknownReference
    assert_eq!(
        result,
        Err(ValidationError::ScopeGuardViolation {
            reference: "$total.count".to_owned(),
            required_scope: "repeat".to_owned(),
        })
    );
}

#[test]
fn total_root_with_any_tail_inside_repeat_scope_is_accepted() {
    // The tail is runtime-defined (e.g. "count", "iterations"), so any
    // subpath is accepted inside the repeat scope.
    let tables = make_tables(&[], &[], &[], &[]);
    assert_eq!(
        validate_single_reference_in_repeat("$total.count", &tables),
        Ok(())
    );
    assert_eq!(
        validate_single_reference_in_repeat("$total.iterations", &tables),
        Ok(())
    );
    assert_eq!(
        validate_single_reference_in_repeat("$total.attempts_remaining", &tables),
        Ok(())
    );
}

#[test]
fn total_root_outside_repeat_scope_message_names_repeat_scope() {
    // Given tables with no declared step/var/input named "total"
    let tables = make_tables(&[], &[], &[], &[]);
    // When validating `$total.count` at workflow level
    let err = validate_single_reference("$total.count", &tables)
        .expect_err("should be ScopeGuardViolation");
    // Then the Display message mentions `repeat` (the required scope) so
    // the user knows where to put the reference
    let msg = err.to_string();
    assert!(
        msg.contains("repeat"),
        "scope guard message should mention the required `repeat` scope; got: {msg}"
    );
    assert!(
        msg.contains("$total.count"),
        "scope guard message should include the original reference; got: {msg}"
    );
}

#[test]
fn total_root_in_repeat_scope_does_not_shadow_unknown_references() {
    // The `total` literal match must NOT eat references like `$totally.x`
    // (root is `totally`, not `total`). Such references should still be
    // routed to UnknownReference. The literal `total` arm matches
    // exactly the byte string `total`, so `$totally.x` falls through.
    let tables = make_tables(&[], &[], &[], &[]);
    let result = validate_single_reference("$totally.x", &tables);
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$totally.x".to_owned(),
        })
    );
}

#[test]
fn total_root_in_repeat_scope_overrides_step_id_classification() {
    // If somehow a step is named "total" (unusual but defensive), the
    // literal `total` match in the repeat scope still wins over the
    // `contains_step_id` guard. The `total` arm is ordered before
    // `contains_step_id` so the runtime binding is honoured first.
    let tables = make_tables(&[], &[], &[], &["total"]);
    let result_in_repeat =
        validate_single_reference_with_context("$total.count", &tables, None, false, true);
    assert_eq!(result_in_repeat, Ok(()));
    // Outside the repeat scope, the literal `total` match still produces
    // ScopeGuardViolation (NOT DirectStepReference) because the `total`
    // arm is ordered before the step-id guard.
    let result_outside = validate_single_reference("$total.count", &tables);
    assert_eq!(
        result_outside,
        Err(ValidationError::ScopeGuardViolation {
            reference: "$total.count".to_owned(),
            required_scope: "repeat".to_owned(),
        })
    );
}

#[test]
fn total_root_via_full_validation_outside_repeat_scope_fails() {
    // Given a workflow referencing `$total.count` at workflow level
    // (no repeat scope)
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        step_ids: vec!["done".to_owned()],
        references: vec!["$total.count".to_owned()],
        ..Default::default()
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns ScopeGuardViolation (NOT UnknownReference)
    assert_eq!(
        result,
        Err(ValidationError::ScopeGuardViolation {
            reference: "$total.count".to_owned(),
            required_scope: "repeat".to_owned(),
        })
    );
}

#[test]
fn total_root_in_repeat_scope_with_on_error_flag_still_allowed() {
    // `$total` lives in the repeat scope, NOT the on_error scope. The
    // on_error flag must not unlock `$total.*`. A reference that is in
    // both scopes is hypothetical, but we lock down the orthogonal-flag
    // behaviour: `in_repeat_scope=true` allows `$total.*` regardless of
    // the on_error flag, and `in_on_error=true` does not.
    let tables = make_tables(&[], &[], &[], &[]);
    let with_on_error_only =
        validate_single_reference_with_context("$total.count", &tables, None, true, false);
    assert_eq!(
        with_on_error_only,
        Err(ValidationError::ScopeGuardViolation {
            reference: "$total.count".to_owned(),
            required_scope: "repeat".to_owned(),
        })
    );
    let with_repeat_only =
        validate_single_reference_with_context("$total.count", &tables, None, false, true);
    assert_eq!(with_repeat_only, Ok(()));
    let with_both =
        validate_single_reference_with_context("$total.count", &tables, None, true, true);
    assert_eq!(with_both, Ok(()));
}

#[test]
fn total_root_message_contains_repeat_and_reference_substrings() {
    // The Display impl of ScopeGuardViolation is what users will see.
    // Lock down the wording so diagnostics remain stable.
    let tables = make_tables(&[], &[], &[], &[]);
    let err = validate_single_reference("$total.count", &tables)
        .expect_err("should be ScopeGuardViolation");
    let msg = err.to_string();
    assert!(
        msg.contains("repeat"),
        "msg should mention `repeat`; got: {msg}"
    );
    assert!(
        msg.contains("$total.count"),
        "msg should include the original reference; got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Tests for `$error` scope guard (vb-ref003)
// ---------------------------------------------------------------------------
//
// The `error` root is the runtime binding populated when an action fails
// and is exposed only inside the body of an `on_error` handler. The
// validator must allow `$error.<field>` when the reference is validated
// from inside an `on_error` body, and reject it as `UnknownReference`
// (not `ScopeGuardViolation`) when the reference is at workflow level or
// inside any other scope. The bead spec asks for `UnknownReference` so the
// failure mode matches every other unrecognised root -- this keeps the
// diagnostic story simple and the master contract's "reserved roots"
// list is also the unknown-reference list until the scope check passes.

#[test]
fn error_root_inside_on_error_is_accepted() {
    // Given tables with no declared step/var/input named "error"
    let tables = make_tables(&[], &[], &[], &[]);
    // When validating `$error.message` from inside an on_error body
    let result = validate_single_reference_in_on_error("$error.message", &tables);
    // Then it returns Ok (the runtime populates the binding)
    assert_eq!(result, Ok(()));
}

#[test]
fn error_root_outside_on_error_is_unknown_reference() {
    // Given tables with no declared step/var/input named "error"
    let tables = make_tables(&[], &[], &[], &[]);
    // When validating `$error.message` at workflow level (no on_error scope)
    let result = validate_single_reference("$error.message", &tables);
    // Then it returns UnknownReference (NOT ScopeGuardViolation, NOT
    // DirectRuntimeReference). The bead spec says: a user who writes
    // `$error.message` outside an `on_error:` block hits the unknown
    // branch and sees the reference name in the error.
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$error.message".to_owned(),
        })
    );
}

#[test]
fn error_root_with_kind_tail_inside_on_error_is_accepted() {
    // `$error.kind` is the canonical second tail. Lock it down so the
    // acceptance path is not limited to `message`.
    let tables = make_tables(&[], &[], &[], &[]);
    let result = validate_single_reference_in_on_error("$error.kind", &tables);
    assert_eq!(result, Ok(()));
}

#[test]
fn error_root_with_any_tail_inside_on_error_is_accepted() {
    // The tail is runtime-defined (e.g. `message`, `kind`, `partial`),
    // so any subpath is accepted inside the on_error scope. The
    // validator must not over-constrain the action error shape.
    let tables = make_tables(&[], &[], &[], &[]);
    assert_eq!(
        validate_single_reference_in_on_error("$error.message", &tables),
        Ok(())
    );
    assert_eq!(
        validate_single_reference_in_on_error("$error.kind", &tables),
        Ok(())
    );
    assert_eq!(
        validate_single_reference_in_on_error("$error.partial", &tables),
        Ok(())
    );
    assert_eq!(
        validate_single_reference_in_on_error("$error.code", &tables),
        Ok(())
    );
}

#[test]
fn error_root_with_any_tail_outside_on_error_is_unknown_reference() {
    // Symmetry check: every tail that is accepted inside on_error must
    // be rejected with UnknownReference outside it.
    let tables = make_tables(&[], &[], &[], &[]);
    assert_eq!(
        validate_single_reference("$error.message", &tables),
        Err(ValidationError::UnknownReference {
            reference: "$error.message".to_owned(),
        })
    );
    assert_eq!(
        validate_single_reference("$error.kind", &tables),
        Err(ValidationError::UnknownReference {
            reference: "$error.kind".to_owned(),
        })
    );
    assert_eq!(
        validate_single_reference("$error.partial", &tables),
        Err(ValidationError::UnknownReference {
            reference: "$error.partial".to_owned(),
        })
    );
}

#[test]
fn error_root_inside_on_error_via_full_context_is_accepted() {
    // The `in_on_error` flag is the second-to-last positional bool. Pass
    // it via the explicit-context entry point so callers that already
    // have a context can opt into the on_error scope.
    let tables = make_tables(&[], &[], &[], &[]);
    let result =
        validate_single_reference_with_context("$error.message", &tables, None, true, false);
    assert_eq!(result, Ok(()));
}

#[test]
fn error_root_in_repeat_scope_does_not_unlock_it() {
    // Orthogonal-flag lock-down: the `in_repeat_scope` flag must NOT
    // unlock `$error.*`. The two scope flags are independent -- `$error`
    // belongs to `on_error` and `$total` belongs to `repeat`. Setting
    // `in_repeat_scope=true` while `in_on_error=false` must keep
    // `$error.*` rejected.
    let tables = make_tables(&[], &[], &[], &[]);
    let result =
        validate_single_reference_with_context("$error.message", &tables, None, false, true);
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$error.message".to_owned(),
        })
    );
}

#[test]
fn error_root_in_on_error_scope_does_not_unlock_total() {
    // Reverse orthogonal-flag lock-down: `in_on_error=true` must NOT
    // unlock `$total.*`. The two scope flags are independent and must
    // remain so.
    let tables = make_tables(&[], &[], &[], &[]);
    let result = validate_single_reference_with_context("$total.count", &tables, None, true, false);
    assert_eq!(
        result,
        Err(ValidationError::ScopeGuardViolation {
            reference: "$total.count".to_owned(),
            required_scope: "repeat".to_owned(),
        })
    );
}

#[test]
fn error_root_via_full_validation_outside_on_error_fails() {
    // Given a workflow referencing `$error.message` at workflow level
    // (no on_error scope)
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        step_ids: vec!["done".to_owned()],
        references: vec!["$error.message".to_owned()],
        ..Default::default()
    };
    // When validate_references is called
    let result = validate_references(&workflow);
    // Then it returns UnknownReference (NOT ScopeGuardViolation)
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$error.message".to_owned(),
        })
    );
}

#[test]
fn error_root_does_not_shadow_step_id_root() {
    // The literal `error` match must NOT eat references like `$errorcode`
    // (root is `errorcode`, not `error`). Such references should still
    // be routed to UnknownReference when outside the on_error scope.
    let tables = make_tables(&[], &[], &[], &[]);
    let result = validate_single_reference("$errorcode.x", &tables);
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$errorcode.x".to_owned(),
        })
    );
    // And inside the on_error scope, `$errorcode.x` is still rejected
    // because only the literal `error` root is allowed.
    let result_in = validate_single_reference_in_on_error("$errorcode.x", &tables);
    assert_eq!(
        result_in,
        Err(ValidationError::UnknownReference {
            reference: "$errorcode.x".to_owned(),
        })
    );
}

// ---------------------------------------------------------------------------
// vb-ref002: route $loop_name root to scope guard
// ---------------------------------------------------------------------------
//
// Loop variable names (for_each, together, collect) become in-scope bindings
// inside the body. The validator tracks them in `RefTables::loop_variable_names`
// and, when a reference root matches a known loop variable, emits
// `DirectLoopReference` instead of `UnknownReference`. The diagnostic message
// tells the user to use the `$loop.<var>` prefix.

#[test]
fn direct_loop_variable_root_emits_direct_loop_reference() {
    // Given: `item` is the loop variable of an enclosing for_each.
    let tables = make_tables_with_loop_vars(&[], &[], &[], &[], &["item"]);
    // When: a body reference uses `$item.x` (no `$loop.` prefix).
    let result = validate_single_reference("$item.x", &tables);
    // Then: it emits DirectLoopReference with the loop variable name,
    // not UnknownReference.
    assert_eq!(
        result,
        Err(ValidationError::DirectLoopReference {
            variable: "item".to_owned(),
        })
    );
}

#[test]
fn direct_together_branch_loop_variable_root_emits_direct_loop_reference() {
    // Given: `branch` is the loop variable of an enclosing together.
    let tables = make_tables_with_loop_vars(&[], &[], &[], &[], &["branch"]);
    // When: a branch body reference uses `$branch.y` (no `$loop.` prefix).
    let result = validate_single_reference("$branch.y", &tables);
    // Then: DirectLoopReference is emitted (regression check: a
    // distinct loop variable name produces the same diagnostic).
    assert_eq!(
        result,
        Err(ValidationError::DirectLoopReference {
            variable: "branch".to_owned(),
        })
    );
}

#[test]
fn direct_loop_reference_message_mentions_loop_prefix() {
    // Given: `item` is a loop variable in scope.
    let tables = make_tables_with_loop_vars(&[], &[], &[], &[], &["item"]);
    // When: validating `$item.x` and rendering the diagnostic.
    let err =
        validate_single_reference("$item.x", &tables).expect_err("should be DirectLoopReference");
    let diag = crate::diag_render::diagnostic_from_error(&err);
    // Then: the diagnostic message names the `$loop` prefix and the
    // variable, so the user knows how to fix the reference.
    let msg = diag.message.to_string();
    assert!(
        msg.contains("$loop"),
        "diagnostic message should mention the `$loop` prefix; got: {msg}"
    );
    assert!(
        msg.contains("item"),
        "diagnostic message should include the variable name; got: {msg}"
    );
}

#[test]
fn direct_loop_reference_does_not_shadow_unknown_reference() {
    // Given: `item` is a loop variable; `ghost` is NOT.
    let tables = make_tables_with_loop_vars(&[], &[], &[], &[], &["item"]);
    // When: validating `$ghost.x` (no loop variable nor any namespace).
    let result = validate_single_reference("$ghost.x", &tables);
    // Then: routing for loop variables must not steal the
    // unknown-reference case. (Regression check.)
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$ghost.x".to_owned(),
        })
    );
}

#[test]
fn direct_loop_reference_does_not_shadow_declared_var() {
    // Given: `count` is a declared var; `item` is a loop variable.
    let tables = make_tables_with_loop_vars(&[], &["count"], &[], &[], &["item"]);
    // When: validating `$vars.count` (a real, declared var).
    let result = validate_single_reference("$vars.count", &tables);
    // Then: it still resolves through the `vars` arm. The loop
    // variable arm must not steal namespace-rooted references.
    assert_eq!(result, Ok(()));
}

#[test]
fn direct_loop_reference_does_not_shadow_declared_input() {
    // Given: `user` is a declared input; `item` is a loop variable.
    let tables = make_tables_with_loop_vars(&["user"], &[], &[], &[], &["item"]);
    // When: validating `$input.user`.
    let result = validate_single_reference("$input.user", &tables);
    // Then: it resolves through the `input` arm.
    assert_eq!(result, Ok(()));
}

#[test]
fn workflow_with_direct_loop_reference_in_body_is_rejected() {
    // Given: a for_each body that uses `$item.x` directly.
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        step_ids: vec!["done".to_owned()],
        loop_variable_names: vec!["item".to_owned()],
        references: vec!["$item.x".to_owned()],
        ..Default::default()
    };
    // When: validate_references is called.
    let result = validate_references(&workflow);
    // Then: it returns DirectLoopReference for the body reference.
    assert_eq!(
        result,
        Err(ValidationError::DirectLoopReference {
            variable: "item".to_owned(),
        })
    );
}

#[test]
fn workflow_with_no_loop_variable_treats_loop_name_as_unknown() {
    // Given: `item` is NOT a loop variable (e.g., outside for_each scope).
    let workflow = WorkflowRefs {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        step_ids: vec!["done".to_owned()],
        loop_variable_names: vec![],
        references: vec!["$item.x".to_owned()],
        ..Default::default()
    };
    // When: validate_references is called.
    let result = validate_references(&workflow);
    // Then: outside for_each scope, `item` is unknown and the
    // reference is rejected as UnknownReference.
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$item.x".to_owned(),
        })
    );
}

#[test]
fn loop_variable_routing_with_context_matches_workflow_level() {
    // Given: `item` is a loop variable; we use the context-aware API.
    let tables = make_tables_with_loop_vars(&[], &[], &[], &[], &["item"]);
    // When: validating `$item.x` with current_step_index = Some(0)
    // (i.e., as if it were inside a for_each body at step 0).
    let result = validate_single_reference_with_context("$item.x", &tables, Some(0), false, false);
    // Then: it emits DirectLoopReference. The context does not
    // affect loop-variable routing.
    assert_eq!(
        result,
        Err(ValidationError::DirectLoopReference {
            variable: "item".to_owned(),
        })
    );
}

#[test]
fn direct_loop_variable_root_does_not_collide_with_reserved_namespace() {
    // Given: `item` is a loop variable; we do NOT have a `vars` named
    // `item` (so the `vars` namespace check is a no-op for this name).
    let tables = make_tables_with_loop_vars(&[], &[], &[], &[], &["item"]);
    // When: validating `$item` (bare, no dot).
    let result = validate_single_reference("$item", &tables);
    // Then: bare references are NOT routed to DirectLoopReference
    // because the routing lives in `validate_rooted_reference` (which
    // requires a dot to split root and tail). This documents the
    // scope: loop variable routing only triggers for `$<var>.<field>`
    // references, matching the master spec's `$loop_name.x` form.
    assert_eq!(
        result,
        Err(ValidationError::UnknownReference {
            reference: "$item".to_owned(),
        })
    );
}
