#![forbid(unsafe_code)]
//! Core reference-validation logic.
//!
//! This module owns every `validate_*` public function and the private
//! routing helpers (`validate_bare_reference`, `validate_rooted_reference`,
//! `validate_step_reference`, `validate_known_step_reference`,
//! `validate_declared`) that dispatch against `$`-prefixed reference shapes.
//!
//! The entry points are:
//!
//! - [`validate_single_reference`] — single reference, no scope context
//! - [`validate_single_reference_with_context`] — single reference with
//!   step-index and scope flags
//! - [`validate_single_reference_in_on_error`] — reference inside
//!   an `on_error` handler body
//! - [`validate_single_reference_in_repeat`] — reference inside a
//!   `repeat` body
//! - [`validate_step_references`] — batch validation for a step, emitting
//!   [`crate::ValidationError::StepSkippedReference`] on first failure

use crate::{ValidationError, ValidationResult};
use std::collections::HashSet;

use super::StepIdx;
use super::parse::{OUTPUT_FIELD_SYMBOL, step_field_is_output, step_index_to_step_idx};
use super::tables::RefTables;

/// Validates all references in a workflow document.
///
/// Builds reference tables from the workflow and validates every
/// `$`-prefixed reference in [`super::WorkflowRefs::references`].
pub fn validate_references(workflow: &super::WorkflowRefs) -> ValidationResult<()> {
    let tables = RefTables::build(workflow);
    for reference in &workflow.references {
        validate_single_reference(reference, &tables)?;
    }
    Ok(())
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
        return validate_bare_reference(reference, body, tables);
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

// ---------------------------------------------------------------------------
// Private routing helpers
// ---------------------------------------------------------------------------

fn validate_bare_reference(
    reference: &str,
    body: &str,
    tables: &RefTables,
) -> ValidationResult<()> {
    if matches!(body, "now" | "random") {
        return Err(ValidationError::DirectRuntimeReference);
    }
    if tables.contains_step_id(body) {
        return Err(ValidationError::DirectStepReference {
            step: body.to_owned(),
        });
    }
    if tables.contains_loop_variable(body) {
        return Err(ValidationError::DirectLoopReference {
            variable: body.to_owned(),
        });
    }
    Err(ValidationError::UnknownReference {
        reference: reference.to_owned(),
    })
}

pub(crate) fn validate_rooted_reference(
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
        // Master §8 allows direct `$step_id.x` references in addition to the
        // legacy `$steps.<step_id>.x` spelling.
        _ if tables.contains_step_id(root) => {
            validate_known_step_reference(reference, root, Some(tail), tables, current_step_index)
        }
        // Master §8 allows direct `$loop_name.x` roots for loop bindings.
        _ if tables.contains_loop_variable(root) => Ok(()),
        _ => Err(ValidationError::UnknownReference {
            reference: reference.to_owned(),
        }),
    }
}

fn validate_step_reference(
    reference: &str,
    tail: &str,
    tables: &RefTables,
    current_step_index: Option<usize>,
) -> ValidationResult<()> {
    let name = reference_name(tail);
    let field_tail = tail.split_once('.').map(|(_, field)| field);
    validate_known_step_reference(reference, name, field_tail, tables, current_step_index)
}

fn reference_name(tail: &str) -> &str {
    match tail.split_once('.') {
        Some((name, _)) => name,
        None => tail,
    }
}

fn validate_known_step_reference(
    reference: &str,
    name: &str,
    field_tail: Option<&str>,
    tables: &RefTables,
    current_step_index: Option<usize>,
) -> ValidationResult<()> {
    if let Some(step_idx) = tables.step_index(name) {
        // Step ID exists in the workflow.
        //
        // The reference is `$steps.<name>.<field>`. Check whether the
        // requested field is "output" and the step does NOT produce
        // one. In that case the runtime would have no value to bind
        // and the reference would silently resolve to absent data, so
        // validation surfaces the failure as
        // [`ValidationError::ResultReferenceMissing`].
        if step_field_is_output(field_tail) && !tables.step_has_output(name) {
            return Err(ValidationError::ResultReferenceMissing {
                step: step_index_to_step_idx(step_idx),
                missing_output: OUTPUT_FIELD_SYMBOL,
            });
        }
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
    kind: &str,
    names: &HashSet<String>,
) -> ValidationResult<()> {
    let name = reference_name(tail);
    if names.contains(name) {
        Ok(())
    } else if kind == "secrets" {
        Err(ValidationError::SecretNotDeclared {
            secret: name.to_owned(),
        })
    } else {
        Err(ValidationError::UnknownReference {
            reference: reference.to_owned(),
        })
    }
}
