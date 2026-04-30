use crate::{CompileError, CompileErrors};
use crate::ast::{AstExpression, AstMapEntry, AstValue, StepAst, StepKindAst, WorkflowAst};
use crate::expression::ParsedExpression;
use std::collections::HashSet;

pub(crate) fn validate_workflow_ast(ast: &WorkflowAst) -> Result<(), CompileErrors> {
    let tables = ReferenceTables::new(ast);
    let mut errors = Vec::new();
    validate_value_entries(&ast.inputs, &tables, &mut errors);
    validate_value_entries(&ast.vars, &tables, &mut errors);
    validate_expression_entries(&ast.result, &tables, &mut errors);
    validate_values(&ast.examples, &tables, &mut errors);
    validate_steps(&ast.steps, &tables, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileErrors(errors))
    }
}

struct ReferenceTables<'a> {
    inputs: HashSet<&'a str>,
    vars: HashSet<&'a str>,
    secrets: HashSet<&'a str>,
    steps: HashSet<&'a str>,
}

impl<'a> ReferenceTables<'a> {
    fn new(ast: &'a WorkflowAst) -> Self {
        Self {
            inputs: entry_names(&ast.inputs),
            vars: entry_names(&ast.vars),
            secrets: entry_names(&ast.secrets),
            steps: step_names(&ast.steps),
        }
    }
}

fn entry_names<T>(entries: &[AstMapEntry<T>]) -> HashSet<&str> {
    let mut names = HashSet::with_capacity(entries.len());
    for entry in entries {
        let _ = names.insert(entry.name.as_ref());
    }
    names
}

fn step_names(steps: &[StepAst]) -> HashSet<&str> {
    let mut names = HashSet::with_capacity(steps.len());
    for step in steps {
        let _ = names.insert(step.id.as_ref());
    }
    names
}

fn validate_value_entries(
    entries: &[AstMapEntry<AstValue>],
    tables: &ReferenceTables<'_>,
    errors: &mut Vec<CompileError>,
) {
    for entry in entries {
        validate_value(&entry.value, tables, errors);
    }
}

fn validate_expression_entries(
    entries: &[AstMapEntry<AstExpression>],
    tables: &ReferenceTables<'_>,
    errors: &mut Vec<CompileError>,
) {
    for entry in entries {
        validate_expression(&entry.value, tables, errors);
    }
}

fn validate_values(values: &[AstValue], tables: &ReferenceTables<'_>, errors: &mut Vec<CompileError>) {
    for value in values {
        validate_value(value, tables, errors);
    }
}

fn validate_steps(steps: &[StepAst], tables: &ReferenceTables<'_>, errors: &mut Vec<CompileError>) {
    for step in steps {
        validate_step_kind(&step.kind, tables, errors);
    }
}

fn validate_step_kind(
    kind: &StepKindAst,
    tables: &ReferenceTables<'_>,
    errors: &mut Vec<CompileError>,
) {
    match kind {
        StepKindAst::Save { fields } => validate_value_entries(fields, tables, errors),
        StepKindAst::Choose { condition, .. } => validate_expression(condition, tables, errors),
        StepKindAst::Finish { result } => validate_expression(result, tables, errors),
    }
}

fn validate_expression(
    expression: &AstExpression,
    tables: &ReferenceTables<'_>,
    errors: &mut Vec<CompileError>,
) {
    match expression {
        AstExpression::Slot(_) => {}
        AstExpression::Reference(reference) => {
            if let Err(e) = validate_reference(reference, tables) {
                errors.push(e);
            }
        }
        AstExpression::Parsed(expression) => validate_parsed_expression(expression, tables, errors),
        AstExpression::Literal(value) => validate_value(value, tables, errors),
    }
}

fn validate_parsed_expression(
    expression: &ParsedExpression,
    tables: &ReferenceTables<'_>,
    errors: &mut Vec<CompileError>,
) {
    match expression {
        ParsedExpression::Reference(reference) => {
            if let Err(e) = validate_reference(reference, tables) {
                errors.push(e);
            }
        }
        ParsedExpression::Unary { expr, .. } => validate_parsed_expression(expr, tables, errors),
        ParsedExpression::Binary { left, right, .. } => {
            validate_parsed_expression(left, tables, errors);
            validate_parsed_expression(right, tables, errors);
        }
        ParsedExpression::HelperCall { args, .. } => validate_parsed_args(args, tables, errors),
        ParsedExpression::Literal(_) => {}
    }
}

fn validate_parsed_args(
    args: &[ParsedExpression],
    tables: &ReferenceTables<'_>,
    errors: &mut Vec<CompileError>,
) {
    for arg in args {
        validate_parsed_expression(arg, tables, errors);
    }
}

fn validate_value(value: &AstValue, tables: &ReferenceTables<'_>, errors: &mut Vec<CompileError>) {
    match value {
        AstValue::Reference(reference) => {
            if let Err(e) = validate_reference(reference, tables) {
                errors.push(e);
            }
        }
        AstValue::Sequence(values) => validate_values(values, tables, errors),
        AstValue::Mapping(entries) => validate_value_entries(entries, tables, errors),
        AstValue::Null | AstValue::Bool(_) | AstValue::I64(_) | AstValue::Text(_) => {}
    }
}

fn validate_reference(reference: &str, tables: &ReferenceTables<'_>) -> Result<(), CompileError> {
    let Some(body) = reference.strip_prefix('$') else {
        return Ok(());
    };
    let Some((root, tail)) = body.split_once('.') else {
        return validate_bare_reference(reference, body);
    };
    validate_rooted_reference(reference, root, tail, tables)
}

fn validate_bare_reference(reference: &str, body: &str) -> Result<(), CompileError> {
    if matches!(body, "now" | "random") {
        Err(illegal_reference(reference))
    } else {
        Err(CompileError::UnknownReferenceRoot {
            reference: Box::<str>::from(reference),
            root: Box::<str>::from(body),
        })
    }
}

fn validate_rooted_reference(
    reference: &str,
    root: &str,
    tail: &str,
    tables: &ReferenceTables<'_>,
) -> Result<(), CompileError> {
    match root {
        "input" => validate_declared(reference, tail, "input", &tables.inputs),
        "var" | "vars" => validate_declared(reference, tail, "var", &tables.vars),
        "secrets" => validate_declared(reference, tail, "secrets", &tables.secrets),
        "runtime" => Err(illegal_reference(reference)),
        "step" | "steps" => validate_step_reference(reference, tail, tables),
        _ => Err(CompileError::UnknownReferenceRoot {
            reference: Box::<str>::from(reference),
            root: Box::<str>::from(root),
        }),
    }
}

fn validate_step_reference(
    reference: &str,
    tail: &str,
    tables: &ReferenceTables<'_>,
) -> Result<(), CompileError> {
    let name = reference_name(tail);
    if tables.steps.contains(name) {
        Err(illegal_reference(reference))
    } else {
        Err(CompileError::UnknownReferenceName {
            kind: "step",
            reference: Box::<str>::from(reference),
            name: Box::<str>::from(name),
        })
    }
}

fn validate_declared(
    reference: &str,
    tail: &str,
    kind: &'static str,
    names: &HashSet<&str>,
) -> Result<(), CompileError> {
    let name = reference_name(tail);
    if names.contains(name) {
        Ok(())
    } else {
        Err(CompileError::UnknownReferenceName {
            kind,
            reference: Box::<str>::from(reference),
            name: Box::<str>::from(name),
        })
    }
}

fn reference_name(tail: &str) -> &str {
    match tail.split_once('.') {
        Some((name, _)) => name,
        None => tail,
    }
}

fn illegal_reference(reference: &str) -> CompileError {
    CompileError::IllegalReference {
        reference: Box::<str>::from(reference),
    }
}

#[cfg(test)]
mod tests;
