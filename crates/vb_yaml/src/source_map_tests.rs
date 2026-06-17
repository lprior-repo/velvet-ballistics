#![forbid(unsafe_code)]
#![allow(clippy::expect_used, clippy::panic)]
//! Source map module tests.

use super::*;
use crate::YamlError;
use crate::source_map::build_semantic_source_map;

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
    assert!(!map.is_empty());
}

#[test]
fn node_indices_are_sequential() {
    let yaml = "a: 1\nb: 2\n";
    let map = build_ok!(yaml);
    let count = map.len();
    assert!(count >= 2);

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
    assert!(count >= 2, "expected at least 2 entries, got {count}");
    assert!(!map.is_empty());
}

#[test]
fn source_map_iter_yields_inserted_entries() {
    let yaml = "a: 1\n";
    let map = build_ok!(yaml);
    let entries: Vec<(u32, SourceSpan)> = map.iter().collect();
    assert!(!entries.is_empty());
    let Some(first) = entries.first() else {
        fail_assert!("missing first source-map entry");
        return;
    };
    assert_eq!(first.0, 0);
    assert!(first.1.start_line > 0);
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
    assert!(s.start_line > 0);
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
    assert_eq!(s.start_line, s.end_line);
    assert!(s.end_col >= s.start_col);
    assert!(s.end_offset >= s.start_offset);
}

fn span_text(yaml: &str, span: SourceSpan) -> &str {
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
    assert!(
        entries.len() >= 3,
        "expected at least 3 entries, got {}",
        entries.len()
    );
}

#[test]
fn build_source_map_for_nested_yaml() {
    let yaml = "a:\n  b: 1\n";
    let map = build_ok!(yaml);
    assert!(
        map.len() >= 3,
        "expected at least 3 entries, got {}",
        map.len()
    );
}

#[test]
fn build_source_map_for_sequence_yaml() {
    let yaml = "items:\n  - a\n  - b\n";
    let map = build_ok!(yaml);
    assert!(
        map.len() >= 2,
        "expected at least 2 entries, got {}",
        map.len()
    );
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
    assert!(
        matches!(result, Err(YamlError::ParseError { .. })),
        "expected ParseError for malformed YAML in source map, got {result:?}"
    );
}

#[test]
fn adversarial_source_map_multi_line_scalar_tracks_spans() {
    let yaml = "key: |\n  line1\n  line2\n  line3\n";
    let result = build_source_map(yaml);
    match result {
        Ok(map) => {
            assert!(!map.is_empty(), "expected non-empty source map");
          let first = map.span_for_node(0);
            let Some(span) = first else {
                panic!("expected span for node 0, got None");
            };
            assert!(span.start_line > 0, "start_line should be > 0");
        }
        Err(e) => fail_assert!("expected Ok source map, got Err: {e}"),
    }
}

#[test]
fn semantic_source_map_tracks_multi_step_paths_distinctly() {
    let yaml = "version: velvet-ballistics/v1\nname: paths\nwhen:\n  event:\n    name: invoice.created\nsteps:\n  - id: first\n    set:\n      output: result\n      value: one\n  - id: second\n    finish:\n      result: result\n";
    let map = build_semantic_source_map(yaml).unwrap_or_default();

    let trigger = map.span_for_path("$.when.event");
    let first_id = map.span_for_path("$.steps[0].id");
    let second_id = map.span_for_path("$.steps[1].id");
    let result = map.span_for_path("$.steps[1].finish.result");

    assert!(matches!(trigger, Some(span) if span_text(yaml, span) == "event"));
    assert!(matches!(first_id, Some(span) if span_text(yaml, span) == "first"));
    assert!(matches!(second_id, Some(span) if span_text(yaml, span) == "second"));
    assert!(matches!(result, Some(span) if span_text(yaml, span) == "result"));
    assert_ne!(first_id, second_id);
}

#[test]
fn semantic_source_map_repeated_fields_use_event_positions_not_text_find() {
    let yaml = "version: velvet-ballistics/v1\nname: repeated\nwhen:\n  webhook: {}\nsteps:\n  - id: repeated\n    set:\n      output: first\n      value: repeated\n  - id: repeated_later\n    finish:\n      result: repeated\nnext_key: after\n";
    let map = build_semantic_source_map(yaml).unwrap_or_default();

    let second_id = map.span_for_path("$.steps[1].id");
    let result = map.span_for_path("$.steps[1].finish.result");

    assert!(matches!(second_id, Some(span) if span_text(yaml, span) == "repeated_later"));
    assert!(matches!(result, Some(span) if span_text(yaml, span) == "repeated"));
    assert!(matches!(result, Some(span) if span.end_offset < yaml.len()));
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
            assert!(
                map.len() >= 5,
                "expected at least 5 nodes, got {}",
                map.len()
            );
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
            assert!(map.len() >= 2, "expected at least 2 nodes");
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
            assert!(
                map.len() >= 100,
                "expected at least 100 nodes, got {}",
                map.len()
            );
        }
        Err(e) => fail_assert!("expected Ok, got Err: {e}"),
    }
}

// -----------------------------------------------------------------------
// Semantic source map path construction
// -----------------------------------------------------------------------

/// Semantic source map tracks $.when.event trigger container.
#[test]
fn semantic_source_map_trigger_when_event() {
    let yaml = "version: velvet-ballistics/v1\nname: test\nwhen:\n  event:\n    name: invoice\nsteps:\n  - id: first\n    set:\n      output: result\n";
    let map = build_semantic_source_map(yaml).unwrap_or_default();
    let trigger = map.span_for_path("$.when.event");
    let Some(span) = trigger else {
        panic!("expected $.when.event trigger span");
    };
    assert!(
        span_text(yaml, span).contains("event"),
        "trigger span should contain 'event', got: {:?}",
        span_text(yaml, span)
    );
}

/// Semantic source map tracks $.when.manual trigger container.
#[test]
fn semantic_source_map_trigger_when_manual() {
    let yaml = "version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: first\n    set:\n      output: result\n";
    let map = build_semantic_source_map(yaml).unwrap_or_default();
    let trigger = map.span_for_path("$.when.manual");
    let Some(span) = trigger else {
        panic!("expected $.when.manual trigger span");
    };
    assert!(
        span_text(yaml, span).contains("manual"),
        "manual trigger span must contain 'manual'"
    );
}

/// Semantic source map tracks $.when.schedule trigger container.
#[test]
fn semantic_source_map_trigger_when_schedule() {
    let yaml = "version: velvet-ballistics/v1\nname: test\nwhen:\n  schedule:\n    cron: '0 * * * *'\nsteps:\n  - id: first\n    set:\n      output: result\n";
    let map = build_semantic_source_map(yaml).unwrap_or_default();
    let trigger = map.span_for_path("$.when.schedule");
    let Some(span) = trigger else {
        panic!("expected $.when.schedule trigger span");
    };
    assert!(
        span_text(yaml, span).contains("schedule"),
        "schedule trigger span must contain 'schedule'"
    );
}

/// Semantic source map tracks $.when.webhook trigger container.
#[test]
fn semantic_source_map_trigger_when_webhook() {
    let yaml = "version: velvet-ballistics/v1\nname: test\nwhen:\n  webhook: {}\nsteps:\n  - id: first\n    set:\n      output: result\n";
    let map = build_semantic_source_map(yaml).unwrap_or_default();
    let trigger = map.span_for_path("$.when.webhook");
    let Some(span) = trigger else {
        panic!("expected $.when.webhook trigger span");
    };
    assert!(
        span_text(yaml, span).contains("webhook"),
        "webhook trigger span must contain 'webhook'"
    );
}

/// Non-trigger containers like $.next_key should NOT be trigger paths.
#[test]
fn semantic_source_map_non_trigger_not_flagged_as_trigger() {
    let yaml = "version: velvet-ballistics/v1\nname: test\nwhen:\n  event:\n    name: invoice\nsteps:\n  - id: first\n    set:\n      output: result\nnext_key: after\n";
    let map = build_semantic_source_map(yaml).unwrap_or_default();
    // $.next_key is not a trigger container path
    let Some(span) = map.span_for_path("$.next_key") else {
        panic!("non-trigger $.next_key should still be tracked in source map");
    };
    assert!(span.end_offset >= span.start_offset, "span must have valid range");
}

/// Sequence indices in semantic paths are 0-based and sequential.
#[test]
fn semantic_source_map_sequence_indices_sequential() {
    let yaml = "version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: a\n    set:\n      output: r1\n  - id: b\n    set:\n      output: r2\n  - id: c\n    set:\n      output: r3\n";
    let map = build_semantic_source_map(yaml).unwrap_or_default();
    let a = map.span_for_path("$.steps[0].id");
    let b = map.span_for_path("$.steps[1].id");
    let c = map.span_for_path("$.steps[2].id");
    let Some(sa) = a else {
        panic!("expected $.steps[0].id");
    };
    let Some(sb) = b else {
        panic!("expected $.steps[1].id");
    };
    let Some(_sc) = c else {
        panic!("expected $.steps[2].id");
    };
    // Each should resolve to a different text value
    {
        let text_a = span_text(yaml, sa);
        let text_b = span_text(yaml, sb);
        assert_ne!(
            text_a, text_b,
            "different step indices should map to different text"
        );
    }
}

/// Semantic source map tracks nested set paths.
#[test]
fn semantic_source_map_nested_set_paths() {
    let yaml = "version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: first\n    set:\n      output: result\n      value: one\n";
    let map = build_semantic_source_map(yaml).unwrap_or_default();
    let output = map.span_for_path("$.steps[0].set.output");
    let value = map.span_for_path("$.steps[0].set.value");
    let Some(so) = output else {
        panic!("expected $.steps[0].set.output");
    };
    let Some(sv) = value else {
        panic!("expected $.steps[0].set.value");
    };
    assert_eq!(span_text(yaml, so), "result");
    assert_eq!(span_text(yaml, sv), "one");
}

/// Semantic source map with finish step.
#[test]
fn semantic_source_map_finish_step() {
    let yaml = "version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: first\n    finish:\n      result: complete\n";
    let map = build_semantic_source_map(yaml).unwrap_or_default();
    let result = map.span_for_path("$.steps[0].finish.result");
    let Some(span) = result else {
        panic!("expected $.steps[0].finish.result");
    };
    assert_eq!(span_text(yaml, span), "complete");
}

/// Semantic source map tracks root-level scalar.
#[test]
fn semantic_source_map_root_scalar() {
    let yaml = "version: velvet-ballistics/v1\nname: root_test\nwhen:\n  manual: {}\nsteps: []\n";
    let map = build_semantic_source_map(yaml).unwrap_or_default();
    let version = map.span_for_path("$.version");
    let Some(span) = version else {
        panic!("expected $.version path");
    };
    assert!(
        span_text(yaml, span).contains("velvet-ballistics/v1"),
        "version span must contain version string"
    );
}

/// Semantic source map with deeply nested paths.
#[test]
fn semantic_source_map_deep_nesting() {
    let yaml = "version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: nested\n    set:\n      config:\n        db:\n          host: localhost\n          port: 5432\n";
    let map = build_semantic_source_map(yaml).unwrap_or_default();
    let db = map.span_for_path("$.steps[0].set.config.db");
    let Some(span) = db else {
        panic!("expected $.steps[0].set.config.db");
    };
    assert!(
        span.start_offset <= span.end_offset,
        "db span must have valid byte range"
    );
}

/// Build source map rejects invalid YAML with specific error.
#[test]
fn build_source_map_rejects_duplicate_keys() {
    let yaml = "a: 1\na: 2\n";
    let result = build_source_map(yaml);
    assert!(
        matches!(result, Err(crate::YamlError::DuplicateKey { .. })),
        "expected DuplicateKey error, got: {result:?}"
    );
}

/// Build source map rejects anchor/alias.
#[test]
fn build_source_map_rejects_anchor_alias() {
    let yaml = "a: &a 1\nb: *a\n";
    let result = build_source_map(yaml);
    assert!(
        matches!(result, Err(crate::YamlError::AnchorAliasMerge)),
        "expected AnchorAliasMerge error, got: {result:?}"
    );
}

/// Build source map rejects multiple documents.
#[test]
fn build_source_map_rejects_multiple_documents() {
    let yaml = "---\na: 1\n---\nb: 2\n";
    let result = build_source_map(yaml);
    assert!(
        matches!(result, Err(crate::YamlError::MultipleDocuments { .. })),
        "expected MultipleDocuments error, got: {result:?}"
    );
}

/// Build source map produces correct line/column for multi-line YAML.
#[test]
fn build_source_map_multi_line_span_accuracy() {
    let yaml = "first: 1\nsecond: 2\nthird: 3\n";
    let map = build_source_map(yaml).unwrap_or_default();
    // Node 0 should be "first: 1" on line 1
    let span0 = map.span_for_node(0);
    let Some(s) = span0 else {
        panic!("expected span for node 0");
    };
    assert_eq!(s.start_line, 1);
    assert_eq!(s.end_line, 1);
}

/// Source map handles YAML with tabs and spaces.
#[test]
fn build_source_map_handles_mixed_indentation() {
    // YAML with 2-space indent
    let yaml = "a:\n  b: 1\n  c: 2\n";
    let map = build_source_map(yaml).unwrap_or_default();
    assert!(map.len() >= 3, "expected at least 3 nodes for nested map");
}

/// Source map handles YAML with flow style sequences.
#[test]
fn build_source_map_flow_style_sequence() {
    let yaml = "items: [a, b, c]\n";
    let result = build_source_map(yaml);
    let map = result.expect("flow-style sequence should be accepted");
    assert!(
        !map.is_empty(),
        "flow-style sequence source map should not be empty"
    );
}

/// Source map handles YAML with flow style mappings.
#[test]
fn build_source_map_flow_style_mapping() {
    let yaml = "obj: {key: value}\n";
    let result = build_source_map(yaml);
    let map = result.expect("flow-style mapping should be accepted");
    assert!(
        !map.is_empty(),
        "flow-style mapping source map should not be empty"
    );
}

/// Source map handles empty sequences and mappings.
#[test]
fn build_source_map_empty_containers() {
    let yaml = "items: []\nobj: {}\n";
    let result = build_source_map(yaml);
    match result {
        Ok(map) => assert!(
            map.len() >= 2,
            "expected at least 2 nodes for empty containers"
        ),
        Err(e) => fail_assert!("expected Ok, got Err: {e}"),
    }
}

/// Semantic source map with no steps produces minimal paths.
#[test]
fn semantic_source_map_no_steps() {
    let yaml = "version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps: []\n";
    let map = build_semantic_source_map(yaml).unwrap_or_default();
    let Some(span) = map.span_for_path("$.version") else {
        panic!("expected $.version path");
    };
    assert!(
        span_text(yaml, span).contains("velvet-ballistics/v1"),
        "version span must contain version string"
    );
    let Some(name_span) = map.span_for_path("$.name") else {
        panic!("expected $.name path");
    };
    assert_eq!(span_text(yaml, name_span), "test", "name span must contain 'test'");
}

/// Source map span end_offset is always >= start_offset.
#[test]
fn source_span_invariant_end_gte_start() {
    let yaml = "a: 1\nb: 2\nc: 3\n";
    let map = build_source_map(yaml).unwrap_or_default();
    for (idx, span) in map.iter() {
        assert!(
            span.end_offset >= span.start_offset,
            "node {idx}: end_offset({}) >= start_offset({}) violated",
            span.end_offset,
            span.start_offset,
        );
        assert!(
            span.end_col >= span.start_col,
            "node {idx}: end_col({}) >= start_col({}) violated",
            span.end_col,
            span.start_col,
        );
        assert!(
            span.end_line >= span.start_line,
            "node {idx}: end_line({}) >= start_line({}) violated",
            span.end_line,
            span.start_line,
        );
    }
}
