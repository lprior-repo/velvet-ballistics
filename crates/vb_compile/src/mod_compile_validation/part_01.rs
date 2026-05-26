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

pub(crate) const WORKFLOW_VERSION: &str = "velvet-ballistics/v1";

pub(crate) fn reject_known_canonical_text_gaps(text: &str) -> Result<(), CompileError> {
    if text.contains("event: \"\"") {
        Err(CompileError::CanonicalYaml {
            category: "field_shape",
            message: Box::from("wait.event must be non-empty"),
            mark: SourceMark::unavailable(),
        })
    } else {
        Ok(())
    }
}

pub(crate) fn canonical_yaml_error(error: vb_yaml::YamlError) -> CompileError {
    let mark = match error.span() {
        Some(span) => SourceMark {
            index: span.start_offset,
            end_index: span.end_offset,
            line: span.start_line,
            column: span.start_col,
            available: true,
        },
        None => SourceMark::unavailable(),
    };
    CompileError::CanonicalYaml {
        category: yaml_error_category(&error),
        message: error.to_string().into_boxed_str(),
        mark,
    }
}

pub(crate) fn yaml_error_category(error: &vb_yaml::YamlError) -> &'static str {
    match error {
        vb_yaml::YamlError::UnsupportedTrigger { .. }
        | vb_yaml::YamlError::UnsupportedFeature { .. }
        | vb_yaml::YamlError::AnchorAliasMerge { .. }
        | vb_yaml::YamlError::CustomTag { .. }
        | vb_yaml::YamlError::BinaryScalar { .. }
        | vb_yaml::YamlError::AmbiguousScalar { .. }
        | vb_yaml::YamlError::ForbiddenFeature { .. } => "forbidden_feature",
        vb_yaml::YamlError::DuplicateKey { .. } => "duplicate_key",
        vb_yaml::YamlError::MultipleDocuments { .. } => "document_count",
        vb_yaml::YamlError::SourceTooLarge { .. }
        | vb_yaml::YamlError::NestingTooDeep { .. }
        | vb_yaml::YamlError::NodeLimitExceeded { .. }
        | vb_yaml::YamlError::ScalarTooLong { .. }
        | vb_yaml::YamlError::SequenceTooLong { .. }
        | vb_yaml::YamlError::MappingTooLarge { .. } => "limit_exceeded",
        vb_yaml::YamlError::UnknownField { .. } => "unknown_field",
        vb_yaml::YamlError::EmptySource => "empty_source",
        vb_yaml::YamlError::MissingField { .. } => "missing_field",
        vb_yaml::YamlError::FieldShape { .. } => "field_shape",
        vb_yaml::YamlError::ParseError { .. } => "parse_error",
        _ => "parse_error",
    }
}

pub(crate) fn validate_canonical_compile_scope(
    source: &vb_yaml::ast::WorkflowSource,
) -> Result<(), CompileErrors> {
    let mut errors = Vec::new();
    if !source.inputs().is_empty() {
        errors.push(CompileError::UnsupportedTopLevelDeclaration { field: "inputs" });
    }
    if !source.vars().is_empty() {
        errors.push(CompileError::UnsupportedTopLevelDeclaration { field: "vars" });
    }
    if !source.secrets().is_empty() {
        errors.push(CompileError::UnsupportedTopLevelDeclaration { field: "secrets" });
    }
    if !source.examples().is_empty() {
        errors.push(CompileError::UnsupportedTopLevelDeclaration { field: "examples" });
    }
    if source.result().is_some() {
        errors.push(CompileError::UnsupportedTopLevelResult);
    }
    let mut step_ids = HashSet::with_capacity(source.steps().len());
    for (index, step) in source.steps().iter().enumerate() {
        if !step_ids.insert(step.id.as_str()) {
            errors.push(CompileError::DuplicateStepId {
                id: Box::from(step.id.as_str()),
            });
        }
        if step.name.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("name"),
            });
        }
        if step.condition.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("if"),
            });
        }
        if step.with.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("with"),
            });
        }
        if step.retry.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("try_again"),
            });
        }
        if step.on_error.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("on_error"),
            });
        }
        if step.then.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("then"),
            });
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileErrors(errors))
    }
}

pub(crate) fn checked_utf8(source: &[u8], limits: YamlLimits) -> Result<&str, CompileError> {
    if source.len() > limits.max_source_bytes {
        return Err(CompileError::SourceTooLarge {
            actual: source.len(),
            limit: limits.max_source_bytes,
        });
    }
    let text = str::from_utf8(source)?;
    if text.trim().is_empty() {
        Err(CompileError::EmptySource)
    } else {
        Ok(text)
    }
}

pub(crate) fn single_document<'a>(docs: &'a [Yaml<'a>]) -> Result<&'a Yaml<'a>, CompileError> {
    match docs {
        [doc] => Ok(doc),
        _ => Err(CompileError::DocumentCount { count: docs.len() }),
    }
}

pub(crate) fn reject_duplicate_mapping_keys(text: &str) -> Result<(), CompileError> {
    let mut parser = Parser::new_from_str(text);

    while let Some((event, mark)) = parser.next_event().transpose()? {
        validate_duplicate_keys_in_started_node(event, mark, &mut parser)?;
    }

    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
pub(super) fn validate_duplicate_keys_in_started_node<'input>(
    event: Event<'input>,
    mark: Span,
    parser: &mut Parser<'input, StrInput<'input>>,
) -> Result<(), CompileError> {
    match event {
        Event::MappingStart(_, _) => validate_duplicate_keys_in_mapping(parser),
        Event::SequenceStart(_, _) => validate_duplicate_keys_in_sequence(parser),
        Event::Alias(_) => Err(CompileError::AliasForbidden {
            mark: SourceMark::from_parser_span(mark),
        }),
        _ => Ok(()),
    }
}

pub(super) fn validate_duplicate_keys_in_mapping<'input>(
    parser: &mut Parser<'input, StrInput<'input>>,
) -> Result<(), CompileError> {
    let mut seen = HashSet::new();
    loop {
        let Some((key_event, key_mark)) = parser.next_event().transpose()? else {
            return Ok(());
        };
        if key_event == Event::MappingEnd {
            return Ok(());
        }
        validate_unique_mapping_key(key_event, key_mark, &mut seen)?;
        let Some((value_event, value_mark)) = parser.next_event().transpose()? else {
            return Ok(());
        };
        validate_duplicate_keys_in_started_node(value_event, value_mark, parser)?;
    }
}

pub(super) fn validate_unique_mapping_key(
    event: Event<'_>,
    mark: Span,
    seen: &mut HashSet<Box<str>>,
) -> Result<(), CompileError> {
    let key = mapping_key_text(event, mark)?;
    let duplicate = key.clone();
    if seen.insert(key) {
        Ok(())
    } else {
        Err(CompileError::DuplicateKey {
            key: duplicate,
            mark: SourceMark::from_parser_span(mark),
        })
    }
}

pub(super) fn validate_duplicate_keys_in_sequence<'input>(
    parser: &mut Parser<'input, StrInput<'input>>,
) -> Result<(), CompileError> {
    loop {
        let Some((event, mark)) = parser.next_event().transpose()? else {
            return Ok(());
        };
        if event == Event::SequenceEnd {
            return Ok(());
        }
        validate_duplicate_keys_in_started_node(event, mark, parser)?;
    }
}

pub(super) fn mapping_key_text(event: Event<'_>, mark: Span) -> Result<Box<str>, CompileError> {
    let source_mark = SourceMark::from_parser_span(mark);
    match event {
        Event::Scalar(value, style, _, tag) => {
            let key = Yaml::value_from_cow_and_metadata(value, style, tag.as_ref());
            match key.as_str() {
                Some("<<") => Err(CompileError::MergeKeyForbidden { mark: source_mark }),
                Some(value) => Ok(Box::<str>::from(value)),
                None => Err(CompileError::NonStringKey { mark: source_mark }),
            }
        }
        Event::Alias(_) => Err(CompileError::AliasForbidden { mark: source_mark }),
        _ => Err(CompileError::NonStringKey { mark: source_mark }),
    }
}
