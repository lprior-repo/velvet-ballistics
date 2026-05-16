//! Shared AST types for the workflow definition language.

use crate::{YamlError, YamlResult};

// ---------------------------------------------------------------------------
// Scalar value
// ---------------------------------------------------------------------------

/// A scalar YAML value used in step fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalarValue {
    /// A string value.
    String(String),
    /// An integer value.
    Integer(i64),
}

// ---------------------------------------------------------------------------
// Field types
// ---------------------------------------------------------------------------

/// An input field declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputField {
    /// Field name.
    pub name: String,
    /// Field type annotation (optional).
    pub field_type: Option<String>,
    /// Default value expression (optional).
    pub default: Option<String>,
}

/// A variable field declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarField {
    /// Variable name.
    pub name: String,
    /// Initial value expression.
    pub value: Option<String>,
}

/// A secret reference declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretField {
    /// Secret name.
    pub name: String,
    /// External key path (optional).
    pub key: Option<String>,
}

/// Result mapping at the end of a workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultMapping {
    /// Result expression.
    pub value: String,
}

/// An inline example / test case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleAst {
    /// Example description.
    pub description: Option<String>,
    /// Input bindings for the example.
    pub input: Option<String>,
    /// Expected result expression.
    pub expected: Option<String>,
}

// ---------------------------------------------------------------------------
// Step-level types
// ---------------------------------------------------------------------------

/// Retry policy for a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum retry attempts.
    pub max_attempts: u16,
    /// Delay between retries (expression or duration string).
    pub delay: Option<String>,
}

/// Error handler attached to a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorHandlerAst {
    /// Handler label or step reference.
    pub handler: String,
}

// ---------------------------------------------------------------------------
// YAML helpers
// ---------------------------------------------------------------------------

/// Look up a key in a mapping node. Returns `None` for absent keys.
pub fn lookup<'a>(node: &'a saphyr::Yaml<'_>, key: &str) -> Option<&'a saphyr::Yaml<'a>> {
    node.as_mapping_get(key)
}

/// Require a non-empty string field.
pub fn require_str(node: &saphyr::Yaml<'_>, field: &'static str) -> YamlResult<String> {
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

/// Require a non-empty string from a sub-node, with a context label.
pub fn require_str_in(
    node: &saphyr::Yaml<'_>,
    field: &str,
    context: &'static str,
) -> YamlResult<String> {
    match lookup(node, field) {
        None => Err(YamlError::MissingField { field: context }),
        Some(v) => match v.as_str() {
            Some(s) if !s.is_empty() => Ok(s.to_string()),
            _ => Err(YamlError::FieldShape {
                field: context,
                expected: "non-empty string",
            }),
        },
    }
}

/// Require a scalar string or integer from a sub-node, with a context label.
pub fn require_scalar_in(
    node: &saphyr::Yaml<'_>,
    field: &str,
    context: &'static str,
) -> YamlResult<ScalarValue> {
    match lookup(node, field) {
        None => Err(YamlError::MissingField { field: context }),
        Some(v) => match v.as_str() {
            Some(s) if !s.is_empty() => Ok(ScalarValue::String(s.to_string())),
            _ => match v.as_integer() {
                Some(i) => Ok(ScalarValue::Integer(i)),
                None => Err(YamlError::FieldShape {
                    field: context,
                    expected: "string or integer scalar",
                }),
            },
        },
    }
}

/// Optional string field.
pub fn opt_str(node: &saphyr::Yaml<'_>, field: &str) -> Option<String> {
    lookup(node, field).and_then(|v| v.as_str().map(std::string::ToString::to_string))
}

/// Optional u32 field.
pub fn opt_u32(node: &saphyr::Yaml<'_>, field: &str) -> Option<u32> {
    lookup(node, field).and_then(|v| v.as_integer().and_then(|i| u32::try_from(i).ok()))
}

/// Require a u16 field.
pub fn require_u16(node: &saphyr::Yaml<'_>, field: &'static str) -> YamlResult<u16> {
    match lookup(node, field) {
        None => Err(YamlError::MissingField { field }),
        Some(v) => {
            v.as_integer()
                .and_then(|i| u16::try_from(i).ok())
                .ok_or(YamlError::FieldShape {
                    field,
                    expected: "u16 integer",
                })
        }
    }
}

// ---------------------------------------------------------------------------
// Field parsers
// ---------------------------------------------------------------------------

/// Parse the inputs list.
pub fn parse_inputs(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<InputField>> {
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

/// Parse the vars list.
pub fn parse_vars(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<VarField>> {
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

/// Parse the secrets list.
pub fn parse_secrets(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<SecretField>> {
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
