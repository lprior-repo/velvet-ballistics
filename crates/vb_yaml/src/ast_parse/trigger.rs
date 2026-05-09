#![forbid(unsafe_code)]
//! Trigger declaration parsing.

use crate::{YamlError, YamlResult, ast::TriggerAst};

use super::super::ast_helpers::{lookup, require_str as helper_require_str, require_str_in};

/// Wrapper for top-level field extraction (used by workflow.rs).
pub(super) fn require_str(node: &saphyr::Yaml<'_>, field: &'static str) -> YamlResult<String> {
    helper_require_str(node, field)
}

/// Parse the trigger declaration from a workflow root node.
pub(super) fn parse_trigger(node: &saphyr::Yaml<'_>) -> YamlResult<TriggerAst> {
    if let Some(when_val) = lookup(node, "when") {
        return parse_when_trigger(when_val);
    }
    Err(YamlError::MissingField { field: "when" })
}

fn parse_when_trigger(when_val: &saphyr::Yaml<'_>) -> YamlResult<TriggerAst> {
    if let Some(manual_val) = lookup(when_val, "manual") {
        if manual_val.is_mapping() {
            return Ok(TriggerAst::Manual);
        }
        return Err(YamlError::FieldShape {
            field: "when.manual",
            expected: "mapping",
        });
    }

    if let Some(ipc_val) = lookup(when_val, "ipc") {
        let name = require_str_in(ipc_val, "name", "when.ipc.name")?;
        return Ok(TriggerAst::Ipc { name });
    }

    if lookup(when_val, "http").is_some() {
        return Err(YamlError::UnsupportedFeature {
            feature: "http trigger",
        });
    }

    Err(YamlError::FieldShape {
        field: "when",
        expected: "manual or ipc mapping",
    })
}
