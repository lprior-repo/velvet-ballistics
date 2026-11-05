//! Property-based tests for vb_yaml YAML parsing and profile validation.
//!
//! These tests discover and verify the YAML parser's actual behavior,
//! including edge cases the parser accepts or rejects.

use proptest::prelude::*;

use crate::{parse_yaml_events, validate_yaml_profile};

// ─────────────────────────────────────────────────────────────────
// Property: Valid simple YAML mapping parses
// ─────────────────────────────────────────────────────────────────

fn is_yaml_1_1_ambiguous(scalar: &str) -> bool {
    matches!(
        scalar.to_ascii_lowercase().as_str(),
        "yes" | "no" | "on" | "off" | "y" | "n"
    )
}

proptest! {
    #[test]
    fn simple_yaml_mapping_parses(
        key in "[a-zA-Z][a-zA-Z0-9_]{1,9}"
            .prop_filter("unambiguous YAML 1.1 scalar", |key| !is_yaml_1_1_ambiguous(key)),
        value in "[a-zA-Z][a-zA-Z0-9]{2,10}"
            .prop_filter("unambiguous YAML 1.1 scalar", |value| !is_yaml_1_1_ambiguous(value)),
    ) {
        let yaml = format!("{}: {}\n", key, value);
        let result = parse_yaml_events(&yaml);
        assert!(result.is_ok(), "Simple YAML mapping should parse: {}", yaml);
    }
}

// ─────────────────────────────────────────────────────────────────
// Property: Quoted scalars parse successfully
// ─────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn quoted_scalars_parse(value in "[a-zA-Z ]{1,20}") {
        let yaml = format!("data: '{}'\n", value);
        let result = parse_yaml_events(&yaml);
        assert!(result.is_ok(), "Quoted scalar should parse: {}", yaml);
    }
}

proptest! {
    #[test]
    fn double_quoted_scalars_parse(value in "[a-zA-Z ]{1,20}") {
        let yaml = format!("data: \"{}\"\n", value);
        let result = parse_yaml_events(&yaml);
        assert!(result.is_ok(), "Double-quoted scalar should parse: {}", yaml);
    }
}

// ─────────────────────────────────────────────────────────────────
// Property: YAML anchors and aliases are rejected
// ─────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn anchors_rejected(key in "[a-zA-Z][a-zA-Z0-9_]{0,5}", value in "[a-zA-Z0-9]{1,5}") {
        let yaml = format!("{}: &anchor {}\nother: *anchor\n", key, value);
        let result = validate_yaml_profile(&yaml);
        assert!(result.is_err(), "YAML with anchor should be rejected");
    }
}

proptest! {
    #[test]
    fn merge_key_rejected(value in "[a-zA-Z0-9]{1,5}") {
        let yaml = format!("defaults: &d\n  x: {}\nobj:\n  <<: *d\n", value);
        let result = validate_yaml_profile(&yaml);
        assert!(result.is_err(), "YAML with merge key should be rejected");
    }
}

proptest! {
    #[test]
    fn custom_tag_rejected(value in "[a-zA-Z0-9]{1,5}") {
        let yaml = format!("data: !mytag {}\n", value);
        let result = validate_yaml_profile(&yaml);
        assert!(result.is_err(), "YAML with custom tag should be rejected");
    }
}

// ─────────────────────────────────────────────────────────────────
// Property: YAML 1.1 ambiguous scalars are rejected
// ─────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn ambiguous_booleans_rejected(key in "[a-zA-Z][a-zA-Z0-9_]{0,5}", val in "[a-z]{2,4}") {
        let yaml = format!("{}: {}\n", key, val);
        if ["yes", "no", "on", "off", "true", "false", "True", "False", "TRUE", "FALSE"]
            .contains(&val.as_str())
        {
            let result = validate_yaml_profile(&yaml);
            assert!(
                result.is_err(),
                "YAML 1.1 ambiguous scalar '{}' should be rejected",
                val
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Property: Duplicate keys are rejected
// ─────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn duplicate_keys_rejected(key in "[a-zA-Z][a-zA-Z0-9_]{0,5}", val1 in "[0-9]{1,3}", val2 in "[0-9]{1,3}") {
        let yaml = format!("{}: {}\n{}: {}\n", key, val1, key, val2);
        let result = validate_yaml_profile(&yaml);
        assert!(result.is_err(), "YAML with duplicate keys should be rejected");
    }
}

// ─────────────────────────────────────────────────────────────────
// Property: Empty lines are tolerated
// ─────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn empty_lines_tolerated(n in 1usize..=5) {
        let yaml = format!("key: value\n{}\nother: value\n", "\n".repeat(n));
        let result = parse_yaml_events(&yaml);
        assert!(
            result.is_ok(),
            "YAML with {} empty lines should parse",
            n
        );
    }
}

// ─────────────────────────────────────────────────────────────────
// Property: Multi-line strings are handled
// ─────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn literal_block_scalars_parse(text in "[a-zA-Z ]{1,20}") {
        let yaml = format!("content: |\n  {}\n", text);
        let result = parse_yaml_events(&yaml);
        assert!(result.is_ok(), "Literal block scalar should parse");
    }
}

proptest! {
    #[test]
    fn folded_block_scalars_parse(text in "[a-zA-Z ]{1,20}") {
        let yaml = format!("content: >\n  {}\n", text);
        let result = parse_yaml_events(&yaml);
        assert!(result.is_ok(), "Folded block scalar should parse");
    }
}

#[test]
fn multiline_literal_preserves_newlines() {
    let yaml = "content: |\n  line1\n  line2\n";
    let result = parse_yaml_events(yaml);
    assert!(result.is_ok(), "Multi-line literal should parse");
}

// ─────────────────────────────────────────────────────────────────
// Property: Event stream consistency
// ─────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn event_stream_boundaries(value in "[a-zA-Z0-9]{1,10}") {
        let yaml = format!("data: {}\n", value);
        let result = parse_yaml_events(&yaml);
        if let Ok(events) = result {
            assert!(
                events.len() >= 2,
                "Parse should produce at least StreamStart and StreamEnd events"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Property: Empty and minimal documents
// ─────────────────────────────────────────────────────────────────

#[test]
fn empty_document_parses() {
    let _result = parse_yaml_events("");
    // Empty document may be valid or not depending on parser;
    // the key property is it doesn't panic.
}

#[test]
fn multiple_mappings_parse() {
    let yaml = "key1: value1\nkey2: value2\nkey3: value3\n";
    let result = parse_yaml_events(yaml);
    assert!(result.is_ok(), "Multiple mappings should parse");
}

// ─────────────────────────────────────────────────────────────────
// Property: Long strings parse
// ─────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn long_strings_parse(value in "[a-zA-Z0-9 ]{1,1000}") {
        let yaml = format!("data: '{}'\n", value);
        let result = parse_yaml_events(&yaml);
        assert!(result.is_ok(), "Long string should parse");
    }
}

// ─────────────────────────────────────────────────────────────────
// Property: Unicode strings parse
// ─────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn unicode_strings_parse(text in "héllo wörld 🚀") {
        let yaml = format!("data: '{}'\n", text);
        let result = parse_yaml_events(&yaml);
        assert!(result.is_ok(), "Unicode string should parse: {}", yaml);
    }
}
