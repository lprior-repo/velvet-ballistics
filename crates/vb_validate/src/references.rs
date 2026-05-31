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
    current_step_index: Option<usize>,
) -> ValidationResult<()> {
    match root {
        "input" => validate_declared(reference, tail, "input", &tables.inputs),
        "var" | "vars" => validate_declared(reference, tail, "var", &tables.vars),
        "secrets" => validate_declared(reference, tail, "secrets", &tables.secrets),
        "runtime" => Err(ValidationError::DirectRuntimeReference),
        "step" | "steps" => validate_step_reference(reference, tail, tables, current_step_index),
        _ => Err(ValidationError::UnknownReference {
            reference: reference.to_owned(),
        }),
    }
}

/// Parses a step reference of the form `$<step_id>.<field>` or
/// `$steps.<step_id>.<field>`.
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
#[cfg(test)]
#[path = "references/tests.rs"]
mod tests;
