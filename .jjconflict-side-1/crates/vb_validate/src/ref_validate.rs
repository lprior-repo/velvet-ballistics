#![forbid(unsafe_code)]
//! Reference validation for workflow documents.
//!
//! Builds reference tables from declared inputs, vars, secrets, and step IDs,
//! then validates that all `$input.*`, `$vars.*`, `$secrets.*`, and `$step.*`
//! references resolve to declared names. Rejects `$runtime.*`, `$now`, `$random`,
//! and direct step-result references.
//!
//! This duplicate test module keeps the same reference-validation behavior as
//! the public `references` module while remaining crate-local under `cfg(test)`.

use crate::{ValidationError, ValidationResult};
use std::collections::HashSet;

/// Builds reference tables and validates all references in a workflow.
pub(crate) fn validate_references(workflow: &WorkflowRefs) -> ValidationResult<()> {
    let tables = RefTables::build(workflow);
    for reference in &workflow.references {
        validate_single_reference(reference, &tables)?;
    }
    Ok(())
}

/// Reference tables built from declared workflow names.
///
/// Crate-local duplicate of the public reference table used by test modules.
pub(crate) struct RefTables {
    pub(crate) inputs: HashSet<String>,
    pub(crate) vars: HashSet<String>,
    pub(crate) secrets: HashSet<String>,
    pub(crate) step_ids: HashSet<String>,
}

impl RefTables {
    /// Builds reference tables from a [`WorkflowRefs`] document.
    pub(crate) fn build(workflow: &WorkflowRefs) -> Self {
        Self {
            inputs: string_set(&workflow.inputs),
            vars: string_set(&workflow.vars),
            secrets: string_set(&workflow.secrets),
            step_ids: string_set(&workflow.step_ids),
        }
    }

    pub(crate) fn from_slices(
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

    pub(crate) fn contains_input(&self, name: &str) -> bool {
        self.inputs.contains(name)
    }

    pub(crate) fn contains_var(&self, name: &str) -> bool {
        self.vars.contains(name)
    }

    pub(crate) fn contains_secret(&self, name: &str) -> bool {
        self.secrets.contains(name)
    }

    pub(crate) fn contains_step_id(&self, name: &str) -> bool {
        self.step_ids.contains(name)
    }
}

/// Workflow reference data used for reference validation.
#[derive(Debug, Clone, Default)]
pub(crate) struct WorkflowRefs {
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

/// Validates a single `$`-prefixed reference against the declared name tables.
///
/// Returns `Ok(())` for non-`$` references (they are not validated here).
/// Returns an error for unknown roots, undeclared names, runtime references,
/// and step-result references.
pub(crate) fn validate_single_reference(
    reference: &str,
    tables: &RefTables,
) -> ValidationResult<()> {
    let Some(body) = reference.strip_prefix('$') else {
        return Ok(());
    };
    let Some((root, tail)) = body.split_once('.') else {
        return validate_bare_reference(reference, body);
    };
    validate_rooted_reference(reference, root, tail, tables)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

pub(crate) fn string_set(names: &[String]) -> HashSet<String> {
    let mut set = HashSet::with_capacity(names.len());
    for name in names {
        set.insert(name.clone());
    }
    set
}

fn reference_name(tail: &str) -> &str {
    match tail.split_once('.') {
        Some((name, _)) => name,
        None => tail,
    }
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
