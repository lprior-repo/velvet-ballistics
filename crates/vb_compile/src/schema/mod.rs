#![forbid(unsafe_code)]
//! YAML input-schema validation for the velvet-ballistics compiler.
//!
//! Public entry point:
//! - [`validate_input_schemas`] — validates the `inputs:` mapping of a YAML doc.
//!
//! Internal modules:
//! - [`kind`] — `SchemaKind` enum and token-to-type parsing.
//! - [`scope`] — `SchemaScope` enum controlling field allow-lists per nesting level.
//! - [`fields`] — field-level validators (unknown fields, bounds, defaults, etc.).

pub(crate) mod kind;
pub(crate) mod scope;

mod fields;

use fields::{
    reject_schema_pattern, reject_unknown_schema_fields, schema_kind, validate_schema_bounds,
    validate_schema_children, validate_schema_default, validate_schema_flags, validate_schema_from,
};

use crate::{CompileError, CompileErrors};
use saphyr::Yaml;

// Re-export the domain types so callers don't need to reach into sub-modules.
pub(crate) use scope::SchemaScope;

// ── Entry point ──────────────────────────────────────────────────────────

pub(crate) fn validate_input_schemas(doc: &Yaml<'_>) -> Result<(), CompileErrors> {
    let Some(node) = doc.as_mapping_get("inputs") else {
        return Ok(());
    };
    let Some(mapping) = node.as_mapping() else {
        return Err(CompileErrors(vec![CompileError::FieldShape {
            field: "inputs",
            expected: "a mapping",
        }]));
    };
    let mut errors = Vec::new();
    for (_, schema) in mapping {
        errors.append(&mut validate_input_schema(schema, SchemaScope::Input));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileErrors(errors))
    }
}

// ── Top-level dispatch ───────────────────────────────────────────────────

/// Shorthand token allow-list (used by both parsing and tests).
const ALLOWED_SHORTHANDS: &[&str] = &[
    "text",
    "number",
    "boolean",
    "object",
    "any",
    "list<any>",
    "list<text>",
    "list<number>",
    "list<boolean>",
];

pub(crate) fn validate_input_schema(schema: &Yaml<'_>, scope: SchemaScope) -> Vec<CompileError> {
    if let Some(value) = schema.as_str() {
        validate_schema_shorthand(value)
    } else if let Some(mapping) = schema.as_mapping() {
        validate_schema_mapping(mapping, scope)
    } else {
        vec![CompileError::FieldShape {
            field: "inputs",
            expected: "a mapping of input names to schema strings or schema mappings",
        }]
    }
}

fn validate_schema_shorthand(value: &str) -> Vec<CompileError> {
    if is_schema_shorthand(value) {
        Vec::new()
    } else {
        vec![CompileError::InvalidInputSchema {
            field: "inputs",
            expected: "an allowed schema shorthand",
        }]
    }
}

pub(crate) fn is_schema_shorthand(value: &str) -> bool {
    ALLOWED_SHORTHANDS.contains(&value)
}

/// Full mapping-validator orchestrator.
fn validate_schema_mapping(mapping: &saphyr::Mapping<'_>, scope: SchemaScope) -> Vec<CompileError> {
    let mut errors = Vec::new();
    errors.append(&mut reject_unknown_schema_fields(mapping, scope));
    errors.append(&mut reject_schema_pattern(mapping));
    errors.append(&mut validate_schema_from(mapping, scope));
    let kind = match schema_kind(mapping) {
        Ok(k) => k,
        Err(e) => {
            errors.push(e);
            return errors;
        }
    };
    errors.append(&mut validate_schema_children(mapping, kind));
    errors.append(&mut validate_schema_flags(mapping));
    errors.append(&mut validate_schema_default(mapping, kind));
    errors.append(&mut validate_schema_bounds(mapping, kind));
    errors
}

#[cfg(test)]
mod tests;
