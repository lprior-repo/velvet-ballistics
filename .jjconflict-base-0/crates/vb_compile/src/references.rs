#![forbid(unsafe_code)]
//! Reference validation for compiled workflow ASTs.
//!
//! Delegates core reference validation to `vb_validate::references` to avoid
//! duplicating validation logic. Handles compile-specific references (slot
//! accessors) locally since those are not part of the standalone validator's
//! surface.

use crate::ast::WorkflowAst;
use crate::{CompileError, CompileErrors};
use vb_validate::references::{RefTables, validate_single_reference_with_context};

#[path = "references_collect.rs"]
mod collect;

use collect::{
    build_ref_tables, collect_references_from_expression_entries, collect_references_from_steps,
    collect_references_from_value_entries, collect_references_from_values,
};

pub(crate) fn validate_workflow_ast(ast: &WorkflowAst) -> Result<(), CompileErrors> {
    let tables = build_ref_tables(ast);
    let mut errors = Vec::new();
    // Top-level declarations have no step context
    collect_references_from_value_entries(&ast.inputs, &tables, &mut errors, None);
    collect_references_from_value_entries(&ast.vars, &tables, &mut errors, None);
    collect_references_from_expression_entries(&ast.result, &tables, &mut errors, None);
    collect_references_from_values(&ast.examples, &tables, &mut errors, None);
    // Steps have step context for prior-reference validation
    collect_references_from_steps(&ast.steps, &tables, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileErrors(errors))
    }
}

/// Validates a reference from the compiler AST.
///
/// Handles compile-specific references (`$slot.*`) locally and delegates
/// everything else to `vb_validate::references::validate_single_reference_with_context`.
fn validate_compile_reference(
    reference: &str,
    tables: &RefTables,
    step_index: Option<usize>,
) -> Result<(), CompileError> {
    let Some(body) = reference.strip_prefix('$') else {
        return Ok(());
    };
    let Some((root, tail)) = body.split_once('.') else {
        // Bare reference -- delegate to shared validation
        return validate_single_reference_with_context(reference, tables, step_index)
            .map_err(|e| map_validation_error(reference, &e));
    };
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
    validate_single_reference_with_context(reference, tables, step_index)
        .map_err(|e| map_validation_error(reference, &e))
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

/// Maps a `vb_validate::ValidationError` from shared reference validation into
/// a `CompileError` with source-location context.
fn map_validation_error(reference: &str, error: &vb_validate::ValidationError) -> CompileError {
    match error {
        vb_validate::ValidationError::UnknownReference { .. } => {
            let Some(body) = reference.strip_prefix('$') else {
                return CompileError::UnknownReferenceRoot {
                    reference: Box::from(reference),
                    root: Box::from(reference),
                };
            };
            let Some((root, tail)) = body.split_once('.') else {
                return CompileError::UnknownReferenceRoot {
                    reference: Box::from(reference),
                    root: Box::from(body),
                };
            };
            let name = match tail.split_once('.') {
                Some((name, _)) => name,
                None => tail,
            };
            let kind = match root {
                "input" => "input",
                "var" | "vars" => "var",
                "secrets" => "secrets",
                "step" | "steps" => "step",
                _ => {
                    return CompileError::UnknownReferenceRoot {
                        reference: Box::from(reference),
                        root: Box::from(root),
                    };
                }
            };
            CompileError::UnknownReferenceName {
                kind,
                reference: Box::from(reference),
                name: Box::from(name),
            }
        }
        _ => CompileError::IllegalReference {
            reference: Box::from(reference),
        },
    }
}

#[cfg(test)]
#[path = "references/tests.rs"]
mod tests;
