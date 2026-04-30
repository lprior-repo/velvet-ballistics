//! Reference validation for workflow documents.
//!
//! Builds reference tables from declared inputs, vars, secrets, and step IDs,
//! then validates that all `$input.*`, `$vars.*`, `$secrets.*`, and `$step.*`
//! references resolve to declared names. Rejects `$runtime.*`, `$now`, `$random`,
//! and direct step-result references.

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
struct RefTables {
    inputs: HashSet<String>,
    vars: HashSet<String>,
    secrets: HashSet<String>,
    step_ids: HashSet<String>,
}

impl RefTables {
    fn build(workflow: &WorkflowRefs) -> Self {
        Self {
            inputs: string_set(&workflow.inputs),
            vars: string_set(&workflow.vars),
            secrets: string_set(&workflow.secrets),
            step_ids: string_set(&workflow.step_ids),
        }
    }
}

fn string_set(names: &[String]) -> HashSet<String> {
    let mut set = HashSet::with_capacity(names.len());
    for name in names {
        let _ = set.insert(name.clone());
    }
    set
}

fn validate_single_reference(reference: &str, tables: &RefTables) -> ValidationResult<()> {
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
            inputs: string_set(&inputs.iter().map(|s| s.to_string()).collect::<Vec<String>>()),
            vars: string_set(&vars.iter().map(|s| s.to_string()).collect::<Vec<String>>()),
            secrets: string_set(&secrets.iter().map(|s| s.to_string()).collect::<Vec<String>>()),
            step_ids: string_set(&step_ids.iter().map(|s| s.to_string()).collect::<Vec<String>>()),
        }
    }

    #[test]
    fn accepts_declared_input_reference() {
        let tables = make_tables(&["user"], &[], &[], &[]);
        assert!(validate_single_reference("$input.user", &tables).is_ok());
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
        assert!(validate_single_reference("$vars.count", &tables).is_ok());
    }

    #[test]
    fn accepts_declared_secret_reference() {
        let tables = make_tables(&[], &[], &["token"], &[]);
        assert!(validate_single_reference("$secrets.token", &tables).is_ok());
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
        assert!(validate_references(&workflow).is_ok());
    }

    #[test]
    fn full_validation_rejects_unknown_reference() {
        let workflow = WorkflowRefs {
            inputs: vec!["user".to_owned()],
            vars: vec![],
            secrets: vec![],
            step_ids: vec!["done".to_owned()],
            references: vec![
                "$input.user".to_owned(),
                "$input.missing".to_owned(),
            ],
        };
        assert!(matches!(
            validate_references(&workflow),
            Err(ValidationError::UnknownReference { .. })
        ));
    }
}
