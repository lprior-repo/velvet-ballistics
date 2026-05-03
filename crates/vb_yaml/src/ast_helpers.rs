//! YAML node lookup and field-extraction helpers.
//!
//! These helpers work on `&saphyr::Yaml<'_>` using `as_mapping_get`.
//! Missing keys yield `None`; the helpers convert that to the appropriate
//! [`YamlError`] variant.

use saphyr::Yaml;

use crate::{YamlError, YamlResult, ast::ScalarValue};

// ---------------------------------------------------------------------------
// Lookup helpers
// ---------------------------------------------------------------------------

/// Look up a key in a mapping node. Returns `None` for absent keys.
pub(super) fn lookup<'a>(node: &'a Yaml<'_>, key: &str) -> Option<&'a Yaml<'a>> {
    node.as_mapping_get(key)
}

/// Require a non-empty string field.
pub(super) fn require_str(node: &Yaml<'_>, field: &'static str) -> YamlResult<String> {
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
pub(super) fn require_str_in(
    node: &Yaml<'_>,
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
pub(super) fn require_scalar_in(
    node: &Yaml<'_>,
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
pub(super) fn opt_str(node: &Yaml<'_>, field: &str) -> Option<String> {
    lookup(node, field).and_then(|v| v.as_str().map(std::string::ToString::to_string))
}

/// Optional u32 field.
pub(super) fn opt_u32(node: &Yaml<'_>, field: &str) -> Option<u32> {
    lookup(node, field).and_then(|v| v.as_integer().and_then(|i| u32::try_from(i).ok()))
}

/// Require a u16 field.
pub(super) fn require_u16(node: &Yaml<'_>, field: &'static str) -> YamlResult<u16> {
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
