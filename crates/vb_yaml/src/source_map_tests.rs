#![forbid(unsafe_code)]
//! Source map module tests.

use super::*;

fn assertion_failed(_message: std::fmt::Arguments<'_>) -> bool {
    false
}

macro_rules! fail_assert {
    ($($arg:tt)*) => {
        assert!(assertion_failed(format_args!($($arg)*)), $($arg)*)
    };
}

macro_rules! build_ok {
    ($yaml:expr) => {
        match build_source_map($yaml) {
            Ok(value) => value,
            Err(error) => {
                fail_assert!("source map failed: {error}");
                return;
            }
        }
    };
}

#[test]
fn empty_source_map() {
    let map = SourceMap::new();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
    assert_eq!(map.span_for_node(0), None);
}

#[test]
fn build_from_simple_yaml() {
    let yaml = "key: value\n";
    let map = build_ok!(yaml);
    assert_eq!(map.len(), 3);
    assert_eq!(span_text(yaml, map.span_for_node(1).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "key");
    assert_eq!(span_text(yaml, map.span_for_node(2).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "value");
}

#[test]
fn node_indices_are_sequential() {
    let yaml = "a: 1\nb: 2\n";
    let map = build_ok!(yaml);
    let count = map.len();
    assert_eq!(count, 5);

    let mut found = Vec::new();
    for (idx, _span) in map.iter() {
        found.push(idx);
    }
    for (i, idx) in found.iter().enumerate() {
        let Ok(expected) = u32::try_from(i) else {
            fail_assert!("index does not fit u32");
            return;
        };
        assert_eq!(*idx, expected);
    }
}

#[test]
fn default_is_empty() {
    let map = SourceMap::default();
    assert!(map.is_empty());
}

// -----------------------------------------------------------------------
// Source Map BDD tests
// -----------------------------------------------------------------------

#[test]
fn source_map_new_is_empty() {
    let map = SourceMap::new();
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());
}

#[test]
fn source_map_len_increases_with_entries() {
    let yaml = "a: 1\nb: 2\n";
    let map = build_ok!(yaml);
    let count = map.len();
    assert_eq!(count, 5);
}

#[test]
fn source_map_iter_yields_inserted_entries() {
    let yaml = "a: 1\n";
    let map = build_ok!(yaml);
    let entries: Vec<(u32, SourceSpan)> = map.iter().collect();
    assert_eq!(entries.len(), 3);
    let Some(first) = entries.first() else {
        fail_assert!("missing first source-map entry");
        return;
    };
    assert_eq!(first.0, 0);
    assert_eq!(first.1, SourceSpan::new(0, 0, 1, 0, 1, 1));
}

#[test]
fn build_source_map_produces_correct_mappings() {
    let yaml = "key: value\n";
    let map = build_ok!(yaml);
    let span = map.span_for_node(0);
    let Some(s) = span else {
        fail_assert!("expected Some span for node 0");
        return;
    };
    assert_eq!(s, SourceSpan::new(0, 0, 1, 0, 1, 1));
    assert_eq!(span_text(yaml, map.span_for_node(1).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "key");
    assert_eq!(span_text(yaml, map.span_for_node(2).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "value");
}

#[test]
fn source_map_span_for_node_returns_correct_range() {
    let yaml = "a: 1\n";
    let map = build_ok!(yaml);
    let span = map.span_for_node(0);
    let Some(s) = span else {
        fail_assert!("expected Some span");
        return;
    };
    assert_eq!(s, SourceSpan::new(0, 0, 1, 0, 1, 1));
    assert_eq!(span_text(yaml, map.span_for_node(1).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "a");
    assert_eq!(span_text(yaml, map.span_for_node(2).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "1");
}

fn span_text<'a>(yaml: &'a str, span: SourceSpan) -> &'a str {
    yaml.get(span.start_offset..span.end_offset).unwrap_or("")
}

#[test]
fn source_map_span_for_node_returns_none_for_out_of_range() {
    let yaml = "a: 1\n";
    let map = build_ok!(yaml);
    let result = map.span_for_node(9999);
    assert_eq!(result, None);
}

#[test]
fn source_map_iter_indices_are_sequential() {
    let yaml = "a: 1\nb: 2\n";
    let map = build_ok!(yaml);
    let indices: Vec<u32> = map.iter().map(|(i, _)| i).collect();
    let mut expected: u32 = 0;
    for idx in &indices {
        assert_eq!(*idx, expected);
        expected = expected.saturating_add(1);
    }
}

#[test]
fn source_span_new_exact_values() {
    let span = SourceSpan::new(0, 4, 1, 2, 3, 4);
    assert_eq!(span.start_line, 1);
    assert_eq!(span.start_col, 2);
    assert_eq!(span.end_line, 3);
    assert_eq!(span.end_col, 4);
}

#[test]
fn source_map_preserves_order_from_yaml() {
    let yaml = "first: a\nsecond: b\nthird: c\n";
    let map = build_ok!(yaml);
    let entries: Vec<(u32, SourceSpan)> = map.iter().collect();
    assert_eq!(entries.len(), 7);
    assert_eq!(span_text(yaml, entries[1].1), "first");
    assert_eq!(span_text(yaml, entries[2].1), "a");
    assert_eq!(span_text(yaml, entries[3].1), "second");
    assert_eq!(span_text(yaml, entries[4].1), "b");
    assert_eq!(span_text(yaml, entries[5].1), "third");
    assert_eq!(span_text(yaml, entries[6].1), "c");
}

#[test]
fn build_source_map_for_nested_yaml() {
    let yaml = "a:\n  b: 1\n";
    let map = build_ok!(yaml);
    assert_eq!(map.len(), 5);
    assert_eq!(span_text(yaml, map.span_for_node(1).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "a");
    assert_eq!(span_text(yaml, map.span_for_node(3).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "b");
    assert_eq!(span_text(yaml, map.span_for_node(4).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "1");
}

#[test]
fn build_source_map_for_sequence_yaml() {
    let yaml = "items:\n  - a\n  - b\n";
    let map = build_ok!(yaml);
    assert_eq!(map.len(), 5);
    assert_eq!(span_text(yaml, map.span_for_node(1).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "items");
    assert_eq!(span_text(yaml, map.span_for_node(3).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "a");
    assert_eq!(span_text(yaml, map.span_for_node(4).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "b");
}

// -----------------------------------------------------------------------
// Adversarial BDD tests - source map edge cases
// -----------------------------------------------------------------------

#[test]
fn adversarial_source_map_empty_input_is_rejected_by_profile() {
    let result = build_source_map("");
    assert_eq!(result, Err(crate::YamlError::EmptySource));
}

#[test]
fn adversarial_source_map_malformed_yaml_returns_error() {
    let yaml = "a: [1, 2\n";
    let result = build_source_map(yaml);
    assert_eq!(
        result,
        Err(crate::YamlError::ParseError {
            line: 2,
            reason: "while parsing a flow sequence, expected ',' or ']'".into()
        })
    );
}

#[test]
fn adversarial_source_map_multi_line_scalar_tracks_spans() {
    let yaml = "key: |\n  line1\n  line2\n  line3\n";
    let result = build_source_map(yaml);
    match result {
        Ok(map) => {
            assert_eq!(map.len(), 3);
            assert_eq!(span_text(yaml, map.span_for_node(1).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "key");
            assert_eq!(
                span_text(yaml, map.span_for_node(2).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))),
                "line1\n  line2\n  line3\n"
            );
        }
        Err(e) => fail_assert!("expected Ok source map, got Err: {e}"),
    }
}

#[test]
fn semantic_source_map_tracks_multi_step_paths_distinctly() {
    let yaml = "version: velvet-ballastics/v1\nname: paths\nwhen:\n  event:\n    name: invoice.created\nsteps:\n  - id: first\n    set:\n      output: result\n      value: one\n  - id: second\n    finish:\n      result: result\n";
    let map = match build_semantic_source_map(yaml) {
        Ok(map) => map,
        Err(error) => {
            fail_assert!("semantic source map should build, got {error:?}");
            return;
        }
    };

    let trigger = map.span_for_path("$.when.event");
    let first_id = map.span_for_path("$.steps[0].id");
    let second_id = map.span_for_path("$.steps[1].id");
    let result = map.span_for_path("$.steps[1].finish.result");

    assert_eq!(trigger.map(|span| span_text(yaml, span)), Some("event"));
    assert_eq!(first_id.map(|span| span_text(yaml, span)), Some("first"));
    assert_eq!(second_id.map(|span| span_text(yaml, span)), Some("second"));
    assert_eq!(result.map(|span| span_text(yaml, span)), Some("result"));
    assert_ne!(first_id, second_id);
}

#[test]
fn semantic_source_map_repeated_fields_use_event_positions_not_text_find() {
    let yaml = "version: velvet-ballastics/v1\nname: repeated\nwhen:\n  webhook: {}\nsteps:\n  - id: repeated\n    set:\n      output: first\n      value: repeated\n  - id: repeated_later\n    finish:\n      result: repeated\nnext_key: after\n";
    let map = match build_semantic_source_map(yaml) {
        Ok(map) => map,
        Err(error) => {
            fail_assert!("semantic source map should build, got {error:?}");
            return;
        }
    };

    let second_id = map.span_for_path("$.steps[1].id");
    let result = map.span_for_path("$.steps[1].finish.result");

    assert_eq!(second_id.map(|span| span_text(yaml, span)), Some("repeated_later"));
    assert_eq!(result.map(|span| span_text(yaml, span)), Some("repeated"));
    assert_eq!(result.map(|span| span.end_offset), yaml.find("\nnext_key"));
}

#[test]
fn adversarial_source_map_null_byte_rejected_by_profile() {
    let yaml = "key: \x00value\n";
    let result = build_source_map(yaml);
    assert_eq!(
        result,
        Err(crate::YamlError::ForbiddenFeature {
            detail: "null_byte_in_source"
        })
    );
}

#[test]
fn adversarial_source_map_anchor_rejected_by_profile() {
    let yaml = "a: &anchor value\nb: *anchor\n";
    let result = build_source_map(yaml);
    assert_eq!(result, Err(crate::YamlError::AnchorAliasMerge));
}

#[test]
fn adversarial_source_map_deeply_nested_yaml_tracked() {
    let yaml = "a:\n  b:\n    c:\n      d:\n        e: 1\n";
    let result = build_source_map(yaml);
    match result {
        Ok(map) => {
            assert_eq!(map.len(), 11);
            assert_eq!(span_text(yaml, map.span_for_node(9).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "e");
            assert_eq!(span_text(yaml, map.span_for_node(10).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "1");
        }
        Err(e) => fail_assert!("expected Ok, got Err: {e}"),
    }
}

#[test]
fn adversarial_source_map_unicode_keys_tracked() {
    let yaml = "\u{00E9}clat: 1\n\u{00FC}ber: 2\n";
    let result = build_source_map(yaml);
    match result {
        Ok(map) => {
            assert_eq!(map.len(), 5);
            assert_eq!(span_text(yaml, map.span_for_node(1).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "écla");
            assert_eq!(span_text(yaml, map.span_for_node(3).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "\nüb");
        }
        Err(e) => fail_assert!("expected Ok, got Err: {e}"),
    }
}

#[test]
fn adversarial_source_map_large_input_tracked() {
    let mut yaml = String::new();
    for i in 0..100 {
        yaml.push_str(&format!("key{i}: val{i}\n"));
    }
    let result = build_source_map(&yaml);
    match result {
        Ok(map) => {
            assert_eq!(map.len(), 201);
            assert_eq!(span_text(&yaml, map.span_for_node(1).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "key0");
            assert_eq!(span_text(&yaml, map.span_for_node(200).unwrap_or(SourceSpan::new(0, 0, 0, 0, 0, 0))), "val99");
        }
        Err(e) => fail_assert!("expected Ok, got Err: {e}"),
    }
}
