#![forbid(unsafe_code)]
//! Reference validation for workflow documents.
//!
//! Builds reference tables from declared inputs, vars, secrets, and step IDs,
//! then validates that all `$input.*`, `$vars.*`, `$secrets.*`, and `$step.*`
//! references resolve to declared names. Rejects `$runtime.*`, `$now`, `$random`,
//! and direct step-result references.
//!
//! The [`RefTables`] type and [`validate_single_reference`] function are public
//! so that `vb_compile` can build tables from its AST and share the core
//! reference validation logic without duplication (DRIFT-5).

use crate::{ValidationError, ValidationResult};
use std::collections::HashSet;
use vb_core::span::Span;

/// Builds reference tables and validates all references in a workflow.
pub fn validate_references(workflow: &WorkflowRefs) -> ValidationResult<()> {
    let tables = RefTables::build(workflow);
    for reference in &workflow.references {
        validate_single_reference(reference, &tables)?;
    }
    Ok(())
}

/// Reference tables built from declared workflow names.
///
/// Public so that downstream crates (e.g. `vb_compile`) can build tables from
/// their own AST types and call [`validate_single_reference`] directly,
/// avoiding duplicate reference validation logic.
pub struct RefTables {
    inputs: HashSet<String>,
    vars: HashSet<String>,
    secrets: HashSet<String>,
    step_ids: Vec<String>,
    step_ids_set: HashSet<String>,
}

impl RefTables {
    /// Builds reference tables from a [`WorkflowRefs`] document.
    pub fn build(workflow: &WorkflowRefs) -> Self {
        let step_ids = workflow.step_ids.clone();
        let step_ids_set = string_set(&workflow.step_ids);
        Self {
            inputs: string_set(&workflow.inputs),
            vars: string_set(&workflow.vars),
            secrets: string_set(&workflow.secrets),
            step_ids,
            step_ids_set,
        }
    }

    /// Builds reference tables from individual name slices.
    ///
    /// This is the shared entry point used by `vb_compile` to avoid
    /// duplicating reference validation logic.
    pub fn from_slices(
        inputs: &[String],
        vars: &[String],
        secrets: &[String],
        step_ids: &[String],
    ) -> Self {
        let step_ids_vec = step_ids.to_vec();
        let step_ids_set = string_set(step_ids);
        Self {
            inputs: string_set(inputs),
            vars: string_set(vars),
            secrets: string_set(secrets),
            step_ids: step_ids_vec,
            step_ids_set,
        }
    }

    /// Returns whether the given name is a declared input.
    pub fn contains_input(&self, name: &str) -> bool {
        self.inputs.contains(name)
    }

    /// Returns whether the given name is a declared variable.
    pub fn contains_var(&self, name: &str) -> bool {
        self.vars.contains(name)
    }

    /// Returns whether the given name is a declared secret.
    pub fn contains_secret(&self, name: &str) -> bool {
        self.secrets.contains(name)
    }

    /// Returns whether the given name is a declared step ID.
    pub fn contains_step_id(&self, name: &str) -> bool {
        self.step_ids_set.contains(name)
    }

    /// Returns the index of the given step ID, or `None` if not found.
    pub fn step_index(&self, step_id: &str) -> Option<usize> {
        self.step_ids.iter().position(|id| id == step_id)
    }
}

fn string_set(names: &[String]) -> HashSet<String> {
    let mut set = HashSet::with_capacity(names.len());
    for name in names {
        set.insert(name.clone());
    }
    set
}

/// Validates a single `$`-prefixed reference against the declared name tables.
///
/// Returns `Ok(())` for non-`$` references (they are not validated here).
/// Returns an error for unknown roots, undeclared names, runtime references,
/// and step-result references.
pub fn validate_single_reference(reference: &str, tables: &RefTables) -> ValidationResult<()> {
    validate_single_reference_with_context(reference, tables, None)
}

/// Validates a single reference with optional step context.
///
/// When `current_step_index` is `Some(idx)`, step references are validated
/// against prior steps only (step_idx < idx). When `None`, step references
/// are allowed if the step ID exists (for workflow-level validation).
pub fn validate_single_reference_with_context(
    reference: &str,
    tables: &RefTables,
    current_step_index: Option<usize>,
) -> ValidationResult<()> {
    let Some(body) = reference.strip_prefix('$') else {
        return Ok(());
    };
    let Some((root, tail)) = body.split_once('.') else {
        return validate_bare_reference(reference, body);
    };
    validate_rooted_reference(reference, root, tail, tables, current_step_index)
}

fn validate_bare_reference(reference: &str, body: &str) -> ValidationResult<()> {
    if matches!(body, "now" | "random") {
        Err(ValidationError::DirectRuntimeReference { span: Span::ZERO })
    } else {
        Err(ValidationError::UnknownReference {
            reference: reference.to_owned(),
            span: Span::ZERO,
        })
    }
}

fn validate_rooted_reference(
    reference: &str,
    root: &str,
    tail: &str,
    tables: &RefTables,
    current_step_index: Option<usize>,
) -> ValidationResult<()> {
    match root {
        "input" => validate_declared(reference, tail, "input", &tables.inputs),
        "var" | "vars" => validate_declared(reference, tail, "var", &tables.vars),
        "secrets" => validate_declared(reference, tail, "secrets", &tables.secrets),
<<<<<<< HEAD
        "runtime" => Err(ValidationError::DirectRuntimeReference),
=======
        "runtime" => Err(ValidationError::DirectRuntimeReference { span: Span::ZERO }),
>>>>>>> landing/vb-xi2f.9
        "step" | "steps" => validate_step_reference(reference, tail, tables, current_step_index),
        _ => Err(ValidationError::UnknownReference {
            reference: reference.to_owned(),
            span: Span::ZERO,
        }),
    }
}

<<<<<<< HEAD
/// Parses a step reference of the form "$<step_id>.<field>" or "$steps.<step_id>.<field>".
=======
/// Parses a step reference of the form `$step_id.field` or `$steps.step_id.field`.
>>>>>>> landing/vb-xi2f.9
///
/// Returns `Some((step_id, field))` if the reference is a valid step reference,
/// or `None` if the reference is not a step reference.
pub fn parse_step_reference(reference: &str) -> Option<(&str, &str)> {
    let body = reference.strip_prefix('$')?;
    let (root, tail) = body.split_once('.')?;
    if !matches!(root, "step" | "steps") {
        return None;
    }
    let (step_id, field) = tail.split_once('.')?;
    Some((step_id, field))
}

fn validate_step_reference(
    reference: &str,
    tail: &str,
    tables: &RefTables,
    current_step_index: Option<usize>,
) -> ValidationResult<()> {
    let name = reference_name(tail);
    if let Some(step_idx) = tables.step_index(name) {
        // Step ID exists in the workflow.
        match current_step_index {
            Some(current_idx) => {
                if step_idx >= current_idx {
                    // Future or same-step reference - not allowed at runtime
                    Err(ValidationError::FutureReference {
                        reference: reference.to_owned(),
<<<<<<< HEAD
=======
                        span: Span::ZERO,
>>>>>>> landing/vb-xi2f.9
                    })
                } else {
                    // Prior step reference - allowed
                    Ok(())
                }
            }
            None => {
                // No step context (e.g., top-level references) - allow if step exists
                Ok(())
            }
        }
    } else {
        Err(ValidationError::UnknownReference {
            reference: reference.to_owned(),
            span: Span::ZERO,
        })
    }
}

fn validate_declared(
    reference: &str,
    tail: &str,
    _kind: &str,
    names: &HashSet<String>,
) -> ValidationResult<()> {
    let name = reference_name(tail);
    if names.contains(name) {
        Ok(())
    } else {
        Err(ValidationError::UnknownReference {
            reference: reference.to_owned(),
            span: Span::ZERO,
        })
    }
}

fn reference_name(tail: &str) -> &str {
    match tail.split_once('.') {
        Some((name, _)) => name,
        None => tail,
    }
}

// ---------------------------------------------------------------------------
// Workflow reference model
// ---------------------------------------------------------------------------

/// Workflow reference data used for reference validation.
#[derive(Debug, Clone, Default)]
pub struct WorkflowRefs {
    /// Declared input names.
    pub inputs: Vec<String>,
    /// Declared variable names.
    pub vars: Vec<String>,
    /// Declared secret names.
    pub secrets: Vec<String>,
    /// Declared step IDs (in order).
    pub step_ids: Vec<String>,
    /// All `$`-prefixed references found in the workflow.
    pub references: Vec<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
            Err(ValidationError::DirectRuntimeReference { span: Span::ZERO })
        ));
    }

    #[test]
    fn rejects_bare_now_reference() {
        let tables = make_tables(&[], &[], &[], &[]);
        assert!(matches!(
            validate_single_reference("$now", &tables),
            Err(ValidationError::DirectRuntimeReference { span: Span::ZERO })
        ));
    }

    #[test]
    fn rejects_bare_random_reference() {
        let tables = make_tables(&[], &[], &[], &[]);
        assert!(matches!(
            validate_single_reference("$random", &tables),
            Err(ValidationError::DirectRuntimeReference { span: Span::ZERO })
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
                span: Span::ZERO
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
        assert_eq!(
            result,
            Err(ValidationError::DirectRuntimeReference { span: Span::ZERO })
        );
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
        assert_eq!(
            result,
            Err(ValidationError::DirectRuntimeReference { span: Span::ZERO })
        );
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
                span: Span::ZERO
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
                span: Span::ZERO
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
                span: Span::ZERO
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
        assert_eq!(
            result,
            Err(ValidationError::DirectRuntimeReference { span: Span::ZERO })
        );
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
        assert_eq!(
            result,
            Err(ValidationError::DirectRuntimeReference { span: Span::ZERO })
        );
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
        assert_eq!(
            result,
            Err(ValidationError::DirectRuntimeReference { span: Span::ZERO })
        );
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
                span: Span::ZERO
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
                span: Span::ZERO
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
                span: Span::ZERO
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
                span: Span::ZERO
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
        let result =
            validate_single_reference_with_context("$steps.step1.output", &tables, Some(2));
        // Then it succeeds (prior step reference)
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn future_step_reference_rejected_with_context() {
        // Given step_ids ["step1", "step2", "step3"] and current step index 1
        let tables = make_tables(&[], &[], &[], &["step1", "step2", "step3"]);
        // When validating a reference to step3 (index 2) from step2 (index 1)
        let result =
            validate_single_reference_with_context("$steps.step3.output", &tables, Some(1));
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
        let result =
            validate_single_reference_with_context("$steps.step2.output", &tables, Some(1));
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
}
