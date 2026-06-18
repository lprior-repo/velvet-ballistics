#![forbid(unsafe_code)]
//! Field-level schema validators.
//!
//! Each function accepts a `&saphyr::Mapping` and returns a `Vec<CompileError>`.
//! The caller orchestrates the full validation pipeline.

use crate::schema::kind::SchemaKind;
use crate::schema::scope::SchemaScope;
use crate::{CompileError, validate_public_name};
use saphyr::Yaml;

// ── Unknown field rejection ──────────────────────────────────────────────

pub(crate) fn reject_unknown_schema_fields(
    mapping: &saphyr::Mapping<'_>,
    scope: SchemaScope,
) -> Vec<CompileError> {
    let mut errors = Vec::new();
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            errors.push(crate::non_string_key_error());
            continue;
        };
        errors.append(&mut reject_unknown_schema_field(field, scope));
    }
    errors
}

fn reject_unknown_schema_field(field: &str, scope: SchemaScope) -> Vec<CompileError> {
    if is_allowed_schema_field(field, scope) {
        Vec::new()
    } else {
        vec![CompileError::UnknownInputSchemaField {
            field: Box::<str>::from(field),
        }]
    }
}

pub(crate) fn is_allowed_schema_field(field: &str, scope: SchemaScope) -> bool {
    const FIELDS: &[&str] = &[
        "is",
        "of",
        "fields",
        "extra",
        "optional",
        "nullable",
        "default",
        "min",
        "max",
        "min_length",
        "max_length",
        "pattern",
        "secret",
    ];
    FIELDS.contains(&field) || (field == "from" && scope.allows_from())
}

// ── Pattern rejection ────────────────────────────────────────────────────

pub(crate) fn reject_schema_pattern(mapping: &saphyr::Mapping<'_>) -> Vec<CompileError> {
    if mapping_get(mapping, "pattern").is_some() {
        vec![CompileError::InvalidInputSchema {
            field: "inputs.pattern",
            expected: "unsupported until a bounded regex engine exists",
        }]
    } else {
        Vec::new()
    }
}

// ── `from` field validation ──────────────────────────────────────────────

pub(crate) fn validate_schema_from(
    mapping: &saphyr::Mapping<'_>,
    scope: SchemaScope,
) -> Vec<CompileError> {
    let Some(value) = mapping_get(mapping, "from") else {
        return Vec::new();
    };
    if !scope.allows_from() {
        return vec![invalid_schema(
            "inputs.from",
            "top-level input schemas only",
        )];
    }
    match value.as_str() {
        Some(text) if !text.is_empty() => Vec::new(),
        _ => vec![invalid_schema("inputs.from", "a non-empty string")],
    }
}

// ── Schema kind extraction ───────────────────────────────────────────────

pub(crate) fn schema_kind(mapping: &saphyr::Mapping<'_>) -> Result<SchemaKind, CompileError> {
    let Some(value) = mapping_get(mapping, "is") else {
        return Err(invalid_schema(
            "inputs.is",
            "one of text, number, boolean, object, list, any",
        ));
    };
    match value.as_str().and_then(SchemaKind::from_long_form) {
        Some(kind) => Ok(kind),
        None => Err(invalid_schema(
            "inputs.is",
            "one of text, number, boolean, object, list, any",
        )),
    }
}

// ── Children validation: `of`, `fields`, `extra` ────────────────────────

pub(crate) fn validate_schema_children(
    mapping: &saphyr::Mapping<'_>,
    kind: SchemaKind,
) -> Vec<CompileError> {
    let mut errors = Vec::new();
    errors.append(&mut validate_schema_of(mapping, kind));
    errors.append(&mut validate_schema_fields(mapping, kind));
    errors.append(&mut validate_schema_extra(mapping, kind));
    errors
}

fn validate_schema_of(mapping: &saphyr::Mapping<'_>, kind: SchemaKind) -> Vec<CompileError> {
    let Some(value) = mapping_get(mapping, "of") else {
        return require_list_element_schema(kind);
    };
    if kind != SchemaKind::List {
        return vec![invalid_schema("inputs.of", "present only when is is list")];
    }
    match value.as_str().and_then(SchemaKind::from_list_element) {
        Some(_) => Vec::new(),
        None => vec![invalid_schema(
            "inputs.of",
            "one of any, text, number, boolean, object",
        )],
    }
}

fn require_list_element_schema(kind: SchemaKind) -> Vec<CompileError> {
    if kind == SchemaKind::List {
        vec![invalid_schema("inputs.of", "required when is is list")]
    } else {
        Vec::new()
    }
}

fn validate_schema_fields(mapping: &saphyr::Mapping<'_>, kind: SchemaKind) -> Vec<CompileError> {
    let Some(value) = mapping_get(mapping, "fields") else {
        return Vec::new();
    };
    if kind != SchemaKind::Object {
        return vec![invalid_schema(
            "inputs.fields",
            "present only when is is object",
        )];
    }
    validate_object_schema_fields(value)
}

fn validate_object_schema_fields(value: &Yaml<'_>) -> Vec<CompileError> {
    let mut errors = Vec::new();
    let Some(fields) = value.as_mapping() else {
        return vec![invalid_schema(
            "inputs.fields",
            "a mapping of field names to schemas",
        )];
    };
    for (key, field_schema) in fields {
        let Some(field) = key.as_str() else {
            errors.push(crate::non_string_key_error());
            continue;
        };
        if let Err(e) = validate_public_name("inputs.fields", field) {
            errors.push(e);
        }
        errors.append(&mut super::validate_input_schema(
            field_schema,
            SchemaScope::ObjectField,
        ));
    }
    errors
}

fn validate_schema_extra(mapping: &saphyr::Mapping<'_>, kind: SchemaKind) -> Vec<CompileError> {
    let Some(value) = mapping_get(mapping, "extra") else {
        return Vec::new();
    };
    if kind != SchemaKind::Object {
        return vec![invalid_schema(
            "inputs.extra",
            "present only when is is object",
        )];
    }
    match value.as_str() {
        Some("allow" | "reject") => Vec::new(),
        _ => vec![invalid_schema("inputs.extra", "allow or reject")],
    }
}

// ── Boolean flags: `optional`, `nullable`, `secret` ─────────────────────

pub(crate) fn validate_schema_flags(mapping: &saphyr::Mapping<'_>) -> Vec<CompileError> {
    let mut errors = Vec::new();
    for field in ["optional", "nullable", "secret"] {
        errors.append(&mut validate_schema_bool_field(mapping, field));
    }
    errors
}

fn validate_schema_bool_field(
    mapping: &saphyr::Mapping<'_>,
    field: &'static str,
) -> Vec<CompileError> {
    match mapping_get(mapping, field) {
        Some(value) if yaml_bool(value).is_none() => {
            vec![invalid_schema("inputs boolean flag", "a boolean")]
        }
        _ => Vec::new(),
    }
}

// ── Default value validation ─────────────────────────────────────────────

pub(crate) fn validate_schema_default(
    mapping: &saphyr::Mapping<'_>,
    kind: SchemaKind,
) -> Vec<CompileError> {
    let Some(value) = mapping_get(mapping, "default") else {
        return Vec::new();
    };
    if matches!(value, Yaml::Value(saphyr::Scalar::Null)) {
        let nullable = match schema_bool(mapping, "nullable") {
            Ok(b) => b,
            Err(e) => return vec![e],
        };
        return validate_null_default(kind, nullable);
    }
    if default_matches_kind(value, kind) {
        Vec::new()
    } else {
        vec![invalid_schema(
            "inputs.default",
            "a value matching the declared schema type",
        )]
    }
}

fn validate_null_default(kind: SchemaKind, nullable: bool) -> Vec<CompileError> {
    if nullable || kind == SchemaKind::Any {
        Vec::new()
    } else {
        vec![invalid_schema(
            "inputs.default",
            "null only when nullable is true or is is any",
        )]
    }
}

fn default_matches_kind(value: &Yaml<'_>, kind: SchemaKind) -> bool {
    match kind {
        SchemaKind::Text => value.as_str().is_some(),
        SchemaKind::Number => value.as_integer().is_some(),
        SchemaKind::Boolean => yaml_bool(value).is_some(),
        SchemaKind::Object => value.is_mapping(),
        SchemaKind::List => value.as_sequence().is_some(),
        SchemaKind::Any => true,
    }
}

// ── Bounds: `min`/`max` (numeric & list) and `min_length`/`max_length` (text) ──

pub(crate) fn validate_schema_bounds(
    mapping: &saphyr::Mapping<'_>,
    kind: SchemaKind,
) -> Vec<CompileError> {
    let mut errors = Vec::new();
    errors.append(&mut validate_min_max_bounds(mapping, kind));
    errors.append(&mut validate_text_length_bounds(mapping, kind));
    errors
}

fn validate_min_max_bounds(mapping: &saphyr::Mapping<'_>, kind: SchemaKind) -> Vec<CompileError> {
    let min = match optional_integer_schema_field(mapping, "min") {
        Ok(v) => v,
        Err(e) => return vec![e],
    };
    let max = match optional_integer_schema_field(mapping, "max") {
        Ok(v) => v,
        Err(e) => return vec![e],
    };
    if min.is_none() && max.is_none() {
        return Vec::new();
    }
    let mut errors = Vec::new();
    errors.append(&mut validate_min_max_kind(kind));
    errors.append(&mut validate_list_bounds(kind, min, max));
    errors.append(&mut validate_ordered_bounds(min, max, "inputs.min/max"));
    errors
}

fn validate_min_max_kind(kind: SchemaKind) -> Vec<CompileError> {
    if kind.accepts_numeric_bounds() {
        Vec::new()
    } else {
        vec![invalid_schema(
            "inputs.min/max",
            "present only for number or list schemas",
        )]
    }
}

fn validate_list_bounds(kind: SchemaKind, min: Option<i64>, max: Option<i64>) -> Vec<CompileError> {
    if kind == SchemaKind::List && [min, max].into_iter().flatten().any(|value| value < 0) {
        vec![invalid_schema(
            "inputs.min/max",
            "non-negative list length bounds",
        )]
    } else {
        Vec::new()
    }
}

fn validate_text_length_bounds(
    mapping: &saphyr::Mapping<'_>,
    kind: SchemaKind,
) -> Vec<CompileError> {
    let min = match optional_integer_schema_field(mapping, "min_length") {
        Ok(v) => v,
        Err(e) => return vec![e],
    };
    let max = match optional_integer_schema_field(mapping, "max_length") {
        Ok(v) => v,
        Err(e) => return vec![e],
    };
    if min.is_none() && max.is_none() {
        return Vec::new();
    }
    let mut errors = Vec::new();
    errors.append(&mut validate_text_bounds_kind(kind));
    errors.append(&mut validate_text_bounds_values(min, max));
    errors.append(&mut validate_ordered_bounds(
        min,
        max,
        "inputs.min_length/max_length",
    ));
    errors
}

fn validate_text_bounds_kind(kind: SchemaKind) -> Vec<CompileError> {
    if kind.is_text() {
        Vec::new()
    } else {
        vec![invalid_schema(
            "inputs.min_length/max_length",
            "present only for text schemas",
        )]
    }
}

fn validate_text_bounds_values(min: Option<i64>, max: Option<i64>) -> Vec<CompileError> {
    if [min, max].into_iter().flatten().any(|value| value < 0) {
        vec![invalid_schema(
            "inputs.min_length/max_length",
            "non-negative text length bounds",
        )]
    } else {
        Vec::new()
    }
}

fn validate_ordered_bounds(
    min: Option<i64>,
    max: Option<i64>,
    field: &'static str,
) -> Vec<CompileError> {
    match (min, max) {
        (Some(min_val), Some(max_val)) if min_val > max_val => {
            vec![invalid_schema(field, "min less than or equal to max")]
        }
        _ => Vec::new(),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

pub(crate) fn optional_integer_schema_field(
    mapping: &saphyr::Mapping<'_>,
    field: &'static str,
) -> Result<Option<i64>, CompileError> {
    match mapping_get(mapping, field) {
        Some(value) => value
            .as_integer()
            .map(Some)
            .ok_or(invalid_schema(field, "an integer")),
        None => Ok(None),
    }
}

pub(crate) fn schema_bool(
    mapping: &saphyr::Mapping<'_>,
    field: &str,
) -> Result<bool, CompileError> {
    match mapping_get(mapping, field) {
        Some(value) => yaml_bool(value).ok_or(CompileError::InvalidInputSchema {
            field: "inputs boolean flag",
            expected: "a boolean",
        }),
        None => Ok(false),
    }
}

pub(crate) fn yaml_bool(node: &Yaml<'_>) -> Option<bool> {
    match node {
        Yaml::Value(saphyr::Scalar::Boolean(value)) => Some(*value),
        _ => None,
    }
}

pub(crate) fn mapping_get<'a>(
    mapping: &'a saphyr::Mapping<'a>,
    field: &str,
) -> Option<&'a Yaml<'a>> {
    mapping.iter().find_map(|(key, value)| match key.as_str() {
        Some(name) if name == field => Some(value),
        _ => None,
    })
}

pub(crate) fn invalid_schema(field: &'static str, expected: &'static str) -> CompileError {
    CompileError::InvalidInputSchema { field, expected }
}
