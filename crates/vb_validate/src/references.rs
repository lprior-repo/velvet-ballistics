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
    step_ids: HashSet<String>,
}

impl RefTables {
    /// Builds reference tables from a [`WorkflowRefs`] document.
    pub fn build(workflow: &WorkflowRefs) -> Self {
        Self {
            inputs: string_set(&workflow.inputs),
            vars: string_set(&workflow.vars),
            secrets: string_set(&workflow.secrets),
            step_ids: string_set(&workflow.step_ids),
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
        Self {
            inputs: string_set(inputs),
            vars: string_set(vars),
            secrets: string_set(secrets),
            step_ids: string_set(step_ids),
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
        self.step_ids.contains(name)
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
    let Some(body) = reference.strip_prefix('$') else {
        return Ok(());
    };
    let Some((root, tail)) = body.split_once('.') else {
        return validate_bare_reference(reference, body);
    };
    validate_rooted_reference(reference, root, tail, tables)
}

fn validate_bare_reference(reference: &str, body: &str) -> ValidationResult<()> {
    if matches!(body, "now" | "random") {
        Err(ValidationError::DirectRuntimeReference)
    } else {
        Err(ValidationError::UnknownReference {
            reference: reference.to_owned(),
        })
    }
}

fn validate_rooted_reference(
    reference: &str,
    root: &str,
    tail: &str,
    tables: &RefTables,
) -> ValidationResult<()> {
    match root {
        "input" => validate_declared(reference, tail, "input", &tables.inputs),
        "var" | "vars" => validate_declared(reference, tail, "var", &tables.vars),
        "secrets" => validate_declared(reference, tail, "secrets", &tables.secrets),
        "runtime" => Err(ValidationError::DirectRuntimeReference),
        "step" | "steps" => validate_step_reference(reference, tail, tables),
        _ => Err(ValidationError::UnknownReference {
            reference: reference.to_owned(),
        }),
    }
}

fn validate_step_reference(
    reference: &str,
    tail: &str,
    tables: &RefTables,
) -> ValidationResult<()> {
    let name = reference_name(tail);
    if tables.step_ids.contains(name) {
        // Step references are always runtime-time, reject them.
        Err(ValidationError::FutureReference {
            reference: reference.to_owned(),
        })
    } else {
        Err(ValidationError::UnknownReference {
            reference: reference.to_owned(),
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
    fn validate_references_rejects_backward_then_reference_exact() {
        // Given a workflow referencing a declared step (future reference)
        let workflow = WorkflowRefs {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            step_ids: vec!["build".to_owned()],
            references: vec!["$steps.build.result".to_owned()],
        };
        // When validate_references is called
        let result = validate_references(&workflow);
        // Then it returns FutureReference with the exact reference
        assert_eq!(
            result,
            Err(ValidationError::FutureReference {
                reference: "$steps.build.result".to_owned(),
            })
        );
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
    fn adversarial_future_reference_to_existing_step_is_rejected() {
        // Given a workflow referencing $steps.build where "build" IS a declared step
        let workflow = WorkflowRefs {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            step_ids: vec!["build".to_owned()],
            references: vec!["$steps.build.result".to_owned()],
        };
        // When validate_references is called
        let result = validate_references(&workflow);
        // Then it returns FutureReference (E0202) -- step refs are runtime-time
        assert_eq!(
            result,
            Err(ValidationError::FutureReference {
                reference: "$steps.build.result".to_owned(),
            })
        );
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
    fn adversarial_step_reference_to_existing_step_without_dot_suffix_is_rejected() {
        // Given a workflow referencing $steps.build (bare step, no dot suffix)
        let tables = make_tables(&[], &[], &[], &["build"]);
        // When validate_single_reference is called with "$steps.build"
        let result = validate_single_reference("$steps.build", &tables);
        // Then "build" IS in step_ids, so FutureReference (E0202)
        assert_eq!(
            result,
            Err(ValidationError::FutureReference {
                reference: "$steps.build".to_owned(),
            })
        );
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
}
