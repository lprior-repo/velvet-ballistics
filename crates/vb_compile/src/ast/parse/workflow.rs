#![forbid(unsafe_code)]
//! Top-level workflow AST parsing.
//!
//! Handles document-level field extraction (version, name, inputs, vars,
//! secrets, result, examples) and orchestrates the full workflow parse.

use crate::CompileError;
use crate::ast::marks::AstMarks;
use crate::ast::types::{AstExpression, AstMapEntry, AstValue, WorkflowAst};
use saphyr::Yaml;

use super::expr::parse_expression;
use super::field::{parse_value, required_mapping, required_sequence, required_str};
use super::step::parse_steps;
use super::trigger::parse_trigger;

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

/// Parse an optional value map from a document field.
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

/// Parse an optional expression map from a document field.
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

/// Generic map parser using a field-specific parse function.
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

/// Generic map entry parser.
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

/// Parse an optional secret map (names to environment variable names).
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

/// Parse optional example documents from the workflow document.
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
