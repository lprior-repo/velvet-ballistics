#![forbid(unsafe_code)]
use super::validate_compile_reference;
use crate::CompileError;
use crate::ast::{AstExpression, AstMapEntry, AstValue, StepAst, WorkflowAst};
use crate::expression::ParsedExpression;
use vb_validate::references::RefTables;

pub(super) fn build_ref_tables(ast: &WorkflowAst) -> RefTables {
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

pub(super) fn collect_references_from_value_entries(
    entries: &[AstMapEntry<AstValue>],
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
    step_index: Option<usize>,
) {
    for entry in entries {
        collect_references_from_value(&entry.value, tables, errors, step_index);
    }
}

pub(super) fn collect_references_from_expression_entries(
    entries: &[AstMapEntry<AstExpression>],
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
    step_index: Option<usize>,
) {
    for entry in entries {
        collect_references_from_expression(&entry.value, tables, errors, step_index);
    }
}

pub(super) fn collect_references_from_values(
    values: &[AstValue],
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
    step_index: Option<usize>,
) {
    for value in values {
        collect_references_from_value(value, tables, errors, step_index);
    }
}

pub(super) fn collect_references_from_steps(
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
        | StepKindAst::Repeat { .. }
        | StepKindAst::Wait { .. }
        | StepKindAst::Ask { .. } => {}
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

fn collect_references_from_expression(
    expression: &AstExpression,
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
    step_index: Option<usize>,
) {
    match expression {
        AstExpression::Slot(_) => {}
        AstExpression::Reference(reference) => {
            if let Err(e) = validate_compile_reference(reference.as_ref(), tables, step_index) {
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
            if let Err(e) = validate_compile_reference(reference.as_ref(), tables, step_index) {
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
            if let Err(e) = validate_compile_reference(reference.as_ref(), tables, step_index) {
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
