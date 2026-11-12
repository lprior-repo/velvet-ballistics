#![allow(unused_imports)]
use super::*;
use crate::limits::YamlLimits;
use crate::mod_compile_errors::non_string_key_error;
use crate::mod_compile_errors::{CompileError, CompileErrors, SourceMark};
use saphyr::Yaml;
use saphyr_parser::{Event, Parser, Span, StrInput};
use std::collections::HashSet;
use std::str;
use vb_core::{ConstValue, SlotIdx, StepIdx};

pub(super) fn validate_optional_top_level_shapes(doc: &Yaml<'_>) -> Result<(), CompileError> {
    optional_inputs_mapping(doc)?;
    optional_vars_mapping(doc)?;
    optional_secret_mapping(doc)?;
    optional_examples_sequence(doc)
}

pub(super) fn optional_inputs_mapping(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("inputs") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "inputs",
        expected: "a mapping",
    })?;
    for (key, _) in mapping {
        let Some(name) = key.as_str() else {
            return Err(non_string_key_error());
        };
        validate_public_name("inputs", name)?;
    }
    Ok(())
}

pub(super) fn optional_vars_mapping(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("vars") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "vars",
        expected: "a mapping",
    })?;
    for (key, value) in mapping {
        let Some(name) = key.as_str() else {
            return Err(non_string_key_error());
        };
        validate_public_name("vars", name)?;
        validate_variable_literal(name, value)?;
    }
    Ok(())
}

fn validate_variable_literal(name: &str, value: &Yaml<'_>) -> Result<(), CompileError> {
    if value.as_str().is_some() {
        return Ok(());
    }
    match value {
        Yaml::Sequence(_) | Yaml::Mapping(_) | Yaml::Tagged(..) => {
            Err(CompileError::ExpressionLoweringUnsupported {
                feature: format!(
                    "vars.{name} currently requires a null, boolean, integer, or text static literal"
                ).into_boxed_str(),
            })
        }
        _ => slot_value(value, 0).map(|_| ()),
    }
}

pub(super) fn optional_secret_mapping(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("secrets") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "secrets",
        expected: "a mapping",
    })?;
    for (key, value) in mapping {
        let Some(name) = key.as_str() else {
            return Err(non_string_key_error());
        };
        validate_public_name("secrets", name)?;
        if value.as_str().is_none() {
            return Err(CompileError::FieldShape {
                field: "secrets",
                expected: "a mapping of secret names to environment variable names",
            });
        }
    }
    Ok(())
}

pub(super) fn optional_examples_sequence(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("examples") else {
        return Ok(());
    };
    let examples = node.as_sequence().ok_or(CompileError::FieldShape {
        field: "examples",
        expected: "a sequence",
    })?;
    for example in examples {
        if !example.is_mapping() {
            return Err(CompileError::FieldShape {
                field: "examples",
                expected: "a sequence of mappings",
            });
        }
        let name = required_example_name(example)?;
        validate_public_name("examples", name)?;
    }
    Ok(())
}

pub(super) fn required_example_name<'a>(example: &'a Yaml<'a>) -> Result<&'a str, CompileError> {
    let name = example
        .as_mapping_get("name")
        .ok_or(CompileError::MissingField {
            field: "examples.name",
        })?;
    name.as_str().ok_or(CompileError::FieldShape {
        field: "examples.name",
        expected: "a string",
    })
}

#[cfg(test)]
mod source_text_tests {
    use super::*;

    #[test]
    fn variable_text_validation_accepts_empty_and_utf8_bytes() {
        for text in ["", "é", " key "] {
            let value = Yaml::Value(saphyr::Scalar::String(text.into()));
            assert_eq!(validate_variable_literal("text", &value), Ok(()));
        }
    }

    #[test]
    fn variable_containers_retain_exact_unsupported_error() {
        for value in [
            Yaml::Sequence(Vec::new()),
            Yaml::Mapping(Default::default()),
        ] {
            assert_eq!(validate_variable_literal("items", &value), Err(
                CompileError::ExpressionLoweringUnsupported {
                    feature: "vars.items currently requires a null, boolean, integer, or text static literal".into(),
                },
            ));
        }
    }

    #[test]
    fn legacy_scalar_variable_validation_is_unchanged() {
        for value in [
            Yaml::Value(saphyr::Scalar::Null),
            Yaml::Value(saphyr::Scalar::Boolean(true)),
            Yaml::Value(saphyr::Scalar::Integer(7)),
        ] {
            assert_eq!(validate_variable_literal("scalar", &value), Ok(()));
        }
    }
}
