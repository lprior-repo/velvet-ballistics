use crate::limits::YamlLimits;
use crate::mod_compile_errors::{CompileError, CompileErrors, SourceMark};
use saphyr::Yaml;
use saphyr_parser::{Event, Parser, Span, StrInput};
use std::collections::HashSet;
use std::str;

pub(crate) const WORKFLOW_VERSION: &str = "velvet-ballistics/v1";

pub(crate) fn reject_known_canonical_text_gaps(text: &str) -> Result<(), CompileError> {
    if text.contains("event: \"\"") {
        Err(CompileError::CanonicalYaml {
            category: "field_shape",
            message: Box::from("wait.event must be non-empty"),
        })
    } else {
        Ok(())
    }
}

pub(crate) fn canonical_yaml_error(error: crate::YamlError) -> CompileError {
    match error {
        crate::YamlError::SourceTooLarge { size, max } => {
            return CompileError::SourceTooLarge {
                actual: size,
                limit: max,
            };
        }
        crate::YamlError::NestingTooDeep { depth, max } => {
            return CompileError::DepthLimit { depth, limit: max };
        }
        crate::YamlError::NodeLimitExceeded { max, .. } => {
            return CompileError::NodeLimit { limit: max };
        }
        crate::YamlError::ScalarTooLong { len, max } => {
            return CompileError::ScalarLimit {
                actual: len,
                limit: max,
            };
        }
        crate::YamlError::SequenceTooLong { len, max } => {
            return CompileError::SequenceLimit {
                actual: len,
                limit: max,
            };
        }
        crate::YamlError::MappingTooLarge { count, max } => {
            return CompileError::MappingLimit {
                actual: count,
                limit: max,
            };
        }
        _ => {}
    }
    CompileError::CanonicalYaml {
        category: yaml_error_category(&error),
        message: error.to_string().into_boxed_str(),
    }
}

pub(crate) fn yaml_error_category(error: &crate::YamlError) -> &'static str {
    match error {
        crate::YamlError::UnsupportedTrigger { .. }
        | crate::YamlError::UnsupportedFeature { .. }
        | crate::YamlError::AnchorAliasMerge
        | crate::YamlError::CustomTag { .. }
        | crate::YamlError::BinaryScalar
        | crate::YamlError::AmbiguousScalar { .. }
        | crate::YamlError::ForbiddenFeature { .. } => "forbidden_feature",
        crate::YamlError::DuplicateKey { .. } | crate::YamlError::DuplicateKeyAt { .. } => {
            "duplicate_key"
        }
        crate::YamlError::MergeKeyForbidden { .. } | crate::YamlError::UnsupportedEncoding => {
            "forbidden_feature"
        }
        crate::YamlError::MultipleDocuments { .. } => "document_count",
        crate::YamlError::SourceTooLarge { .. }
        | crate::YamlError::NestingTooDeep { .. }
        | crate::YamlError::NodeLimitExceeded { .. }
        | crate::YamlError::ScalarTooLong { .. }
        | crate::YamlError::SequenceTooLong { .. }
        | crate::YamlError::MappingTooLarge { .. } => "limit_exceeded",
        crate::YamlError::UnknownField { .. } => "unknown_field",
        crate::YamlError::EmptySource => "empty_source",
        crate::YamlError::MissingField { .. } => "missing_field",
        crate::YamlError::FieldShape { .. } => "field_shape",
        crate::YamlError::ParseError { .. } => "parse_error",
        _ => "parse_error",
    }
}

pub(crate) fn validate_canonical_compile_scope(
    source: &crate::WorkflowSource,
) -> Result<(), CompileErrors> {
    let mut errors = Vec::new();
    let mut input_keys = HashSet::with_capacity(source.inputs().len());
    for input in source.inputs() {
        if !input_keys.insert(input.key.as_str()) {
            errors.push(CompileError::DuplicateInputName {
                name: Box::from(input.key.as_str()),
            });
        }
    }
    let mut secret_keys = HashSet::new();
    for secret in source.secrets() {
        if !secret_keys.insert(secret.key.as_str()) {
            errors.push(CompileError::DuplicateInputName {
                name: secret.key.clone().into(),
            });
        }
        if let Err(error) = crate::validate_public_name("secrets", &secret.key) {
            errors.push(error);
        }
        if secret.value.is_empty() {
            errors.push(CompileError::FieldShape {
                field: "secrets",
                expected: "non-empty host requirement names",
            });
        }
    }
    for example in source.examples() {
        if example.expected.is_none()
            || example
                .input
                .as_ref()
                .is_some_and(|input| !matches!(input, crate::AuthorValue::Mapping(_)))
        {
            errors.push(CompileError::FieldShape {
                field: "examples",
                expected: "an optional input mapping and a required expected value",
            });
        }
    }

    let mut step_ids = HashSet::with_capacity(source.steps().len());
    for (index, step) in source.steps().iter().enumerate() {
        if !step_ids.insert(step.id.as_str()) {
            errors.push(CompileError::DuplicateStepId {
                id: Box::from(step.id.as_str()),
            });
        }
        if step.retry.is_some() && !matches!(step.primitive, crate::StepPrimitive::Do { .. }) {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("try_again"),
            });
        }
    }
    let input_names: HashSet<&str> = source
        .inputs()
        .iter()
        .map(|input| input.key.as_str())
        .collect();
    // A top-level `set` output may not shadow a declared input. Nested
    // bodies are lexically scoped: branch-local shadowing is intended.
    for step in source.steps() {
        if let crate::StepPrimitive::Set { output, .. } = &step.primitive
            && input_names.contains(output.as_str())
        {
            errors.push(CompileError::SetOutputShadowsInput {
                name: Box::from(output.as_str()),
            });
        }
        super::part_10::check_step_tree(step, &mut errors);
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
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
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
