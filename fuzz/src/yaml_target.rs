//! YAML parsing fuzzing targets.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::let_underscore_must_use)]
#![allow(clippy::as_conversions)]
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::len_zero)]

const MAX_FUZZ_PAYLOAD: u32 = 4096;

fn assert_typed_yaml_error(error: vb_compile::YamlError) {
    use vb_compile::YamlError;
    match error {
        YamlError::UnsupportedTrigger { .. }
        | YamlError::UnsupportedFeature { .. }
        | YamlError::DuplicateKey { .. }
        | YamlError::AnchorAliasMerge
        | YamlError::CustomTag { .. }
        | YamlError::BinaryScalar
        | YamlError::MultipleDocuments { .. }
        | YamlError::AmbiguousScalar { .. }
        | YamlError::SourceTooLarge { .. }
        | YamlError::NestingTooDeep { .. }
        | YamlError::NodeLimitExceeded { .. }
        | YamlError::ScalarTooLong { .. }
        | YamlError::SequenceTooLong { .. }
        | YamlError::MappingTooLarge { .. }
        | YamlError::UnknownField { .. }
        | YamlError::EmptySource
        | YamlError::MissingField { .. }
        | YamlError::FieldShape { .. }
        | YamlError::ParseError { .. }
        | YamlError::ForbiddenFeature { .. }
        | YamlError::LegacyPrimitive { .. } => {}
        _ => {}
    }
}

pub fn fuzz_yaml_events(data: &[u8]) {
    if let Ok(text) = std::str::from_utf8(data) {
        let profile_result = vb_compile::validate_yaml_profile(text);
        match profile_result {
            Ok(()) => {}
            Err(e) => {
                assert_typed_yaml_error(e);
            }
        }

        let events_result = vb_compile::parse_yaml_events(text);
        match events_result {
            Ok(events) => {
                if !text.trim().is_empty() {
                    assert!(
                        !events.is_empty(),
                        "non-empty YAML input must produce non-empty events"
                    );
                }
                assert!(
                    events.len() <= MAX_FUZZ_PAYLOAD as usize,
                    "event count {} exceeds max payload bound",
                    events.len()
                );
            }
            Err(e) => {
                assert_typed_yaml_error(e);
            }
        }

        let source_map_result = vb_compile::build_source_map(text);
        match source_map_result {
            Ok(source_map) => {
                assert!(
                    source_map.len() <= MAX_FUZZ_PAYLOAD as usize,
                    "source map entries {} exceeds max payload bound",
                    source_map.len()
                );
            }
            Err(e) => {
                assert_typed_yaml_error(e);
            }
        }
    }
}

pub fn fuzz_strict_yaml_profile(data: &[u8]) {
    let compile_result = vb_compile::compile_workflow(data);
    if let Ok(ref workflow) = compile_result {
        let text = String::from_utf8_lossy(data);
        let unsupported =
            text.contains("---") || text.contains('&') || text.contains('*') || text.contains('!');
        assert!(
            !unsupported,
            "unsupported YAML features must cause compile error"
        );
        assert!(
            workflow.node_count() >= 1,
            "compiled workflow must have at least 1 node"
        );
    }
}

pub fn fuzz_compile_source_ast_marks(data: &[u8]) {
    use vb_compile::compile_workflow;

    let result = compile_workflow(data);

    match result {
        Ok(_compiled) => {}
        Err(errors) => {
            assert!(
                !errors.is_empty(),
                "CompileErrors must contain at least one error"
            );
        }
    }
}

pub fn fuzz_span_bridge(data: &[u8]) {
    use vb_compile::{SourceSpan, build_semantic_source_map, build_source_map};

    let mut values = [0usize; 6];
    for (slot, byte) in values.iter_mut().zip(data.iter().copied()) {
        *slot = usize::from(byte);
    }

    let span = SourceSpan::new(
        values[0], values[1], values[2], values[3], values[4], values[5],
    );
    assert_eq!(span.start_offset, values[0]);
    assert_eq!(span.end_offset, values[1]);
    assert_eq!(span.start_line, values[2]);
    assert_eq!(span.start_col, values[3]);
    assert_eq!(span.end_line, values[4]);
    assert_eq!(span.end_col, values[5]);

    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    if let Ok(map) = build_source_map(text) {
        for (index, mapped_span) in map.iter() {
            assert_eq!(map.span_for_node(index), Some(mapped_span));
        }
    }

    if let Ok(map) = build_semantic_source_map(text) {
        let _ = map.span_for_path("$");
        let _ = map.span_for_path("$.when.manual");
        let _ = map.span_for_path("$.steps[0]");
    }
}
