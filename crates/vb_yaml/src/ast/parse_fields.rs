#![forbid(unsafe_code)]
//! Field parsing logic for inputs, vars, secrets, result, and examples.

use crate::{YamlError, YamlResult};

use super::parse::{lookup, mapping, reject_unknown_fields, sequence};
use super::types::*;

// ---------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------

pub(super) fn parse_inputs(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<InputField>> {
    let Some(inputs) = lookup(node, "inputs") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (key, value) in mapping(inputs, "inputs")? {
        let Some(key) = key.as_str() else {
            return Err(YamlError::FieldShape {
                span: None,
                field: "inputs key",
                expected: "string",
            });
        };
        out.push(InputField {
            key: key.to_string(),
            value: parse_author_value(value)?,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Vars
// ---------------------------------------------------------------------------

pub(super) fn parse_vars(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<VarField>> {
    let Some(vars) = lookup(node, "vars") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (key, value) in mapping(vars, "vars")? {
        let Some(key) = key.as_str() else {
            return Err(YamlError::FieldShape {
                span: None,
                field: "vars key",
                expected: "string",
            });
        };
        out.push(VarField {
            key: key.to_string(),
            value: parse_author_value(value)?,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Secrets
// ---------------------------------------------------------------------------

pub(super) fn parse_secrets(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<SecretField>> {
    let Some(secrets) = lookup(node, "secrets") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (key, value) in mapping(secrets, "secrets")? {
        let Some(key) = key.as_str() else {
            return Err(YamlError::FieldShape {
                span: None,
                field: "secrets key",
                expected: "string",
            });
        };
        let Some(value) = value.as_str() else {
            return Err(YamlError::FieldShape {
                span: None,
                field: "secrets",
                expected: "mapping of non-empty strings",
            });
        };
        if value.is_empty() {
            return Err(YamlError::FieldShape {
                span: None,
                field: "secrets",
                expected: "mapping of non-empty strings",
            });
        }
        out.push(SecretField {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

pub(super) fn parse_result(node: &saphyr::Yaml<'_>) -> YamlResult<Option<ResultMapping>> {
    let Some(sub) = lookup(node, "result") else {
        return Ok(None);
    };
    let mut fields = Vec::new();
    for (key, value) in mapping(sub, "result")? {
        let Some(key) = key.as_str() else {
            return Err(YamlError::FieldShape {
                span: None,
                field: "result key",
                expected: "string",
            });
        };
        fields.push(AuthorEntry {
            key: key.to_string(),
            value: parse_author_value(value)?,
        });
    }
    Ok(Some(ResultMapping { fields }))
}

// ---------------------------------------------------------------------------
// Examples
// ---------------------------------------------------------------------------

pub(super) fn parse_examples(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<ExampleAst>> {
    let Some(node) = lookup(node, "examples") else {
        return Ok(Vec::new());
    };

    let mut examples = Vec::new();
    for item in sequence(node, "examples")? {
        reject_unknown_fields(item, &["description", "input", "expected"])?;
        let description = match lookup(item, "description") {
            Some(v) => Some(
                v.as_str()
                    .ok_or(YamlError::FieldShape {
                        span: None,
                        field: "examples.description",
                        expected: "string",
                    })?
                    .to_string(),
            ),
            None => None,
        };
        let input = match lookup(item, "input") {
            Some(v) => Some(parse_author_value(v)?),
            None => None,
        };
        let expected = match lookup(item, "expected") {
            Some(v) => Some(parse_author_value(v)?),
            None => None,
        };
        examples.push(ExampleAst {
            description,
            input,
            expected,
        });
    }
    Ok(examples)
}

pub(super) fn parse_author_value(node: &saphyr::Yaml<'_>) -> YamlResult<AuthorValue> {
    if node.is_null() {
        Ok(AuthorValue::Null)
    } else if let Some(value) = node.as_bool() {
        Ok(AuthorValue::Bool(value))
    } else if let Some(value) = node.as_integer() {
        Ok(AuthorValue::I64(value))
    } else if let Some(value) = node.as_str() {
        Ok(AuthorValue::Text(value.to_string()))
    } else if let Some(values) = node.as_sequence() {
        let mut out = Vec::new();
        for value in values {
            out.push(parse_author_value(value)?);
        }
        Ok(AuthorValue::Sequence(out))
    } else if let Some(map) = node.as_mapping() {
        let mut out = Vec::new();
        for (key, value) in map {
            let Some(key) = key.as_str() else {
                return Err(YamlError::FieldShape {
                    span: None,
                    field: "mapping key",
                    expected: "string",
                });
            };
            out.push(AuthorEntry {
                key: key.to_string(),
                value: parse_author_value(value)?,
            });
        }
        Ok(AuthorValue::Mapping(out))
    } else {
        Err(YamlError::FieldShape {
            span: None,
            field: "value",
            expected: "author value",
        })
    }
}
