//! AST walk functions for reference collection.
//!
//! Two parallel walks exist: the standard walk (with step context and
//! `in_repeat_body = false`) and the repeat-body walk (no step context,
//! `in_repeat_body = true` which lifts the `$attempt.*` scope guard).

use super::tables::build_ref_tables;
use super::validate::{
    scan_idempotency_key_references, validate_compile_reference,
    validate_idempotency_key_determinism,
};
use crate::ast::{AstExpression, AstMapEntry, AstValue, StepAst, TriggerAst, WorkflowAst};
use crate::expression::ParsedExpression;
use crate::{CompileError, CompileErrors};
use vb_validate::references::RefTables;

/// Entry point: validates all references in a workflow AST.
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
    // Master §65: webhook trigger `unique` is the YAML idempotency-key
    // surface. Reject non-deterministic references before any step sees
    // the value, so a workflow with `$runtime.now` in `unique` never
    // reaches runtime admission.
    validate_idempotency_key_surface(&ast.trigger, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileErrors(errors))
    }
}

/// Master §65 idempotency-key determinism gate for YAML trigger surfaces.
///
/// The webhook trigger carries an optional `unique` field whose value is
/// the per-request idempotency key. References in that string are extracted
/// and routed through `validate_idempotency_key_determinism`. Triggers
/// without a `unique` field (manual / schedule / event) have no key surface
/// here and pass through untouched; future per-action `idempotency.key`
/// fields will be added at the same call site.
fn validate_idempotency_key_surface(trigger: &TriggerAst, errors: &mut Vec<CompileError>) {
    let TriggerAst::Webhook { unique, .. } = trigger else {
        return;
    };
    let Some(unique_text) = unique.as_ref() else {
        return;
    };
    let mut references: Vec<Box<str>> = Vec::new();
    scan_idempotency_key_references(unique_text.as_ref(), &mut references);
    let borrowed: Vec<&str> = references.iter().map(Box::as_ref).collect();
    if let Err(error) = validate_idempotency_key_determinism(&borrowed) {
        errors.push(error);
    }
}

// ── Top-level entry walkers ──────────────────────────────────────────────

fn collect_references_from_value_entries(
    entries: &[AstMapEntry<AstValue>],
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
    step_index: Option<usize>,
) {
    for entry in entries {
        collect_references_from_value(&entry.value, tables, errors, step_index);
    }
}

fn collect_references_from_expression_entries(
    entries: &[AstMapEntry<AstExpression>],
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
    step_index: Option<usize>,
) {
    for entry in entries {
        collect_references_from_expression(&entry.value, tables, errors, step_index);
    }
}

fn collect_references_from_values(
    values: &[AstValue],
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
    step_index: Option<usize>,
) {
    for value in values {
        collect_references_from_value(value, tables, errors, step_index);
    }
}

fn collect_references_from_steps(
    steps: &[StepAst],
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
) {
    for (step_index, step) in steps.iter().enumerate() {
        collect_references_from_step_kind(&step.kind, tables, errors, step_index);
    }
}

fn collect_references_from_step_kind(
    kind: &crate::ast::StepKindAst,
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
    step_index: usize,
) {
    use crate::ast::StepKindAst;
    match kind {
        StepKindAst::Run { .. }
        | StepKindAst::ForEach { .. }
        | StepKindAst::Together { .. }
        | StepKindAst::Collect { .. }
        | StepKindAst::Wait { .. }
        | StepKindAst::Ask { .. } => {}
        StepKindAst::Repeat { body, .. } => {
            collect_references_from_repeat_body(body, tables, errors);
        }
        StepKindAst::Save { fields } => {
            collect_references_from_value_entries(fields, tables, errors, Some(step_index))
        }
        StepKindAst::Choose { condition, .. } => {
            collect_references_from_expression(condition, tables, errors, Some(step_index));
        }
        StepKindAst::Reduce { initial, .. } => {
            collect_references_from_value(initial, tables, errors, Some(step_index));
        }
        StepKindAst::Finish { result } => {
            collect_references_from_expression(result, tables, errors, Some(step_index));
        }
    }
}

// ── Repeat-body walkers (no step context, in_repeat_body = true) ─────────

/// Walks a `Repeat` body with the scope flag that allows `$attempt.*`.
///
/// Body steps live below the parent `Repeat` step. Body sub-steps are not
/// part of the top-level `ast.steps` vector, so we have no `step_index` to
/// pass; references in body steps are validated with `in_repeat_body = true`.
/// The shared `vb_validate` table only needs the declared name sets, which
/// are already populated by `build_ref_tables`.
fn collect_references_from_repeat_body(
    body: &[crate::ast::StepAst],
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
) {
    use crate::ast::StepKindAst;
    for body_step in body {
        match &body_step.kind {
            StepKindAst::Run { .. }
            | StepKindAst::Together { .. }
            | StepKindAst::Collect { .. }
            | StepKindAst::Wait { .. }
            | StepKindAst::Ask { .. } => {}
            StepKindAst::ForEach { .. } => {}
            // Nested Repeat bodies remain inside the repeat scope.
            StepKindAst::Repeat { body: inner, .. } => {
                collect_references_from_repeat_body(inner, tables, errors);
            }
            StepKindAst::Save { fields } => {
                collect_references_from_repeat_body_value_entries(fields, tables, errors);
            }
            StepKindAst::Choose { condition, .. } => {
                collect_references_from_repeat_body_expression(condition, tables, errors);
            }
            StepKindAst::Reduce { initial, .. } => {
                collect_references_from_repeat_body_value(initial, tables, errors);
            }
            StepKindAst::Finish { result } => {
                collect_references_from_repeat_body_expression(result, tables, errors);
            }
        }
    }
}

fn collect_references_from_repeat_body_expression(
    expression: &AstExpression,
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
) {
    match expression {
        AstExpression::Slot(_) => {}
        AstExpression::Reference(reference) => {
            if let Err(e) = validate_compile_reference(reference.as_ref(), tables, None, true) {
                errors.push(e);
            }
        }
        AstExpression::Parsed(expression) => {
            collect_references_from_repeat_body_parsed_expression(expression, tables, errors);
        }
        AstExpression::Literal(value) => {
            collect_references_from_repeat_body_value(value, tables, errors);
        }
    }
}

fn collect_references_from_repeat_body_parsed_expression(
    expression: &ParsedExpression,
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
) {
    match expression {
        ParsedExpression::Reference(reference) => {
            if let Err(e) = validate_compile_reference(reference.as_ref(), tables, None, true) {
                errors.push(e);
            }
        }
        ParsedExpression::Unary { expr, .. } => {
            collect_references_from_repeat_body_parsed_expression(expr, tables, errors);
        }
        ParsedExpression::Binary { left, right, .. } => {
            collect_references_from_repeat_body_parsed_expression(left, tables, errors);
            collect_references_from_repeat_body_parsed_expression(right, tables, errors);
        }
        ParsedExpression::HelperCall { args, .. } => {
            for arg in args {
                collect_references_from_repeat_body_parsed_expression(arg, tables, errors);
            }
        }
        ParsedExpression::Literal(_) => {}
    }
}

fn collect_references_from_repeat_body_value(
    value: &AstValue,
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
) {
    match value {
        AstValue::Reference(reference) => {
            if let Err(e) = validate_compile_reference(reference.as_ref(), tables, None, true) {
                errors.push(e);
            }
        }
        AstValue::Sequence(values) => {
            for value in values {
                collect_references_from_repeat_body_value(value, tables, errors);
            }
        }
        AstValue::Mapping(entries) => {
            collect_references_from_repeat_body_value_entries(entries, tables, errors);
        }
        AstValue::Null | AstValue::Bool(_) | AstValue::I64(_) | AstValue::Text(_) => {}
    }
}

fn collect_references_from_repeat_body_value_entries(
    entries: &[AstMapEntry<AstValue>],
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
) {
    for entry in entries {
        collect_references_from_repeat_body_value(&entry.value, tables, errors);
    }
}

// ── Single-value expression walkers (standard, with step context) ────────

fn collect_references_from_expression(
    expression: &AstExpression,
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
    step_index: Option<usize>,
) {
    match expression {
        AstExpression::Slot(_) => {}
        AstExpression::Reference(reference) => {
            if let Err(e) =
                validate_compile_reference(reference.as_ref(), tables, step_index, false)
            {
                errors.push(e);
            }
        }
        AstExpression::Parsed(expression) => {
            collect_references_from_parsed_expression(expression, tables, errors, step_index);
        }
        AstExpression::Literal(value) => {
            collect_references_from_value(value, tables, errors, step_index)
        }
    }
}

fn collect_references_from_parsed_expression(
    expression: &ParsedExpression,
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
    step_index: Option<usize>,
) {
    match expression {
        ParsedExpression::Reference(reference) => {
            if let Err(e) =
                validate_compile_reference(reference.as_ref(), tables, step_index, false)
            {
                errors.push(e);
            }
        }
        ParsedExpression::Unary { expr, .. } => {
            collect_references_from_parsed_expression(expr, tables, errors, step_index);
        }
        ParsedExpression::Binary { left, right, .. } => {
            collect_references_from_parsed_expression(left, tables, errors, step_index);
            collect_references_from_parsed_expression(right, tables, errors, step_index);
        }
        ParsedExpression::HelperCall { args, .. } => {
            for arg in args {
                collect_references_from_parsed_expression(arg, tables, errors, step_index);
            }
        }
        ParsedExpression::Literal(_) => {}
    }
}

fn collect_references_from_value(
    value: &AstValue,
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
    step_index: Option<usize>,
) {
    match value {
        AstValue::Reference(reference) => {
            if let Err(e) =
                validate_compile_reference(reference.as_ref(), tables, step_index, false)
            {
                errors.push(e);
            }
        }
        AstValue::Sequence(values) => {
            collect_references_from_values(values, tables, errors, step_index)
        }
        AstValue::Mapping(entries) => {
            collect_references_from_value_entries(entries, tables, errors, step_index)
        }
        AstValue::Null | AstValue::Bool(_) | AstValue::I64(_) | AstValue::Text(_) => {}
    }
}
