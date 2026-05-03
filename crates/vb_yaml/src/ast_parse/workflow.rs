//! Top-level workflow document parsing.

use saphyr::LoadableYamlNode;

use crate::{YamlError, YamlResult};

use super::{fields, metadata, steps};
use crate::ast::WorkflowSource;

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

    let version = super::trigger::require_str(root, "version")?;
    let name = super::trigger::require_str(root, "name")?;
    let trig = super::trigger::parse_trigger(root)?;
    let inputs = fields::parse_inputs(root)?;
    let vars = fields::parse_vars(root)?;
    let secrets = fields::parse_secrets(root)?;
    let step_list = steps::parse_steps(root)?;
    let result = metadata::parse_result(root)?;
    let examples = metadata::parse_examples(root)?;

    Ok(WorkflowSource {
        version,
        name,
        trigger: trig,
        inputs,
        vars,
        secrets,
        steps: step_list,
        result,
        examples,
    })
}
