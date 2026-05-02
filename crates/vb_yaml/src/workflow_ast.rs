//! Workflow-level AST and parsing entry point.

use saphyr::LoadableYamlNode;

use crate::{YamlError, YamlResult};

use super::types::{lookup, parse_inputs, parse_secrets, parse_vars, ExampleAst, InputField,
                  ResultMapping, SecretField, VarField};
use super::trigger_ast::{parse_trigger, TriggerAst};
use super::step::StepAst;
use super::step_parsing::parse_steps;
use super::step_metadata_parsing::{parse_examples, parse_result};

/// Top-level workflow AST produced by parsing a workflow YAML document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSource {
    /// Language version string (e.g. "velvet-ballastics/v1").
    pub version: String,
    /// Workflow name.
    pub name: String,
    /// Trigger declaration.
    pub trigger: TriggerAst,
    /// Declared input fields.
    pub inputs: Vec<InputField>,
    /// Declared workflow-level variables.
    pub vars: Vec<VarField>,
    /// Declared secret references.
    pub secrets: Vec<SecretField>,
    /// Ordered step list.
    pub steps: Vec<StepAst>,
    /// Optional result mapping.
    pub result: Option<ResultMapping>,
    /// Inline examples / test cases.
    pub examples: Vec<ExampleAst>,
}

/// Parse YAML text into a [`WorkflowSource`] AST.
///
/// This is a low-level function. Prefer [`crate::parse_workflow_source`] which
/// runs profile validation first.
pub fn parse_workflow_ast(text: &str) -> YamlResult<WorkflowSource> {
    let docs = saphyr::Yaml::load_from_str(text).map_err(|e| YamlError::ParseError {
        line: e.marker().line(),
        reason: e.info().into(),
    })?;

    let root = docs.into_iter().next().ok_or(YamlError::EmptySource)?;
    parse_workflow_from_yaml(&root)
}

/// Parse a single workflow document from a loaded saphyr Yaml node.
fn parse_workflow_from_yaml(root: &saphyr::Yaml<'_>) -> YamlResult<WorkflowSource> {
    if !root.is_mapping() {
        return Err(YamlError::FieldShape {
            field: "workflow",
            expected: "mapping",
        });
    }

    let version = require_str(root, "version")?;
    let name = require_str(root, "name")?;
    let trigger = parse_trigger(root)?;
    let inputs = parse_inputs(root)?;
    let vars = parse_vars(root)?;
    let secrets = parse_secrets(root)?;
    let steps = parse_steps(root)?;
    let result = parse_result(root)?;
    let examples = parse_examples(root)?;

    Ok(WorkflowSource {
        version,
        name,
        trigger,
        inputs,
        vars,
        secrets,
        steps,
        result,
        examples,
    })
}

/// Require a non-empty string field at the top level.
fn require_str(node: &saphyr::Yaml<'_>, field: &'static str) -> YamlResult<String> {
    match lookup(node, field) {
        None => Err(YamlError::MissingField { field }),
        Some(v) => match v.as_str() {
            Some(s) if !s.is_empty() => Ok(s.to_string()),
            _ => Err(YamlError::FieldShape {
                field,
                expected: "non-empty string",
            }),
        },
    }
}
