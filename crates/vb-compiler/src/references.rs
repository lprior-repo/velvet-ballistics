use crate::CompileError;
use crate::ast::{AstExpression, AstMapEntry, AstValue, StepAst, StepKindAst, WorkflowAst};
use crate::expression::ParsedExpression;
use std::collections::HashSet;

pub(crate) fn validate_workflow_ast(ast: &WorkflowAst) -> Result<(), CompileError> {
    let tables = ReferenceTables::new(ast);
    validate_value_entries(&ast.inputs, &tables)?;
    validate_value_entries(&ast.vars, &tables)?;
    validate_expression_entries(&ast.result, &tables)?;
    validate_values(&ast.examples, &tables)?;
    validate_steps(&ast.steps, &tables)
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
) -> Result<(), CompileError> {
    for entry in entries {
        validate_value(&entry.value, tables)?;
    }
    Ok(())
}

fn validate_expression_entries(
    entries: &[AstMapEntry<AstExpression>],
    tables: &ReferenceTables<'_>,
) -> Result<(), CompileError> {
    for entry in entries {
        validate_expression(&entry.value, tables)?;
    }
    Ok(())
}

fn validate_values(values: &[AstValue], tables: &ReferenceTables<'_>) -> Result<(), CompileError> {
    for value in values {
        validate_value(value, tables)?;
    }
    Ok(())
}

fn validate_steps(steps: &[StepAst], tables: &ReferenceTables<'_>) -> Result<(), CompileError> {
    for step in steps {
        validate_step_kind(&step.kind, tables)?;
    }
    Ok(())
}

fn validate_step_kind(
    kind: &StepKindAst,
    tables: &ReferenceTables<'_>,
) -> Result<(), CompileError> {
    match kind {
        StepKindAst::Save { fields } => validate_value_entries(fields, tables),
        StepKindAst::Choose { condition, .. } => validate_expression(condition, tables),
        StepKindAst::Finish { result } => validate_expression(result, tables),
    }
}

fn validate_expression(
    expression: &AstExpression,
    tables: &ReferenceTables<'_>,
) -> Result<(), CompileError> {
    match expression {
        AstExpression::Slot(_) => Ok(()),
        AstExpression::Reference(reference) => validate_reference(reference, tables),
        AstExpression::Parsed(expression) => validate_parsed_expression(expression, tables),
        AstExpression::Literal(value) => validate_value(value, tables),
    }
}

fn validate_parsed_expression(
    expression: &ParsedExpression,
    tables: &ReferenceTables<'_>,
) -> Result<(), CompileError> {
    match expression {
        ParsedExpression::Reference(reference) => validate_reference(reference, tables),
        ParsedExpression::Unary { expr, .. } => validate_parsed_expression(expr, tables),
        ParsedExpression::Binary { left, right, .. } => {
            validate_parsed_expression(left, tables)?;
            validate_parsed_expression(right, tables)
        }
        ParsedExpression::HelperCall { args, .. } => validate_parsed_args(args, tables),
        ParsedExpression::Literal(_) => Ok(()),
    }
}

fn validate_parsed_args(
    args: &[ParsedExpression],
    tables: &ReferenceTables<'_>,
) -> Result<(), CompileError> {
    for arg in args {
        validate_parsed_expression(arg, tables)?;
    }
    Ok(())
}

fn validate_value(value: &AstValue, tables: &ReferenceTables<'_>) -> Result<(), CompileError> {
    match value {
        AstValue::Reference(reference) => validate_reference(reference, tables),
        AstValue::Sequence(values) => validate_values(values, tables),
        AstValue::Mapping(entries) => validate_value_entries(entries, tables),
        AstValue::Null | AstValue::Bool(_) | AstValue::I64(_) | AstValue::Text(_) => Ok(()),
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
