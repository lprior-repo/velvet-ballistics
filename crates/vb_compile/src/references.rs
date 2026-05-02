//! Reference validation for compiled workflow ASTs.
//!
//! Delegates core reference validation to `vb_validate::references` to avoid
//! duplicating validation logic. Handles compile-specific references (slot
//! accessors) locally since those are not part of the standalone validator's
//! surface.

use crate::ast::{AstExpression, AstMapEntry, AstValue, StepAst, WorkflowAst};
use crate::expression::ParsedExpression;
use crate::{CompileError, CompileErrors};
use vb_validate::references::{RefTables, validate_single_reference};

pub(crate) fn validate_workflow_ast(ast: &WorkflowAst) -> Result<(), CompileErrors> {
    let tables = build_ref_tables(ast);
    let mut errors = Vec::new();
    collect_references_from_value_entries(&ast.inputs, &tables, &mut errors);
    collect_references_from_value_entries(&ast.vars, &tables, &mut errors);
    collect_references_from_expression_entries(&ast.result, &tables, &mut errors);
    collect_references_from_values(&ast.examples, &tables, &mut errors);
    collect_references_from_steps(&ast.steps, &tables, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileErrors(errors))
    }
}

fn build_ref_tables(ast: &WorkflowAst) -> RefTables {
    let inputs = entry_names_owned(&ast.inputs);
    let vars = entry_names_owned(&ast.vars);
    let secrets = secret_names_owned(&ast.secrets);
    let step_ids = step_names_owned(&ast.steps);
    RefTables::from_slices(&inputs, &vars, &secrets, &step_ids)
}

fn entry_names_owned<T>(entries: &[AstMapEntry<T>]) -> Vec<String> {
    let mut names = Vec::with_capacity(entries.len());
    for entry in entries {
        names.push(entry.name.as_ref().to_owned());
    }
    names
}

fn secret_names_owned(entries: &[AstMapEntry<Box<str>>]) -> Vec<String> {
    let mut names = Vec::with_capacity(entries.len());
    for entry in entries {
        names.push(entry.name.as_ref().to_owned());
    }
    names
}

fn step_names_owned(steps: &[StepAst]) -> Vec<String> {
    let mut names = Vec::with_capacity(steps.len());
    for step in steps {
        names.push(step.id.as_ref().to_owned());
    }
    names
}

fn collect_references_from_value_entries(
    entries: &[AstMapEntry<AstValue>],
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
) {
    for entry in entries {
        collect_references_from_value(&entry.value, tables, errors);
    }
}

fn collect_references_from_expression_entries(
    entries: &[AstMapEntry<AstExpression>],
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
) {
    for entry in entries {
        collect_references_from_expression(&entry.value, tables, errors);
    }
}

fn collect_references_from_values(
    values: &[AstValue],
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
) {
    for value in values {
        collect_references_from_value(value, tables, errors);
    }
}

fn collect_references_from_steps(
    steps: &[StepAst],
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
) {
    for step in steps {
        collect_references_from_step_kind(&step.kind, tables, errors);
    }
}

fn collect_references_from_step_kind(
    kind: &crate::ast::StepKindAst,
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
) {
    use crate::ast::StepKindAst;
    match kind {
        StepKindAst::Run { .. }
        | StepKindAst::ForEach { .. }
        | StepKindAst::Together { .. }
        | StepKindAst::Collect { .. }
        | StepKindAst::Repeat { .. }
        | StepKindAst::Wait { .. }
        | StepKindAst::Ask { .. } => {}
        StepKindAst::Save { fields } => collect_references_from_value_entries(fields, tables, errors),
        StepKindAst::Choose { condition, .. } => {
            collect_references_from_expression(condition, tables, errors);
        }
        StepKindAst::Reduce { initial, .. } => {
            collect_references_from_value(initial, tables, errors);
        }
        StepKindAst::Finish { result } => {
            collect_references_from_expression(result, tables, errors);
        }
    }
}

fn collect_references_from_expression(
    expression: &AstExpression,
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
) {
    match expression {
        AstExpression::Slot(_) => {}
        AstExpression::Reference(reference) => {
            if let Err(e) = validate_compile_reference(reference.as_ref(), tables) {
                errors.push(e);
            }
        }
        AstExpression::Parsed(expression) => {
            collect_references_from_parsed_expression(expression, tables, errors);
        }
        AstExpression::Literal(value) => collect_references_from_value(value, tables, errors),
    }
}

fn collect_references_from_parsed_expression(
    expression: &ParsedExpression,
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
) {
    match expression {
        ParsedExpression::Reference(reference) => {
            if let Err(e) = validate_compile_reference(reference.as_ref(), tables) {
                errors.push(e);
            }
        }
        ParsedExpression::Unary { expr, .. } => {
            collect_references_from_parsed_expression(expr, tables, errors);
        }
        ParsedExpression::Binary { left, right, .. } => {
            collect_references_from_parsed_expression(left, tables, errors);
            collect_references_from_parsed_expression(right, tables, errors);
        }
        ParsedExpression::HelperCall { args, .. } => {
            for arg in args {
                collect_references_from_parsed_expression(arg, tables, errors);
            }
        }
        ParsedExpression::Literal(_) => {}
    }
}

fn collect_references_from_value(
    value: &AstValue,
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
) {
    match value {
        AstValue::Reference(reference) => {
            if let Err(e) = validate_compile_reference(reference.as_ref(), tables) {
                errors.push(e);
            }
        }
        AstValue::Sequence(values) => collect_references_from_values(values, tables, errors),
        AstValue::Mapping(entries) => collect_references_from_value_entries(entries, tables, errors),
        AstValue::Null | AstValue::Bool(_) | AstValue::I64(_) | AstValue::Text(_) => {}
    }
}

/// Validates a reference from the compiler AST.
///
/// Handles compile-specific references (`$slot.*`) locally and delegates
/// everything else to `vb_validate::references::validate_single_reference`.
fn validate_compile_reference(reference: &str, tables: &RefTables) -> Result<(), CompileError> {
    let Some(body) = reference.strip_prefix('$') else {
        return Ok(());
    };
    let Some((root, tail)) = body.split_once('.') else {
        // Bare reference -- delegate to shared validation
        return validate_single_reference(reference, tables)
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
    validate_single_reference(reference, tables)
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
mod tests;
