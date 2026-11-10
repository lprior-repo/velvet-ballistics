#![forbid(unsafe_code)]
//! Parsing entry points for the AST.
//!
//! This module provides the high-level parsing functions that convert
//! raw YAML text into typed AST structures.

use saphyr::LoadableYamlNode;

use crate::yaml_error::{YamlError, YamlResult};

use super::parse_fields::{parse_examples, parse_inputs, parse_result, parse_secrets, parse_vars};
use super::parse_steps::parse_steps;
use super::parse_trigger::parse_trigger;
use super::types::*;

// ---------------------------------------------------------------------------
// Helpers that work on &Yaml<'_> using as_mapping_get
//
// The `Yaml` type provides `as_mapping_get(&str)` which returns
// `Option<&Yaml>` and handles lifetime bridging internally. Missing keys
// yield `None`.
// ---------------------------------------------------------------------------

/// Look up a key in a mapping node. Returns `None` for absent keys.
pub(super) fn lookup<'a>(node: &'a saphyr::Yaml<'_>, key: &str) -> Option<&'a saphyr::Yaml<'a>> {
    node.as_mapping_get(key)
}

pub(super) fn mapping<'a>(
    node: &'a saphyr::Yaml<'a>,
    field: &'static str,
) -> YamlResult<&'a saphyr::Mapping<'a>> {
    node.as_mapping().ok_or(YamlError::FieldShape {
        field,
        expected: "mapping",
    })
}

pub(super) fn sequence<'a>(
    node: &'a saphyr::Yaml<'a>,
    field: &'static str,
) -> YamlResult<&'a saphyr::Sequence<'a>> {
    node.as_sequence().ok_or(YamlError::FieldShape {
        field,
        expected: "sequence",
    })
}

pub(super) fn reject_unknown_fields(node: &saphyr::Yaml<'_>, allowed: &[&str]) -> YamlResult<()> {
    for (key, _) in mapping(node, "mapping")? {
        let Some(key) = key.as_str() else {
            return Err(YamlError::FieldShape {
                field: "mapping key",
                expected: "string",
            });
        };
        if !allowed.contains(&key) {
            return Err(YamlError::UnknownField { field: key.into() });
        }
    }
    Ok(())
}

/// Require a non-empty string field.
pub(super) fn require_str(node: &saphyr::Yaml<'_>, field: &'static str) -> YamlResult<String> {
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
pub(super) fn require_scalar_in(
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
                    expected: "string or integer scalar in i64 range",
                }),
            },
        },
    }
}

/// Optional string field. A present field with the wrong type is an error,
/// never silently treated as absent.
pub(super) fn opt_str(node: &saphyr::Yaml<'_>, field: &'static str) -> YamlResult<Option<String>> {
    match lookup(node, field) {
        None => Ok(None),
        Some(v) => match v.as_str() {
            Some(s) => Ok(Some(s.to_string())),
            None => Err(YamlError::FieldShape {
                field,
                expected: "string",
            }),
        },
    }
}

/// Optional control field (`with`, `then`): presence is what the compiler
/// gates on, so a present field of any shape stays present.
pub(super) fn opt_control_str(
    node: &saphyr::Yaml<'_>,
    field: &'static str,
) -> YamlResult<Option<String>> {
    match lookup(node, field) {
        None => Ok(None),
        Some(v) => v
            .as_str()
            .filter(|value| !value.is_empty())
            .map(|value| Some(value.to_owned()))
            .ok_or(YamlError::FieldShape {
                field,
                expected: "a non-empty string",
            }),
    }
}

/// Optional u32 field. A present field with the wrong type or out of range
/// is an error, never silently treated as absent.
pub(super) fn opt_u32(node: &saphyr::Yaml<'_>, field: &'static str) -> YamlResult<Option<u32>> {
    match lookup(node, field) {
        None => Ok(None),
        Some(v) => match v.as_integer() {
            Some(i) => match u32::try_from(i) {
                Ok(n) => Ok(Some(n)),
                Err(_) => Err(YamlError::FieldShape {
                    field,
                    expected: "u32 integer in range 0..=4294967295",
                }),
            },
            None => Err(YamlError::FieldShape {
                field,
                expected: "u32 integer",
            }),
        },
    }
}

/// Require a u16 field.
pub(super) fn require_u16(node: &saphyr::Yaml<'_>, field: &'static str) -> YamlResult<u16> {
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
// Parse entry points
// ---------------------------------------------------------------------------

/// Parse YAML text into a [`WorkflowSource`] AST.
///
/// This is a low-level function. Prefer [`crate::parse_workflow_source`] which
/// runs profile validation first.
pub(crate) fn parse_workflow_ast(text: &str) -> YamlResult<WorkflowSource> {
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
    reject_unknown_fields(
        root,
        &[
            "version", "name", "when", "inputs", "vars", "secrets", "steps", "result", "examples",
        ],
    )?;
    let trigger = parse_trigger(root)?;
    let inputs = parse_inputs(root)?;
    let vars = parse_vars(root)?;
    let secrets = parse_secrets(root)?;
    let steps = parse_steps(root)?;
    let result = parse_result(root)?;
    let examples = parse_examples(root)?;

    Ok(WorkflowSource::new(WorkflowSourceParts {
        version,
        name,
        trigger,
        inputs,
        vars,
        secrets,
        steps,
        result,
        examples,
    }))
}
