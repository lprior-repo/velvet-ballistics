#![forbid(unsafe_code)]
use crate::{CompileError, CompileErrors, non_string_key_error, validate_public_name};
use saphyr::Yaml;

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


mod validate;
mod tests;
