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
    assert_eq!(events.len(), 8);
    assert_eq!(events[3].as_scalar(), Some("key"));
    assert_eq!(events[4].as_scalar(), Some("value"));
}

#[test]
fn collect_events_simple_mapping_has_exact_event_sequence_and_scalar_fields() {
    let yaml = "key: value\n";
    let events = collect_ok!(yaml);
    assert_eq!(events.len(), 8);

    assert_eq!(events[0], YamlEvent::StreamStart { span: events[0].span() });
    assert_eq!(
        events[1],
        YamlEvent::DocumentStart {
            explicit: false,
            span: events[1].span()
        }
    );
    assert_eq!(
        events[2],
        YamlEvent::MappingStart {
            anchor_id: 0,
            tag: None,
            span: events[2].span()
        }
    );
    assert_eq!(
        events[3],
        YamlEvent::Scalar {
            value: "key".into(),
            style: ScalarStyle::Plain,
            anchor_id: 0,
            tag: None,
            span: EventSpan {
                start: 0,
                end: 3,
                line: 1,
                column: 0,
            }
        }
    );
    assert_eq!(
        events[4],
        YamlEvent::Scalar {
            value: "value".into(),
            style: ScalarStyle::Plain,
            anchor_id: 0,
            tag: None,
            span: EventSpan {
                start: 5,
                end: 10,
                line: 1,
                column: 5,
            }
        }
    );
    assert_eq!(events[5], YamlEvent::MappingEnd { span: events[5].span() });
    assert_eq!(events[6], YamlEvent::DocumentEnd { span: events[6].span() });
    assert_eq!(events[7], YamlEvent::StreamEnd { span: events[7].span() });
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
    assert_eq!(
        doc_start,
        Some(&YamlEvent::DocumentStart {
            explicit: true,
            span: doc_start.map(YamlEvent::span).unwrap_or(EventSpan {
                start: 0,
                end: 0,
                line: 0,
                column: 0,
            }),
        })
    );
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
    assert_eq!(scalar.span(), EventSpan { start: 0, end: 1, line: 1, column: 0 });
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
    assert_eq!(evt, &YamlEvent::DocumentStart { explicit: true, span: evt.span() });
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
    assert_eq!(span, EventSpan { start: 0, end: 1, line: 1, column: 0 });
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
    assert_eq!(tag_str, "!mytag");
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
    assert_eq!(evt.anchor_id(), 1);
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
    assert_eq!(*span, EventSpan { start: 0, end: 0, line: 1, column: 0 });
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
            let key_scalar = events.iter().find_map(|e| match e {
                YamlEvent::Scalar { value, .. } if value.as_ref() == "key" => Some(value.clone()),
                _ => None,
            });
            assert_eq!(events.len(), 8);
            assert_eq!(key_scalar.as_deref(), Some("key"));
            assert_eq!(events.iter().filter(|e| e.as_scalar().is_some()).count(), 2);
            let profile_result = crate::profile::validate_yaml_profile(yaml);
            assert_eq!(profile_result, Err(crate::YamlError::ForbiddenFeature { detail: "null_byte_in_source" }));
        }
        Err(e) => {
            assert_eq!(e, crate::YamlError::ParseError { line: 1, reason: "control characters are not allowed".into() });
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
            assert_eq!(scalar.as_deref(), Some("hello\u{200D}world"));
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
            assert_eq!(scalar.as_deref(), Some("hello\u{202E}world"));
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
    assert_eq!(tag.as_ref(), "!mytag");
}

#[test]
fn adversarial_events_anchor_produces_nonzero_anchor_id() {
    let yaml = "a: &anc value\n";
    let events = collect_ok!(yaml);
    let anchored = events.iter().find(|e| e.anchor_id() != 0);
    assert_eq!(anchored.map(YamlEvent::anchor_id), Some(1));
}

#[test]
fn adversarial_events_alias_produces_alias_variant() {
    let yaml = "a: &anc value\nb: *anc\n";
    let events = collect_ok!(yaml);
    let alias = events.iter().find(|e| e.is_alias());
    assert_eq!(alias, Some(&YamlEvent::Alias { anchor_id: 1, span: alias.map(YamlEvent::span).unwrap_or(EventSpan { start: 0, end: 0, line: 0, column: 0 }) }));
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
    assert_eq!(val.as_ref(), "line1\nline2\n");
}

#[test]
fn adversarial_events_folded_scalar_preserves_value() {
    let yaml = "key: >\n  line1\n  line2\n";
    let events = collect_ok!(yaml);
    let scalar = events.iter().find_map(|e| match e {
        YamlEvent::Scalar {
            value,
            style: ScalarStyle::Folded,
            ..
        } => Some(value.clone()),
        _ => None,
    });
    assert_eq!(scalar.as_deref(), Some("line1 line2\n"));
}
