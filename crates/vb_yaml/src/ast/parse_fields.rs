//! Field parsing logic for inputs, vars, secrets, result, and examples.

use crate::{YamlError, YamlResult};

use super::parse::{lookup, opt_str, require_str_in};
use super::types::*;

// ---------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------

pub(super) fn parse_inputs(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<InputField>> {
    let Some(seq) = lookup(node, "inputs").and_then(|v| v.as_vec()) else {
        return Ok(Vec::new());
    };

    let mut inputs = Vec::new();
    for item in seq {
        if !item.is_mapping() {
            return Err(YamlError::FieldShape {
                field: "inputs",
                expected: "mapping",
            });
        }
        let name = require_str_in(item, "name", "inputs[].name")?;
        let field_type = opt_str(item, "type");
        let default = opt_str(item, "default");
        inputs.push(InputField {
            name,
            field_type,
            default,
        });
    }
    Ok(inputs)
}

// ---------------------------------------------------------------------------
// Vars
// ---------------------------------------------------------------------------

pub(super) fn parse_vars(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<VarField>> {
    let Some(seq) = lookup(node, "vars").and_then(|v| v.as_vec()) else {
        return Ok(Vec::new());
    };

    let mut vars = Vec::new();
    for item in seq {
        if !item.is_mapping() {
            return Err(YamlError::FieldShape {
                field: "vars",
                expected: "mapping",
            });
        }
        let name = require_str_in(item, "name", "vars[].name")?;
        let value = opt_str(item, "value");
        vars.push(VarField { name, value });
    }
    Ok(vars)
}

// ---------------------------------------------------------------------------
// Secrets
// ---------------------------------------------------------------------------

pub(super) fn parse_secrets(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<SecretField>> {
    let Some(seq) = lookup(node, "secrets").and_then(|v| v.as_vec()) else {
        return Ok(Vec::new());
    };

    let mut secrets = Vec::new();
    for item in seq {
        if !item.is_mapping() {
            return Err(YamlError::FieldShape {
                field: "secrets",
                expected: "mapping",
            });
        }
        let name = require_str_in(item, "name", "secrets[].name")?;
        let key = opt_str(item, "key");
        secrets.push(SecretField { name, key });
    }
    Ok(secrets)
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

pub(super) fn parse_result(node: &saphyr::Yaml<'_>) -> YamlResult<Option<ResultMapping>> {
    let Some(sub) = lookup(node, "result") else {
        return Ok(None);
    };
    if !sub.is_mapping() {
        return Ok(None);
    }

    let value = require_str_in(sub, "value", "result.value")?;
    Ok(Some(ResultMapping { value }))
}

// ---------------------------------------------------------------------------
// Examples
// ---------------------------------------------------------------------------

pub(super) fn parse_examples(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<ExampleAst>> {
    let Some(seq) = lookup(node, "examples").and_then(|v| v.as_vec()) else {
        return Ok(Vec::new());
    };

    let mut examples = Vec::new();
    for item in seq {
        if !item.is_mapping() {
            return Err(YamlError::FieldShape {
                field: "examples",
                expected: "mapping",
            });
        }
        let description = opt_str(item, "description");
        let input = opt_str(item, "input");
        let expected = opt_str(item, "expected");
        examples.push(ExampleAst {
            description,
            input,
            expected,
        });
    }
    Ok(examples)
}
