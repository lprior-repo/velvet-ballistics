#![forbid(unsafe_code)]
use crate::{CompileError, CompileErrors, non_string_key_error};
use saphyr::Yaml;

#[path = "schema_fields.rs"]
mod fields;

use fields::{
    validate_schema_bounds, validate_schema_default, validate_schema_extra, validate_schema_fields,
    validate_schema_flags,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchemaScope {
    Input,
    ObjectField,
}

impl SchemaScope {
    const fn allows_from(self) -> bool {
        matches!(self, Self::Input)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchemaKind {
    Text,
    Number,
    Boolean,
    Object,
    List,
    Any,
}

impl SchemaKind {
    fn from_long_form(value: &str) -> Option<Self> {
        match value {
            "text" => Some(Self::Text),
            "number" => Some(Self::Number),
            "boolean" => Some(Self::Boolean),
            "object" => Some(Self::Object),
            "list" => Some(Self::List),
            "any" => Some(Self::Any),
            _ => None,
        }
    }

    fn from_list_element(value: &str) -> Option<Self> {
        match value {
            "any" => Some(Self::Any),
            "text" => Some(Self::Text),
            "number" => Some(Self::Number),
            "boolean" => Some(Self::Boolean),
            "object" => Some(Self::Object),
            _ => None,
        }
    }
}

fn validate_input_schema(schema: &Yaml<'_>, scope: SchemaScope) -> Vec<CompileError> {
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

fn is_schema_shorthand(value: &str) -> bool {
    matches!(
        value,
        "text"
            | "number"
            | "boolean"
            | "object"
            | "any"
            | "list<any>"
            | "list<text>"
            | "list<number>"
            | "list<boolean>"
    )
}

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

fn reject_unknown_schema_fields(
    mapping: &saphyr::Mapping<'_>,
    scope: SchemaScope,
) -> Vec<CompileError> {
    let mut errors = Vec::new();
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            errors.push(non_string_key_error());
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

fn is_allowed_schema_field(field: &str, scope: SchemaScope) -> bool {
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

fn reject_schema_pattern(mapping: &saphyr::Mapping<'_>) -> Vec<CompileError> {
    if mapping_get(mapping, "pattern").is_some() {
        vec![CompileError::InvalidInputSchema {
            field: "inputs.pattern",
            expected: "unsupported until a bounded regex engine exists",
        }]
    } else {
        Vec::new()
    }
}

fn validate_schema_from(mapping: &saphyr::Mapping<'_>, scope: SchemaScope) -> Vec<CompileError> {
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

fn schema_kind(mapping: &saphyr::Mapping<'_>) -> Result<SchemaKind, CompileError> {
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

fn validate_schema_children(mapping: &saphyr::Mapping<'_>, kind: SchemaKind) -> Vec<CompileError> {
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

fn schema_bool(mapping: &saphyr::Mapping<'_>, field: &str) -> Result<bool, CompileError> {
    match mapping_get(mapping, field) {
        Some(value) => yaml_bool(value).ok_or(CompileError::InvalidInputSchema {
            field: "inputs boolean flag",
            expected: "a boolean",
        }),
        None => Ok(false),
    }
}

fn yaml_bool(node: &Yaml<'_>) -> Option<bool> {
    match node {
        Yaml::Value(saphyr::Scalar::Boolean(value)) => Some(*value),
        _ => None,
    }
}

fn mapping_get<'a>(mapping: &'a saphyr::Mapping<'a>, field: &str) -> Option<&'a Yaml<'a>> {
    mapping.iter().find_map(|(key, value)| match key.as_str() {
        Some(name) if name == field => Some(value),
        _ => None,
    })
}

fn invalid_schema(field: &'static str, expected: &'static str) -> CompileError {
    CompileError::InvalidInputSchema { field, expected }
}

#[cfg(test)]
#[path = "schema/tests.rs"]
mod tests;
