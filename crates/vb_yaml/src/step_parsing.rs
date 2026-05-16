//! Step AST parsing - main step and body parsing.

use crate::{YamlError, YamlResult};

use super::types::{
    opt_str, require_str_in,
};
use super::step::StepAst;
use super::step_primitive_parsing::parse_step_primitive;

// ---------------------------------------------------------------------------
// Step parsing
// ---------------------------------------------------------------------------

/// Parse the steps list from a workflow node.
pub fn parse_steps(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<StepAst>> {
    let Some(seq) = super::types::lookup(node, "steps").and_then(|v| v.as_vec()) else {
        return Ok(Vec::new());
    };

    let mut steps = Vec::new();
    for item in seq {
        steps.push(parse_step(item)?);
    }
    Ok(steps)
}

fn parse_step(yaml: &saphyr::Yaml<'_>) -> YamlResult<StepAst> {
    if !yaml.is_mapping() {
        return Err(YamlError::FieldShape {
            field: "step",
            expected: "mapping",
        });
    }

    let id = require_str_in(yaml, "id", "step.id")?;
    let name = opt_str(yaml, "name");
    let condition = opt_str(yaml, "if");
    let primitive = parse_step_primitive(yaml)?;
    let with = opt_str(yaml, "with");
    let retry = super::step_metadata_parsing::parse_retry(yaml)?;
    let on_error = super::step_metadata_parsing::parse_error_handler(yaml)?;
    let then = opt_str(yaml, "then");

    Ok(StepAst {
        id,
        name,
        condition,
        primitive,
        with,
        retry,
        on_error,
        then,
    })
}

/// Parse the "steps" sub-sequence from a node.
pub fn parse_body_steps(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<StepAst>> {
    let Some(seq) = super::types::lookup(node, "steps").and_then(|v| v.as_vec()) else {
        return Ok(Vec::new());
    };

    let mut steps = Vec::new();
    for item in seq {
        steps.push(parse_step(item)?);
    }
    Ok(steps)
}
