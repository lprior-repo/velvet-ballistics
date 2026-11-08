#![forbid(unsafe_code)]
use super::marks::AstMarks;
use super::parse_step_fields::step_str;
use super::parse_step_kinds::parse_step_kind;
use super::parse_trigger::parse_trigger;
use super::parse_values::{parse_expression, parse_value};
use super::types::{AstExpression, AstMapEntry, AstValue, StepAst, WorkflowAst};
use crate::CompileError;
use saphyr::Yaml;

/// Parses a semantically validated YAML tree into the cold typed AST.
pub(crate) fn parse_workflow_ast(text: &str, doc: &Yaml<'_>) -> Result<WorkflowAst, CompileError> {
    let marks = AstMarks::new(text)?;
    Ok(WorkflowAst {
        version: required_str(doc, "version")?.into(),
        name: required_str(doc, "name")?.into(),
        trigger: parse_trigger(required_mapping(doc, "when")?, &marks)?,
        inputs: parse_value_map(doc, "inputs", &marks)?,
        vars: parse_value_map(doc, "vars", &marks)?,
        secrets: parse_secret_map(doc, &marks)?,
        result: parse_expression_map(doc, "result", &marks)?,
        examples: parse_examples(doc)?,
        steps: parse_steps(required_sequence(doc, "steps")?, &marks)?,
        mark: marks.document(),
    })
}

fn required_str<'a>(doc: &'a Yaml<'a>, field: &'static str) -> Result<&'a str, CompileError> {
    doc.as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?
        .as_str()
        .ok_or(CompileError::FieldShape {
            field,
            expected: "a string",
        })
}

fn required_mapping<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a saphyr::Mapping<'a>, CompileError> {
    doc.as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?
        .as_mapping()
        .ok_or(CompileError::FieldShape {
            field,
            expected: "a mapping",
        })
}

fn required_sequence<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a saphyr::Sequence<'a>, CompileError> {
    doc.as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?
        .as_sequence()
        .ok_or(CompileError::FieldShape {
            field,
            expected: "a sequence",
        })
}

pub(super) fn optional_str<'a>(value: &'a Yaml<'a>, field: &str) -> Option<&'a str> {
    value.as_mapping_get(field).and_then(Yaml::as_str)
}

fn parse_value_map(
    doc: &Yaml<'_>,
    field: &'static str,
    marks: &AstMarks,
) -> Result<Vec<AstMapEntry<AstValue>>, CompileError> {
    let Some(node) = doc.as_mapping_get(field) else {
        return Ok(Vec::new());
    };
    parse_map(node, field, marks, parse_value)
}

fn parse_expression_map(
    doc: &Yaml<'_>,
    field: &'static str,
    marks: &AstMarks,
) -> Result<Vec<AstMapEntry<AstExpression>>, CompileError> {
    let Some(node) = doc.as_mapping_get(field) else {
        return Ok(Vec::new());
    };
    parse_map(node, field, marks, parse_expression)
}

fn parse_map<T, F>(
    node: &Yaml<'_>,
    field: &'static str,
    marks: &AstMarks,
    parse: F,
) -> Result<Vec<AstMapEntry<T>>, CompileError>
where
    F: Fn(&Yaml<'_>) -> Result<T, CompileError>,
{
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field,
        expected: "a mapping",
    })?;
    let mut entries = Vec::with_capacity(mapping.len());
    for (key, value) in mapping {
        entries.push(parse_entry(key, value, field, marks, &parse)?);
    }
    Ok(entries)
}

fn parse_entry<T, F>(
    key: &Yaml<'_>,
    value: &Yaml<'_>,
    field: &'static str,
    marks: &AstMarks,
    parse: &F,
) -> Result<AstMapEntry<T>, CompileError>
where
    F: Fn(&Yaml<'_>) -> Result<T, CompileError>,
{
    let name = key.as_str().ok_or_else(crate::non_string_key_error)?;
    Ok(AstMapEntry {
        name: name.into(),
        value: parse(value)?,
        mark: marks.nested_key(field, name),
    })
}

fn parse_secret_map(
    doc: &Yaml<'_>,
    marks: &AstMarks,
) -> Result<Vec<AstMapEntry<Box<str>>>, CompileError> {
    let Some(node) = doc.as_mapping_get("secrets") else {
        return Ok(Vec::new());
    };
    parse_map(node, "secrets", marks, |value| {
        value
            .as_str()
            .map(Box::<str>::from)
            .ok_or(CompileError::FieldShape {
                field: "secrets",
                expected: "a mapping of secret names to environment variable names",
            })
    })
}

fn parse_examples(doc: &Yaml<'_>) -> Result<Vec<AstValue>, CompileError> {
    let Some(node) = doc.as_mapping_get("examples") else {
        return Ok(Vec::new());
    };
    let sequence = node.as_sequence().ok_or(CompileError::FieldShape {
        field: "examples",
        expected: "a sequence",
    })?;
    let mut examples = Vec::with_capacity(sequence.len());
    for item in sequence {
        examples.push(parse_value(item)?);
    }
    Ok(examples)
}

fn parse_steps(
    steps: &saphyr::Sequence<'_>,
    marks: &AstMarks,
) -> Result<Vec<StepAst>, CompileError> {
    let mut parsed = Vec::with_capacity(steps.len());
    for (index, step) in steps.iter().enumerate() {
        parsed.push(parse_step(step, index, marks)?);
    }
    Ok(parsed)
}

fn parse_step(step: &Yaml<'_>, index: usize, marks: &AstMarks) -> Result<StepAst, CompileError> {
    let mapping = step
        .as_mapping()
        .ok_or(CompileError::StepShape { step: index })?;
    let id = step_str(step, index, "id")?;
    let (primitive, kind) = parse_step_kind(mapping, index)?;
    Ok(StepAst {
        id: id.into(),
        name: optional_str(step, "name").map(Box::<str>::from),
        primitive,
        kind,
        mark: marks.step(id),
    })
}
