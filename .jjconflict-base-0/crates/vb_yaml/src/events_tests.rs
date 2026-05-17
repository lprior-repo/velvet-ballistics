#![forbid(unsafe_code)]
//! Events module tests.

use super::*;

fn assertion_failed(_message: std::fmt::Arguments<'_>) -> bool {
    false
}

macro_rules! fail_assert {
    ($($arg:tt)*) => {
        assert!(assertion_failed(format_args!($($arg)*)), $($arg)*)
    };
}

macro_rules! collect_ok {
    ($yaml:expr) => {
        match collect_events($yaml) {
            Ok(value) => value,
            Err(error) => {
                fail_assert!("event collection failed: {error}");
                return;
            }
        }
    };
}

#[test]
fn collect_events_empty_mapping() {
    let yaml = "key: value\n";
    let events = collect_ok!(yaml);
    assert!(events.len() >= 6);
}

#[test]
fn scalar_event_carries_value() {
    let yaml = "hello\n";
    let events = collect_ok!(yaml);
    let scalar = events.iter().find_map(|e| match e {
        YamlEvent::Scalar { value, .. } => Some(value.clone()),
        _ => None,
    });
    assert_eq!(scalar.as_deref(), Some("hello"));
}

#[test]
fn document_start_is_explicit() {
    let yaml = "---\nkey: value\n";
    let events = collect_ok!(yaml);
    let doc_start = events.iter().find(|e| e.is_document_start());
    assert!(matches!(
        doc_start,
        Some(YamlEvent::DocumentStart { explicit: true, .. })
    ));
}

#[test]
fn anchor_id_is_zero_by_default() {
    let yaml = "a: b\n";
    let events = collect_ok!(yaml);
    for event in &events {
        assert_eq!(event.anchor_id(), 0);
    }
}

#[test]
fn span_has_nonzero_line_for_content() {
    let yaml = "a: b\n";
    let events = collect_ok!(yaml);
    let scalar = events.iter().find(|e| e.as_scalar() == Some("a"));
    let Some(scalar) = scalar else {
        fail_assert!("missing scalar");
        return;
    };
    assert!(scalar.span().line > 0);
}

// -----------------------------------------------------------------------
// AST Node Behavior Tests (YamlEvent accessors)
// -----------------------------------------------------------------------

#[test]
fn typed_node_scalar_returns_value_via_as_scalar() {
    let yaml = "hello\n";
    let events = collect_ok!(yaml);
    let scalar_event = events.iter().find(|e| e.as_scalar().is_some());
    let Some(evt) = scalar_event else {
        fail_assert!("missing scalar event");
        return;
    };
    assert_eq!(evt.as_scalar(), Some("hello"));
}

#[test]
fn typed_node_scalar_returns_none_for_non_scalar() {
    let yaml = "a: 1\n";
    let events = collect_ok!(yaml);
    let stream_start = events
        .iter()
        .find(|e| matches!(e, YamlEvent::StreamStart { .. }));
    let Some(evt) = stream_start else {
        fail_assert!("missing stream start");
        return;
    };
    assert_eq!(evt.as_scalar(), None);
}

#[test]
fn typed_node_mapping_start_anchor_id_returns_zero() {
    let yaml = "a: 1\n";
    let events = collect_ok!(yaml);
    let mapping_start = events
        .iter()
        .find(|e| matches!(e, YamlEvent::MappingStart { .. }));
    let Some(evt) = mapping_start else {
        fail_assert!("missing mapping start");
        return;
    };
    assert_eq!(evt.anchor_id(), 0);
}

#[test]
fn typed_node_is_document_start_for_doc_start_event() {
    let yaml = "---\nkey: value\n";
    let events = collect_ok!(yaml);
    let doc_start = events.iter().find(|e| e.is_document_start());
    let Some(evt) = doc_start else {
        fail_assert!("missing document start");
        return;
    };
    assert!(evt.is_document_start());
}

#[test]
fn typed_node_is_document_start_for_non_doc_start() {
    let yaml = "hello\n";
    let events = collect_ok!(yaml);
    let scalar = events.iter().find(|e| e.as_scalar().is_some());
    let Some(evt) = scalar else {
        fail_assert!("missing scalar");
        return;
    };
    assert!(!evt.is_document_start());
}

#[test]
fn typed_node_is_alias_returns_true_for_alias() {
    let yaml = "a: &anc value\nb: *anc\n";
    let events = collect_ok!(yaml);
    let alias_event = events.iter().find(|e| e.is_alias());
    let Some(evt) = alias_event else {
        fail_assert!("missing alias event");
        return;
    };
    assert!(evt.is_alias());
}

#[test]
fn typed_node_is_alias_returns_false_for_scalar() {
    let yaml = "a: b\n";
    let events = collect_ok!(yaml);
    let scalar = events.iter().find(|e| e.as_scalar().is_some());
    let Some(evt) = scalar else {
        fail_assert!("missing scalar");
        return;
    };
    assert!(!evt.is_alias());
}

#[test]
fn typed_node_span_returns_correct_line_column() {
    let yaml = "a: b\n";
    let events = collect_ok!(yaml);
    let scalar = events.iter().find(|e| e.as_scalar() == Some("a"));
    let Some(evt) = scalar else {
        fail_assert!("missing scalar");
        return;
    };
    let span = evt.span();
    assert!(span.line > 0);
}

#[test]
fn typed_node_tag_returns_none_for_untagged() {
    let yaml = "a: b\n";
    let events = collect_ok!(yaml);
    let scalar = events.iter().find(|e| e.as_scalar().is_some());
    let Some(evt) = scalar else {
        fail_assert!("missing scalar");
        return;
    };
    assert_eq!(evt.tag(), None);
}

#[test]
fn typed_node_tag_returns_some_for_tagged() {
    let yaml = "a: !mytag b\n";
    let events = collect_ok!(yaml);
    let scalar = events.iter().find_map(|e| match e {
        YamlEvent::Scalar { tag: Some(_), .. } => Some(e.clone()),
        _ => None,
    });
    let Some(evt) = scalar else {
        fail_assert!("missing tagged scalar");
        return;
    };
    let tag = evt.tag();
    let Some(tag_str) = tag else {
        fail_assert!("expected Some tag");
        return;
    };
    assert!(
        tag_str.contains("mytag"),
        "tag should contain 'mytag', got: {tag_str}"
    );
}

#[test]
fn typed_node_anchor_id_returns_nonzero_for_anchored() {
    let yaml = "a: &anc value\n";
    let events = collect_ok!(yaml);
    let anchored = events.iter().find(|e| e.anchor_id() != 0);
    let Some(evt) = anchored else {
        fail_assert!("missing anchored event");
        return;
    };
    assert!(evt.anchor_id() > 0);
}

#[test]
fn typed_node_anchor_id_returns_zero_for_unanchored() {
    let yaml = "a: b\n";
    let events = collect_ok!(yaml);
    let all_zero = events.iter().all(|e| e.anchor_id() == 0);
    assert!(all_zero);
}

#[test]
fn event_span_fields_are_populated() {
    let yaml = "a: b\n";
    let events = collect_ok!(yaml);
    let stream_start = events
        .iter()
        .find(|e| matches!(e, YamlEvent::StreamStart { .. }));
    let Some(YamlEvent::StreamStart { span }) = stream_start else {
        fail_assert!("missing stream start");
        return;
    };
    assert!(span.end >= span.start);
}

#[test]
fn collect_events_produces_stream_lifecycle() {
    let yaml = "a: 1\n";
    let events = collect_ok!(yaml);
    let has_stream_start = events
        .iter()
        .any(|e| matches!(e, YamlEvent::StreamStart { .. }));
    let has_stream_end = events
        .iter()
        .any(|e| matches!(e, YamlEvent::StreamEnd { .. }));
    assert!(has_stream_start, "missing StreamStart");
    assert!(has_stream_end, "missing StreamEnd");
}

#[test]
fn document_start_carries_explicit_flag() {
    let yaml = "---\nkey: value\n";
    let events = collect_ok!(yaml);
    let doc_start = events.iter().find_map(|e| match e {
        YamlEvent::DocumentStart { explicit, .. } => Some(*explicit),
        _ => None,
    });
    assert_eq!(doc_start, Some(true));
}

#[test]
fn implicit_document_start_is_not_explicit() {
    let yaml = "key: value\n";
    let events = collect_ok!(yaml);
    let doc_start = events.iter().find_map(|e| match e {
        YamlEvent::DocumentStart { explicit, .. } => Some(*explicit),
        _ => None,
    });
    assert_eq!(doc_start, Some(false));
}

#[test]
fn sequence_events_have_start_and_end() {
    let yaml = "items:\n  - a\n  - b\n";
    let events = collect_ok!(yaml);
    let has_start = events
        .iter()
        .any(|e| matches!(e, YamlEvent::SequenceStart { .. }));
    let has_end = events
        .iter()
        .any(|e| matches!(e, YamlEvent::SequenceEnd { .. }));
    assert!(has_start, "missing SequenceStart");
    assert!(has_end, "missing SequenceEnd");
}

#[test]
fn mapping_events_have_start_and_end() {
    let yaml = "a: 1\n";
    let events = collect_ok!(yaml);
    let has_start = events
        .iter()
        .any(|e| matches!(e, YamlEvent::MappingStart { .. }));
    let has_end = events
        .iter()
        .any(|e| matches!(e, YamlEvent::MappingEnd { .. }));
    assert!(has_start, "missing MappingStart");
    assert!(has_end, "missing MappingEnd");
}

#[test]
fn scalar_style_plain_for_unquoted() {
    let yaml = "key: value\n";
    let events = collect_ok!(yaml);
    let scalar = events.iter().find_map(|e| match e {
        YamlEvent::Scalar { value, style, .. } if value.as_ref() == "value" => Some(*style),
        _ => None,
    });
    assert_eq!(scalar, Some(ScalarStyle::Plain));
}

#[test]
fn scalar_style_single_quoted() {
    let yaml = "key: 'value'\n";
    let events = collect_ok!(yaml);
    let scalar = events.iter().find_map(|e| match e {
        YamlEvent::Scalar { value, style, .. } if value.as_ref() == "value" => Some(*style),
        _ => None,
    });
    assert_eq!(scalar, Some(ScalarStyle::SingleQuoted));
}

#[test]
fn scalar_style_double_quoted() {
    let yaml = "key: \"value\"\n";
    let events = collect_ok!(yaml);
    let scalar = events.iter().find_map(|e| match e {
        YamlEvent::Scalar { value, style, .. } if value.as_ref() == "value" => Some(*style),
        _ => None,
    });
    assert_eq!(scalar, Some(ScalarStyle::DoubleQuoted));
}

#[test]
fn event_span_from_parser_span_fields() {
    let span = EventSpan {
        start: 0,
        end: 10,
        line: 1,
        column: 1,
    };
    assert_eq!(span.start, 0);
    assert_eq!(span.end, 10);
    assert_eq!(span.line, 1);
    assert_eq!(span.column, 1);
}

#[test]
fn scalar_event_has_zero_anchor_when_unanchored() {
    let yaml = "key: value\n";
    let events = collect_ok!(yaml);
    let scalar = events.iter().find_map(|e| match e {
        YamlEvent::Scalar {
            value, anchor_id, ..
        } if value.as_ref() == "value" => Some(*anchor_id),
        _ => None,
    });
    assert_eq!(scalar, Some(0));
}

#[test]
fn sequence_start_tag_returns_none_when_untagged() {
    let yaml = "items:\n  - a\n";
    let events = collect_ok!(yaml);
    let seq_start = events.iter().find_map(|e| match e {
        YamlEvent::SequenceStart { tag, .. } => Some(tag.clone()),
        _ => None,
    });
    let Some(tag) = seq_start else {
        fail_assert!("missing sequence start");
        return;
    };
    assert_eq!(tag, None);
}

// -----------------------------------------------------------------------
// Adversarial BDD tests - event layer attack vectors
// -----------------------------------------------------------------------

#[test]
fn adversarial_events_null_byte_accepted_by_parser_but_rejected_by_profile() {
    let yaml = "key: val\x00ue\n";
    let result = collect_events(yaml);
    match result {
        Ok(events) => {
            let _scalar = events.iter().find_map(|e| match e {
                YamlEvent::Scalar { value, .. } if value.contains('\x00') => Some(value.clone()),
                _ => None,
            });
            assert!(!events.is_empty(), "events should not be empty");
            let profile_result = crate::profile::validate_yaml_profile(yaml);
            assert!(
                matches!(
                    profile_result,
                    Err(crate::YamlError::ForbiddenFeature {
                        detail: "null_byte_in_source"
                    })
                ),
                "profile validation must reject null bytes, got: {profile_result:?}"
            );
        }
        Err(e) => {
            assert!(!e.to_string().is_empty());
        }
    }
}

#[test]
fn adversarial_events_unicode_zero_width_char_accepted_as_events() {
    let yaml = "key: hello\u{200D}world\n";
    let result = collect_events(yaml);
    match result {
        Ok(events) => {
            let scalar = events.iter().find_map(|e| match e {
                YamlEvent::Scalar { value, .. } if value.contains('\u{200D}') => {
                    Some(value.clone())
                }
                _ => None,
            });
            assert!(scalar.is_some(), "expected scalar with zero-width joiner");
        }
        Err(e) => fail_assert!("expected Ok events, got Err: {e}"),
    }
}

#[test]
fn adversarial_events_rtl_override_in_scalar_parsed() {
    let yaml = "key: hello\u{202E}world\n";
    let result = collect_events(yaml);
    match result {
        Ok(events) => {
            let scalar = events.iter().find_map(|e| match e {
                YamlEvent::Scalar { value, .. } if value.contains('\u{202E}') => {
                    Some(value.clone())
                }
                _ => None,
            });
            assert!(scalar.is_some(), "expected scalar with RTL override");
        }
        Err(e) => fail_assert!("expected Ok events, got Err: {e}"),
    }
}

#[test]
fn adversarial_events_tagged_scalar_carries_tag() {
    let yaml = "key: !mytag value\n";
    let events = collect_ok!(yaml);
    let tagged = events.iter().find_map(|e| match e {
        YamlEvent::Scalar { tag: Some(t), .. } => Some(t.clone()),
        _ => None,
    });
    let Some(tag) = tagged else {
        fail_assert!("missing tagged scalar event");
        return;
    };
    assert!(tag.contains("mytag"), "expected 'mytag' in tag, got: {tag}");
}

#[test]
fn adversarial_events_anchor_produces_nonzero_anchor_id() {
    let yaml = "a: &anc value\n";
    let events = collect_ok!(yaml);
    let anchored = events.iter().find(|e| e.anchor_id() != 0);
    assert!(anchored.is_some(), "expected anchored event");
}

#[test]
fn adversarial_events_alias_produces_alias_variant() {
    let yaml = "a: &anc value\nb: *anc\n";
    let events = collect_ok!(yaml);
    let alias = events.iter().find(|e| e.is_alias());
    assert!(alias.is_some(), "expected Alias event in stream");
}

#[test]
fn adversarial_events_multi_doc_produces_multiple_document_starts() {
    let yaml = "---\na: 1\n---\nb: 2\n";
    let events = collect_ok!(yaml);
    let doc_starts: Vec<_> = events.iter().filter(|e| e.is_document_start()).collect();
    assert_eq!(doc_starts.len(), 2, "expected 2 document starts");
}

#[test]
fn adversarial_events_block_scalar_preserves_value() {
    let yaml = "key: |\n  line1\n  line2\n";
    let events = collect_ok!(yaml);
    let scalar = events.iter().find_map(|e| match e {
        YamlEvent::Scalar {
            value,
            style: ScalarStyle::Literal,
            ..
        } => Some(value.clone()),
        _ => None,
    });
    let Some(val) = scalar else {
        fail_assert!("missing literal block scalar");
        return;
    };
    assert!(val.contains("line1"), "expected 'line1' in block scalar");
    assert!(val.contains("line2"), "expected 'line2' in block scalar");
}

#[test]
fn adversarial_events_folded_scalar_preserves_value() {
    let yaml = "key: >\n  line1\n  line2\n";
    let events = collect_ok!(yaml);
    let scalar = events.iter().find_map(|e| match e {
        YamlEvent::Scalar {
            style: ScalarStyle::Folded,
            ..
        } => Some(true),
        _ => None,
    });
    assert_eq!(scalar, Some(true), "expected Folded scalar");
}
