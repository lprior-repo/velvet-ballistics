//! Field-list parsing: inputs, vars, secrets.

use crate::{YamlError, YamlResult, ast::{InputField, VarField, SecretField}};

use super::super::ast_helpers::{lookup, require_str_in, opt_str};

/// Parse the `inputs` list from a workflow root node.
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

/// Parse the `vars` list from a workflow root node.
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

/// Parse the `secrets` list from a workflow root node.
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
