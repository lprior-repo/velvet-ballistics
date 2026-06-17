#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::panic)]
// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
use crate::ValidationError;
#[cfg(test)]
use crate::references::{
    RefTables, WorkflowRefs, parse_step_reference, validate_references, validate_single_reference,
    validate_single_reference_in_on_error, validate_single_reference_in_repeat,
    validate_single_reference_with_context, validate_step_references,
};
#[cfg(test)]
use vb_core::ids::StepIdx;

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
    let inputs = owned_strings(inputs);
    let vars = owned_strings(vars);
    let secrets = owned_strings(secrets);
    let step_ids = owned_strings(step_ids);
    let loop_variable_names = owned_strings(loop_variable_names);
    RefTables::from_slices_with_loop_vars(&inputs, &vars, &secrets, &step_ids, &loop_variable_names)
}

/// Builds reference tables with both loop variable names AND step output
/// declarations populated. The `step_outputs` slice controls which step
/// IDs are considered output-producing; an empty slice means output tracking
/// is known and no step produces output.
fn make_tables_with_loop_vars_and_step_outputs(
    inputs: &[&str],
    vars: &[&str],
    secrets: &[&str],
    step_ids: &[&str],
    loop_variable_names: &[&str],
    step_outputs: &[&str],
) -> RefTables {
    let inputs = owned_strings(inputs);
    let vars = owned_strings(vars);
    let secrets = owned_strings(secrets);
    let step_ids = owned_strings(step_ids);
    let loop_variable_names = owned_strings(loop_variable_names);
    let step_outputs = owned_strings(step_outputs);
    RefTables::from_slices_with_outputs(
        &inputs,
        &vars,
        &secrets,
        &step_ids,
        &loop_variable_names,
        &step_outputs,
    )
}

fn owned_strings(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
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
    // Then it returns SecretNotDeclared -- secret name not declared
    assert_eq!(
        result,
        Err(ValidationError::SecretNotDeclared {
            secret: "api_key".to_owned(),
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
// Tests for direct step-ID roots (Master §8)
// ---------------------------------------------------------------------------
//
// A reference like `$build_result.output` uses a declared step ID as the root.
// The master contract lists `$step_id.x` as an allowed reference root, so the
// validator accepts it and applies the same prior-step and output checks as the
// `$steps.<step_id>.<field>` spelling.

#[test]
fn direct_step_id_root_is_accepted() {
    // Given step_ids ["build_result"] and a reference using the step ID
    // directly as the root
    let tables = make_tables(&[], &[], &[], &["build_result"]);
    // When validating the bare step reference
    let result = validate_single_reference("$build_result.output", &tables);
    // Then it is valid per Master §8.
    assert_eq!(result, Ok(()));
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
fn direct_step_id_root_via_full_validation_is_accepted() {
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
    // Then it is valid per Master §8.
    assert_eq!(result, Ok(()));
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
fn direct_step_id_root_obeys_prior_step_rule() {
    // Given step_ids ["first", "current"] and current step index 1.
    let tables = make_tables(&[], &[], &[], &["first", "current"]);
    // When validating a future/same-step direct-root reference.
    let result =
        validate_single_reference_with_context("$current.output", &tables, Some(1), false, false);
    // Then it is rejected by the same prior-step rule as `$steps.current.output`.
    assert!(matches!(
        result,
        Err(ValidationError::FutureReference { .. })
    ));
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
// vb-ref002: accept $loop_name root in loop scope
// ---------------------------------------------------------------------------
//
// Loop variable names (for_each, together, collect) become in-scope bindings
// inside the body. Master §8 lists `$loop_name.x` as an allowed root, so the
// validator accepts references whose root matches a known loop variable.

#[test]
fn direct_loop_variable_root_is_accepted() {
    // Given: `item` is the loop variable of an enclosing for_each.
    let tables = make_tables_with_loop_vars(&[], &[], &[], &[], &["item"]);
    // When: a body reference uses `$item.x` (no `$loop.` prefix).
    let result = validate_single_reference("$item.x", &tables);
    // Then: it is valid per Master §8.
    assert_eq!(result, Ok(()));
}

#[test]
fn direct_together_branch_loop_variable_root_is_accepted() {
    // Given: `branch` is the loop variable of an enclosing together.
    let tables = make_tables_with_loop_vars(&[], &[], &[], &[], &["branch"]);
    // When: a branch body reference uses `$branch.y` (no `$loop.` prefix).
    let result = validate_single_reference("$branch.y", &tables);
    // Then: a distinct loop variable name is also valid.
    assert_eq!(result, Ok(()));
}

#[test]
fn direct_loop_reference_is_not_classified_as_unknown() {
    // Given: `item` is a loop variable in scope.
    let tables = make_tables_with_loop_vars(&[], &[], &[], &[], &["item"]);
    // When: validating `$item.x`.
    let result = validate_single_reference("$item.x", &tables);
    // Then: it is not UnknownReference.
    assert!(!matches!(
        result,
        Err(ValidationError::UnknownReference { .. })
    ));
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
fn workflow_with_direct_loop_reference_in_body_is_accepted() {
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
    // Then: it is valid per Master §8.
    assert_eq!(result, Ok(()));
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
    // Then: it is valid. The context does not affect loop-variable routing.
    assert_eq!(result, Ok(()));
}

#[test]
fn direct_loop_variable_root_does_not_collide_with_reserved_namespace() {
    // Given: `item` is a loop variable; we do NOT have a `vars` named
    // `item` (so the `vars` namespace check is a no-op for this name).
    let tables = make_tables_with_loop_vars(&[], &[], &[], &[], &["item"]);
    // When: validating `$item` (bare, no dot).
    let result = validate_single_reference("$item", &tables);
    // Then: bare references to a loop variable name emit
    // DirectLoopReference (not UnknownReference), giving the user
    // a more specific diagnostic pointing at the direct variable usage.
    assert_eq!(
        result,
        Err(ValidationError::DirectLoopReference {
            variable: "item".to_owned(),
        })
    );
}

// ---------------------------------------------------------------------------
// Tests for `StepSkippedReference` (vb-ref005)
// ---------------------------------------------------------------------------
//
// When a step body carries a reference that the validator cannot
// resolve, the runtime would silently skip the step and continue
// with stale or default values. The validator surfaces that decision
// with a typed `StepSkippedReference` diagnostic so callers can fail
// the run or escalate the error instead of masking it. This section
// locks down the emission contract for the diagnostic.

#[test]
fn validate_step_references_emits_step_skipped_reference() {
    // Given: tables with at least one declared input. We deliberately
    // reference an undeclared input so the inner single-reference
    // check fails.
    let tables = make_tables(&["user"], &[], &[], &[]);
    // When: validate_step_references is called with a single broken
    // reference and a current step index of 0.
    let result =
        validate_step_references(StepIdx::new(0), &["$nonexistent.x".to_string()], &tables, 0);
    // Then: the validator surfaces the step-skip diagnostic with the
    // original reference text preserved for the user.
    assert_eq!(
        result,
        Err(ValidationError::StepSkippedReference {
            step: StepIdx::new(0),
            reference: "$nonexistent.x".to_owned().into_boxed_str(),
        })
    );
}

#[test]
fn validate_step_references_emits_step_skipped_reference_for_first_broken_reference() {
    // Given: a list of references, the first of which is broken and
    // the second is valid. The validator must report the first broken
    // reference and not inspect later ones (mirroring the documented
    // first-failure-wins behaviour).
    let tables = make_tables(&["user"], &[], &[], &[]);
    // When: validate_step_references is called with [broken, valid].
    let result = validate_step_references(
        StepIdx::new(0),
        &["$nonexistent.x".to_string(), "$input.user".to_string()],
        &tables,
        0,
    );
    // Then: the diagnostic reports the broken reference, not the valid one.
    assert_eq!(
        result,
        Err(ValidationError::StepSkippedReference {
            step: StepIdx::new(0),
            reference: "$nonexistent.x".to_owned().into_boxed_str(),
        })
    );
}

#[test]
fn validate_step_references_returns_ok_when_all_references_resolve() {
    // Given: a step with a single, resolvable input reference.
    let tables = make_tables(&["user"], &[], &[], &[]);
    // When: validate_step_references is called with a valid reference.
    let result =
        validate_step_references(StepIdx::new(0), &["$input.user".to_string()], &tables, 0);
    // Then: the validator returns Ok (no skip).
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_step_references_returns_ok_for_empty_reference_list() {
    // Given: a step with no references (e.g. an action step that
    // takes no `$`-prefixed arguments).
    let tables = make_tables(&[], &[], &[], &[]);
    // When: validate_step_references is called with an empty list.
    let result = validate_step_references(StepIdx::new(0), &[], &tables, 0);
    // Then: the validator returns Ok.
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_step_references_step_index_round_trips_through_diagnostic() {
    // Given: a step index that is non-zero, to ensure the diagnostic
    // records the exact index it received (not always 0).
    let tables = make_tables(&["user"], &[], &[], &[]);
    // When: validate_step_references is called with a broken reference
    // and step index 7.
    let result =
        validate_step_references(StepIdx::new(7), &["$nonexistent.x".to_string()], &tables, 0);
    // Then: the diagnostic records StepIdx::new(7), not StepIdx::new(0).
    assert_eq!(
        result,
        Err(ValidationError::StepSkippedReference {
            step: StepIdx::new(7),
            reference: "$nonexistent.x".to_owned().into_boxed_str(),
        })
    );
}

// ---------------------------------------------------------------------------
// Tests for `ResultReferenceMissing` (vb-ref006)
// ---------------------------------------------------------------------------
//
// A workflow may reference `$steps.<step_id>.output` to read the
// canonical "output" slot of a step. When the step does NOT produce
// an output (e.g. an action step that has no `output:` binding), the
// runtime would have no value to bind and the reference would
// silently resolve to absent data. The validator surfaces that
// decision with a typed `ResultReferenceMissing` diagnostic that
// names the producing step and the missing field symbol.
//
// Note: this diagnostic is only emitted when the workflow has supplied output
// tracking. `RefTables::from_slices_with_outputs(..., &[])` means tracking is
// known and no step produces output; `RefTables::from_slices` remains the
// permissive compatibility path for callers that have not wired tracking.

#[test]
fn result_reference_missing_emitted_when_step_has_no_output() {
    // Given: tables with a declared step "build" (index 0) and a
    // non-empty step-output set that explicitly EXCLUDES "build"
    // while including another step. The empty set is the
    // "permissive" default that treats every step as
    // output-producing; we must therefore populate the set with at
    // least one entry (so the membership check is exercised) and
    // ensure "build" is NOT in that set (so `step_has_output`
    // returns `false` for "build").
    let tables = make_tables_with_loop_vars_and_step_outputs(
        &[],
        &[],
        &[],
        &["build"],
        &[],
        &["other"], // "other" produces an output, "build" does NOT
    );
    // When: validate_rooted_reference is called directly with the
    // canonical "$steps.<id>.output" reference. The root is "steps"
    // and the tail is "build.output" so the function dispatches to
    // the step-reference arm, which then checks whether "build"
    // produces an output. current step index 1 is past "build"
    // (index 0) so the prior-step gate also passes.
    let result = super::validate_rooted_reference(
        "$steps.build.output",
        "steps",
        "build.output",
        &tables,
        Some(1),
        false,
        false,
    );
    // Then: the validator emits ResultReferenceMissing with the
    // correct producing step index and the sentinel output field
    // symbol.
    assert_eq!(
        result,
        Err(ValidationError::ResultReferenceMissing {
            step: StepIdx::new(0),
            missing_output: super::OUTPUT_FIELD_SYMBOL,
        })
    );
}

#[test]
fn result_reference_missing_not_emitted_when_step_has_output() {
    // Given: tables with a declared step "build" that IS in the
    // step-output set (so the validator knows "build" produces a
    // result output). The step_outputs list intentionally
    // excludes some OTHER step so the test exercises the membership
    // check, not the empty-set fallback.
    let tables = make_tables_with_loop_vars_and_step_outputs(
        &[],
        &[],
        &[],
        &["build", "other"],
        &[],
        &["build"], // only "build" produces an output
    );
    // When: validate_rooted_reference is called for $steps.build.output.
    let result = super::validate_rooted_reference(
        "$steps.build.output",
        "steps",
        "build.output",
        &tables,
        Some(1),
        false,
        false,
    );
    // Then: the validator returns Ok (no missing output).
    assert_eq!(result, Ok(()));
}

#[test]
fn result_reference_missing_not_emitted_when_output_tracking_is_not_supplied() {
    // Given: tables with a declared step "build" and no output-tracking
    // signal. This is the compatibility path that treats every step as
    // output-producing.
    let tables = make_tables(&[], &[], &[], &["build"]);
    // When: validate_rooted_reference is called for $steps.build.output.
    let result = super::validate_rooted_reference(
        "$steps.build.output",
        "steps",
        "build.output",
        &tables,
        Some(1),
        false,
        false,
    );
    // Then: the validator returns Ok (no missing output).
    assert_eq!(result, Ok(()));
}

#[test]
fn result_reference_missing_emitted_when_known_output_set_is_empty() {
    // Given: output tracking is supplied and the known output-producing set is
    // empty, meaning no step can satisfy `.output`.
    let tables = make_tables_with_loop_vars_and_step_outputs(&[], &[], &[], &["build"], &[], &[]);

    let result = super::validate_rooted_reference(
        "$steps.build.output",
        "steps",
        "build.output",
        &tables,
        Some(1),
        false,
        false,
    );

    assert_eq!(
        result,
        Err(ValidationError::ResultReferenceMissing {
            step: StepIdx::new(0),
            missing_output: super::OUTPUT_FIELD_SYMBOL,
        })
    );
}

#[test]
fn direct_result_reference_missing_emitted_when_known_output_set_is_empty() {
    // Given: direct `$step_id.output` uses the same output tracking as
    // `$steps.step_id.output`.
    let tables = make_tables_with_loop_vars_and_step_outputs(&[], &[], &[], &["build"], &[], &[]);

    let result = super::validate_rooted_reference(
        "$build.output",
        "build",
        "output",
        &tables,
        Some(1),
        false,
        false,
    );

    assert_eq!(
        result,
        Err(ValidationError::ResultReferenceMissing {
            step: StepIdx::new(0),
            missing_output: super::OUTPUT_FIELD_SYMBOL,
        })
    );
}

#[test]
fn result_reference_missing_emitted_for_correct_step_in_multi_step_workflow() {
    // Given: a multi-step workflow where step "build" is the FIRST
    // declared step (index 0) and does NOT produce an output, while
    // step "test" (index 1) DOES produce one.
    let tables = make_tables_with_loop_vars_and_step_outputs(
        &[],
        &[],
        &[],
        &["build", "test"],
        &[],
        &["test"], // only "test" produces an output
    );
    // When: validate_rooted_reference is called for $steps.build.output
    // with a current step index beyond "build" (so the prior-step
    // check would pass on its own).
    let result = super::validate_rooted_reference(
        "$steps.build.output",
        "steps",
        "build.output",
        &tables,
        Some(2),
        false,
        false,
    );
    // Then: the diagnostic names step index 0 (build), not 1 (test).
    assert_eq!(
        result,
        Err(ValidationError::ResultReferenceMissing {
            step: StepIdx::new(0),
            missing_output: super::OUTPUT_FIELD_SYMBOL,
        })
    );
}
