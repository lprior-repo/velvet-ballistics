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
use vb_core::ids::StepIdx;

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
    loop_variable_names: HashSet<String>,
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
            loop_variable_names: string_set(&workflow.loop_variable_names),
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
        Self::from_slices_with_loop_vars(inputs, vars, secrets, step_ids, &[])
    }

    /// Builds reference tables from individual name slices, including
    /// loop variable names that are in scope (for_each, together, collect).
    ///
    /// When a reference root matches a loop variable name, the validator
    /// emits `DirectLoopReference` instead of `UnknownReference`.
    pub fn from_slices_with_loop_vars(
        inputs: &[String],
        vars: &[String],
        secrets: &[String],
        step_ids: &[String],
        loop_variable_names: &[String],
    ) -> Self {
        let step_ids_vec = step_ids.to_vec();
        let step_ids_set = string_set(step_ids);
        Self {
            inputs: string_set(inputs),
            vars: string_set(vars),
            secrets: string_set(secrets),
            step_ids: step_ids_vec,
            step_ids_set,
            loop_variable_names: string_set(loop_variable_names),
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

    /// Returns whether the given name is a loop variable in scope.
    pub fn contains_loop_variable(&self, name: &str) -> bool {
        self.loop_variable_names.contains(name)
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
/// and step-result references. This entry point is not inside any
/// control-flow scope, so scope-bound references (e.g., `$error.*`,
/// `$total.*`) are rejected here. Use
/// [`validate_single_reference_in_on_error`] or
/// [`validate_single_reference_in_repeat`] when validating a reference that
/// lives inside the corresponding control-flow scope.
pub fn validate_single_reference(reference: &str, tables: &RefTables) -> ValidationResult<()> {
    validate_single_reference_with_context(reference, tables, None, false, false)
}

/// Validates a single reference with optional step and scope context.
///
/// When `current_step_index` is `Some(idx)`, step references are validated
/// against prior steps only (step_idx < idx). When `None`, step references
/// are allowed if the step ID exists (for workflow-level validation).
///
/// `in_on_error` marks the reference as living inside an `on_error` handler
/// body, which permits the `$error.*` scope binding per the language spec.
///
/// `in_repeat_scope` marks the reference as living inside a `repeat` body,
/// which permits the `$total.*` scope binding per the language spec. Using
/// `$total.*` outside a `repeat` body produces
/// [`ValidationError::ScopeGuardViolation`].
pub fn validate_single_reference_with_context(
    reference: &str,
    tables: &RefTables,
    current_step_index: Option<usize>,
    in_on_error: bool,
    in_repeat_scope: bool,
) -> ValidationResult<()> {
    let Some(body) = reference.strip_prefix('$') else {
        return Ok(());
    };
    let Some((root, tail)) = body.split_once('.') else {
        return validate_bare_reference(reference, body);
    };
    validate_rooted_reference(
        reference,
        root,
        tail,
        tables,
        current_step_index,
        in_on_error,
        in_repeat_scope,
    )
}

/// Validates a single reference that lives inside an `on_error` handler body.
///
/// The `$error` root is the runtime binding populated when an action fails
/// and is only legal inside the on_error scope. The tail is not statically
/// known (it depends on the action's error shape), so any subpath is
/// accepted here. All other scope guard rules from
/// [`validate_single_reference_with_context`] still apply: `$error` outside
/// the on_error scope falls through to the unknown-root branch and is
/// rejected as `UnknownReference`.
pub fn validate_single_reference_in_on_error(
    reference: &str,
    tables: &RefTables,
) -> ValidationResult<()> {
    validate_single_reference_with_context(reference, tables, None, true, false)
}

/// Validates a single reference that lives inside a `repeat` body.
///
/// The `$total` root is the runtime binding populated with the iteration
/// count inside a `repeat` body and is only legal in that scope. The tail
/// is not statically known (e.g. `count`), so any subpath is accepted
/// inside the repeat scope. Outside the repeat scope, `$total.*` is
/// rejected with [`ValidationError::ScopeGuardViolation`] (which points the
/// user at the `repeat` scope requirement), per the language spec
/// (Master §8: reserved roots include `total`, allowed only inside
/// `repeat` bodies).
pub fn validate_single_reference_in_repeat(
    reference: &str,
    tables: &RefTables,
) -> ValidationResult<()> {
    validate_single_reference_with_context(reference, tables, None, false, true)
}

/// Validates all references belonging to a single step, returning a
/// [`ValidationError::StepSkippedReference`] for the first reference that
/// fails to resolve.
///
/// This is the step-aware skip path: when a step carries a broken
/// reference, the runtime would silently skip the step and continue with
/// stale or default values. Validation surfaces that decision with a
/// typed diagnostic that records the failing step index and the
/// offending reference, so callers can fail the run or escalate the
/// error to the user instead of masking it.
///
/// `references` is the list of `$`-prefixed references that appear
/// inside the step body, in source order. The first reference whose
/// single-reference validation fails is reported; subsequent
/// references are not inspected. An empty list returns `Ok(())`.
///
/// `current_step_index` is forwarded to
/// [`validate_single_reference_with_context`] so step references are
/// restricted to prior steps (and the same step is rejected, matching
/// the existing prior-step rule).
pub fn validate_step_references(
    step: StepIdx,
    references: &[String],
    tables: &RefTables,
    current_step_index: usize,
) -> ValidationResult<()> {
    for reference in references {
        if validate_single_reference_with_context(
            reference.as_str(),
            tables,
            Some(current_step_index),
            false,
            false,
        )
        .is_err()
        {
            // Translate every reference-resolution failure into the
            // step-skip diagnostic so the caller has a single typed
            // signal for "this step was skipped because one of its
            // references did not resolve." The failing reference text
            // is preserved in the diagnostic so the user can locate
            // the broken reference inside the skipped step.
            return Err(ValidationError::StepSkippedReference {
                step,
                reference: reference.as_str().to_owned().into_boxed_str(),
            });
        }
    }
    Ok(())
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
    in_on_error: bool,
    in_repeat_scope: bool,
) -> ValidationResult<()> {
    match root {
        "input" => validate_declared(reference, tail, "input", &tables.inputs),
        "var" | "vars" => validate_declared(reference, tail, "var", &tables.vars),
        "secrets" => validate_declared(reference, tail, "secrets", &tables.secrets),
        "runtime" => Err(ValidationError::DirectRuntimeReference),
        "step" | "steps" => validate_step_reference(reference, tail, tables, current_step_index),
        // Scope guard: `$error` is the runtime binding populated by the
        // executor when an action fails. It is only legal inside the body
        // of an `on_error` handler (language spec, on_error rules: `$error`
        // is available only inside the handler). Outside the on_error
        // scope it falls through to the unknown-root arm below so the
        // message names `$error` and the user sees the scope violation.
        "error" if in_on_error => Ok(()),
        // Scope guard: `$total` is the runtime binding that holds the
        // iteration count inside a `repeat` body. It is only legal inside
        // that scope. The literal `total` match must come before the
        // `contains_step_id` and `contains_loop_variable` guards so a
        // literal `total` root is always recognised as the repeat-scope
        // root, not as a step ID or a loop variable. The tail (e.g. `count`)
        // is runtime-defined, so we accept any subpath when in scope.
        "total" if in_repeat_scope => Ok(()),
        "total" => Err(ValidationError::ScopeGuardViolation {
            reference: reference.to_owned(),
            required_scope: "repeat".to_owned(),
        }),
        // A reference whose root is a declared step ID (e.g. `$build_result.output`)
        // bypasses the required `$steps.<step_id>.<field>` prefix. Surface a
        // dedicated diagnostic so the user can be told to add the prefix.
        _ if tables.contains_step_id(root) => Err(ValidationError::DirectStepReference {
            step: root.to_owned(),
        }),
        _ if tables.contains_loop_variable(root) => Err(ValidationError::DirectLoopReference {
            variable: root.to_owned(),
        }),
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
    /// Loop variable names in scope (from for_each, together, collect bodies).
    pub loop_variable_names: Vec<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[cfg(test)]
#[path = "references/tests.rs"]
mod tests;
