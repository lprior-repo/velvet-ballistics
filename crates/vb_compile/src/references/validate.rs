//! Compile-specific reference validation routing.
//!
//! Validates a reference from the compiler AST, handling compile-specific
//! references (`$slot.*`, `$attempt.*`, accessor paths) locally and delegating
//! everything else to `vb_validate::references::validate_single_reference_with_context`.
//!
//! `in_repeat_body` lifts the `$attempt.*` scope guard for references that
//! appear inside a `Repeat` body. The flag is propagated from
//! `collect_references_from_repeat_body` and is `false` for top-level
//! references (where `$attempt.*` is rejected with `InvalidVariableScope`).

use super::errors::map_validation_error;
use crate::{CompileError, SourceMark};
use vb_validate::references::{RefTables, validate_single_reference_with_context};

pub(super) fn validate_compile_reference(
    reference: &str,
    tables: &RefTables,
    step_index: Option<usize>,
    in_repeat_body: bool,
) -> Result<(), CompileError> {
    let Some(body) = reference.strip_prefix('$') else {
        return Ok(());
    };
    let Some((root, tail)) = body.split_once('.') else {
        // Bare reference -- delegate to shared validation. Inside a repeat
        // body the bare `$attempt` is still illegal (it is only meaningful
        // with the `.number` accessor), so the scope guard does not lift for
        // bare references.
        return validate_single_reference_with_context(reference, tables, step_index, false, false)
            .map_err(|e| map_validation_error(reference, &e));
    };
    if root == "attempt" {
        if in_repeat_body {
            return Ok(());
        }
        return Err(reject_attempt_scope(reference));
    }
    // Compile-specific: slot references are not in the standalone validator
    if matches!(root, "slot" | "slots") {
        return validate_slot_reference(reference, root, tail);
    }
    // Compile-specific: reject accessor paths after declared names
    // (e.g., $vars.data.field is unsupported because the compiler
    // does not support accessor traversal on vars/inputs/secrets)
    if let Some(error) = check_accessor_path(reference, root, tail, tables) {
        return Err(error);
    }
    validate_single_reference_with_context(reference, tables, step_index, false, false)
        .map_err(|e| map_validation_error(reference, &e))
}

/// Rejects a `$attempt.*` reference observed outside a `Repeat` body.
///
/// Scope guard: `$attempt.*` is only legal inside a `Repeat` body step.
/// Architectural invariant: the cold AST (master spec §45) drops
/// `StepKindAst::Repeat` body expressions at construction. Any
/// `$attempt.*` reference that reaches this validator is therefore
/// by definition outside a `Repeat` body — there is no per-step
/// "in a Repeat body" flag on `RefTables` (only declared name
/// sets), and the cold-AST `Repeat` variant carries no body to
/// inspect. The blanket reject is correct under the cold-AST
/// invariant. When canonical lowering adds body retention (master
/// §45 follow-up), this guard will need a `repeat_step_indices`
/// set threaded through `RefTables` to support the legal
/// use case (see `references_scope_guard_tests.rs` for the
/// architectural note).
fn reject_attempt_scope(reference: &str) -> CompileError {
    CompileError::InvalidVariableScope {
        reference: Box::from(reference),
        context: "outside repeat body",
        allowed: Box::from(["repeat_attempt.body", "repeat_check"].as_slice()),
        mark: SourceMark::unavailable(),
    }
}

/// Validates a `$slot.*` reference (compile-specific).
fn validate_slot_reference(reference: &str, root: &str, tail: &str) -> Result<(), CompileError> {
    let (slot, path) = match tail.split_once('.') {
        Some((slot, path)) => (slot, Some(path)),
        None => (tail, None),
    };
    if slot.parse::<u16>().is_err() {
        return Err(CompileError::UnknownReferenceName {
            kind: "slot",
            reference: Box::from(reference),
            name: Box::from(slot),
        });
    }
    if let Some(path) = path {
        if numeric_accessor_path(path) {
            return Ok(());
        }
        let accessor_root = format!("{root}.{slot}");
        return Err(CompileError::UnsupportedAccessorReference {
            reference: Box::from(reference),
            root: Box::from(accessor_root),
            path: Box::from(path),
        });
    }
    Ok(())
}

fn numeric_accessor_path(path: &str) -> bool {
    let mut saw_segment = false;
    for segment in path.split('.') {
        // Reject empty segments (e.g., from "$slot.1..0") and non-numeric segments.
        if segment.is_empty() {
            return false;
        }
        if segment.parse::<u32>().is_err() {
            return false;
        }
        saw_segment = true;
    }
    saw_segment
}

/// Checks for unsupported accessor paths after declared names.
///
/// For example, `$vars.data.field` has an accessor path `field` after the
/// declared name `data`, which the compiler does not support.
fn check_accessor_path(
    reference: &str,
    root: &str,
    tail: &str,
    tables: &RefTables,
) -> Option<CompileError> {
    // Only check accessor paths for name-rooted references
    #[allow(clippy::question_mark)]
    let Some((name, path)) = tail.split_once('.') else {
        return None;
    };
    // Check if the root+name is declared; if so, the trailing path is unsupported
    let is_declared = match root {
        "input" | "inputs" => tables.contains_input(name),
        "var" | "vars" => tables.contains_var(name),
        "secrets" => tables.contains_secret(name),
        _ => return None,
    };
    if is_declared {
        let accessor_root = format!("{root}.{name}");
        return Some(CompileError::UnsupportedAccessorReference {
            reference: Box::from(reference),
            root: Box::from(accessor_root),
            path: Box::from(path),
        });
    }
    None
}
