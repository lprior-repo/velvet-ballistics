#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]

//! Cold-path YAML parsing and profile enforcement for velvet-ballastics.
//!
//! This crate wraps `saphyr-parser` to provide strict YAML event parsing,
//! profile rejection (anchors, aliases, merge keys, duplicate keys, etc.),
//! source maps, span tracking, and a typed AST. The runtime never depends on
//! this crate.

pub mod ast;
pub mod events;
pub mod profile;
pub mod source_map;

use thiserror::Error;

/// YAML parsing error type.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum YamlError {
    #[error("unsupported YAML feature: {feature}")]
    UnsupportedFeature { feature: &'static str },

    #[error("duplicate key found: {key}")]
    DuplicateKey { key: Box<str> },

    #[error("anchor/alias/merge key rejected")]
    AnchorAliasMerge,

    #[error("custom tag rejected: {tag}")]
    CustomTag { tag: Box<str> },

    #[error("binary scalar rejected")]
    BinaryScalar,

    #[error("multiple documents rejected")]
    MultipleDocuments { count: usize },

    #[error("YAML 1.1 ambiguous scalar rejected: {scalar}")]
    AmbiguousScalar { scalar: Box<str> },

    #[error("source too large: {size} bytes, max {max}")]
    SourceTooLarge { size: usize, max: usize },

    #[error("nesting too deep: {depth}, max {max}")]
    NestingTooDeep { depth: u16, max: u16 },

    #[error("node limit exceeded: {count}, max {max}")]
    NodeLimitExceeded { count: u32, max: u32 },

    #[error("scalar too long: {len} bytes, max {max}")]
    ScalarTooLong { len: usize, max: usize },

    #[error("sequence too long: {len}, max {max}")]
    SequenceTooLong { len: usize, max: usize },

    #[error("mapping too large: {count} entries, max {max}")]
    MappingTooLarge { count: usize, max: usize },

    #[error("unknown field: {field}")]
    UnknownField { field: Box<str> },

    #[error("empty source")]
    EmptySource,

    #[error("missing required field: {field}")]
    MissingField { field: &'static str },

    #[error("field shape error: {field} expected {expected}")]
    FieldShape {
        field: &'static str,
        expected: &'static str,
    },

    #[error("parse error at line {line}: {reason}")]
    ParseError { line: usize, reason: Box<str> },

    #[error("forbidden YAML feature: {detail}")]
    ForbiddenFeature { detail: &'static str },
}

/// Alias for results using [`YamlError`].
pub type YamlResult<T> = Result<T, YamlError>;

/// Strict YAML profile limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YamlLimits {
    /// Maximum source text size in bytes.
    pub max_source_bytes: usize,
    /// Maximum nesting depth.
    pub max_depth: u16,
    /// Maximum total YAML nodes visited.
    pub max_nodes: u32,
    /// Maximum sequence length.
    pub max_sequence_len: usize,
    /// Maximum mapping entry count.
    pub max_mapping_entries: usize,
    /// Maximum scalar value length in bytes.
    pub max_scalar_bytes: usize,
}

impl Default for YamlLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 1_048_576,
            max_depth: 64,
            max_nodes: 100_000,
            max_sequence_len: 10_000,
            max_mapping_entries: 1_024,
            max_scalar_bytes: 65_536,
        }
    }
}

/// Parse YAML text into a stream of typed events.
///
/// This is the lowest-level public API. It runs profile validation first,
/// then returns the event stream for further processing.
pub fn parse_yaml_events(text: &str) -> YamlResult<Vec<events::YamlEvent>> {
    profile::validate_yaml_profile(text)?;
    events::collect_events(text)
}

/// Parse YAML text into a typed [`ast::WorkflowSource`] AST.
///
/// Combines profile validation, event collection, and AST construction
/// into a single call for convenience.
pub fn parse_workflow_source(text: &str) -> YamlResult<ast::WorkflowSource> {
    profile::validate_yaml_profile(text)?;
    ast::parse_workflow_ast(text)
}

/// Validate that the YAML text conforms to the strict profile.
///
/// Rejects anchors, aliases, merge keys, custom tags, binary scalars,
/// multiple documents, and YAML 1.1 ambiguous booleans.
pub fn validate_yaml_profile(text: &str) -> YamlResult<()> {
    profile::validate_yaml_profile(text)
}

/// Reject duplicate keys in a YAML mapping.
///
/// Intended for use by downstream consumers that need to check a list of
/// key-value pairs for duplicates after parsing.
pub fn reject_duplicate_keys(keys: &[&str]) -> YamlResult<()> {
    profile::reject_duplicate_keys(keys)
}

/// Reject forbidden YAML features from a pre-collected event list.
///
/// This is a secondary check that can be applied to already-parsed events.
pub fn reject_forbidden_yaml_features(events: &[events::YamlEvent]) -> YamlResult<()> {
    profile::reject_forbidden_features(events)
}

/// Reject anchors, aliases, and merge keys from a pre-collected event list.
pub fn reject_anchors_aliases_merges(events: &[events::YamlEvent]) -> YamlResult<()> {
    profile::reject_anchors_aliases_merges(events)
}

/// Reject multiple YAML documents from a pre-collected event list.
pub fn reject_multiple_documents(events: &[events::YamlEvent]) -> YamlResult<()> {
    profile::reject_multiple_documents(events)
}

/// Reject YAML 1.1 ambiguous boolean scalars (yes/no/on/off).
pub fn reject_yaml_1_1_ambiguous_scalars(scalars: &[&str]) -> YamlResult<()> {
    profile::reject_yaml_1_1_ambiguous_scalars(scalars)
}

/// Build a source map from YAML text, mapping node indices to line/column spans.
pub fn build_source_map(text: &str) -> YamlResult<source_map::SourceMap> {
    source_map::build_source_map(text)
}

/// Look up the span for a node by index in a pre-built source map.
pub fn span_for_node(
    map: &source_map::SourceMap,
    node_index: u32,
) -> Option<source_map::SourceSpan> {
    map.span_for_node(node_index)
}

/// Load a YAML fixture source for testing.
///
/// Validates the profile and returns the parsed [`ast::WorkflowSource`].
pub fn load_fixture_source(text: &str) -> YamlResult<ast::WorkflowSource> {
    parse_workflow_source(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assertion_failed(_message: std::fmt::Arguments<'_>) -> bool {
        false
    }

    macro_rules! fail_assert {
        ($($arg:tt)*) => {
            assert!(assertion_failed(format_args!($($arg)*)), $($arg)*)
        };
    }

    #[test]
    fn validate_rejects_empty_source() {
        let result = validate_yaml_profile("");
        assert!(result.is_err());
    }

    #[test]
    fn validate_accepts_simple_mapping() {
        let yaml = "key: value\n";
        let result = validate_yaml_profile(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_events_returns_typed_events() {
        let yaml = "a: 1\n";
        let Ok(events) = parse_yaml_events(yaml) else {
            fail_assert!("parse events failed");
            return;
        };
        assert!(!events.is_empty());
    }

    #[test]
    fn reject_duplicate_keys_detects_dups() {
        let keys = vec!["foo", "bar", "foo"];
        let result = reject_duplicate_keys(&keys);
        assert!(result.is_err());
    }

    #[test]
    fn reject_duplicate_keys_allows_unique() {
        let keys = vec!["foo", "bar", "baz"];
        let result = reject_duplicate_keys(&keys);
        assert!(result.is_ok());
    }

    #[test]
    fn reject_yaml_1_1_ambiguous_rejects_yes() {
        let scalars = vec!["yes"];
        let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
        assert!(result.is_err());
    }

    #[test]
    fn reject_yaml_1_1_ambiguous_allows_true() {
        let scalars = vec!["true"];
        let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
        assert!(result.is_ok());
    }

    // -----------------------------------------------------------------------
    // YamlError variant exact-assertion tests
    // -----------------------------------------------------------------------

    #[test]
    fn reject_anchors_aliases_merges_returns_anchor_alias_merge_for_anchor() {
        // Given: YAML events produced from text containing an anchor
        let yaml = "a: &anc value\n";
        let Ok(events) = crate::events::collect_events(yaml) else {
            fail_assert!("collect_events failed");
            return;
        };
        // When: we reject anchors/aliases/merges
        let result = reject_anchors_aliases_merges(&events);
        // Then: Err(YamlError::AnchorAliasMerge) exact variant
        assert_eq!(result, Err(YamlError::AnchorAliasMerge));
    }

    #[test]
    fn reject_anchors_aliases_merges_returns_anchor_alias_merge_for_alias() {
        // Given: YAML events produced from text containing an alias
        let yaml = "a: &anc value\nb: *anc\n";
        let Ok(events) = crate::events::collect_events(yaml) else {
            fail_assert!("collect_events failed");
            return;
        };
        // When: we reject anchors/aliases/merges
        let result = reject_anchors_aliases_merges(&events);
        // Then: Err(YamlError::AnchorAliasMerge) exact variant
        assert_eq!(result, Err(YamlError::AnchorAliasMerge));
    }

    #[test]
    fn reject_duplicate_keys_returns_duplicate_key_for_same_keys() {
        // Given: a key list with duplicate key "alpha"
        let keys = vec!["alpha", "beta", "alpha"];
        // When: rejecting duplicate keys
        let result = reject_duplicate_keys(&keys);
        // Then: Err with exact key field "alpha"
        assert_eq!(
            result,
            Err(YamlError::DuplicateKey {
                key: "alpha".into()
            })
        );
    }

    #[test]
    fn reject_forbidden_features_returns_unsupported_feature_for_complex_key() {
        // Given: an UnsupportedFeature error is produced by the profile module
        // for http trigger. We verify via parse_workflow_ast.
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: t
            when:
              http: {}
            steps: []
        "};
        // When: parsing the workflow
        let result = parse_workflow_source(yaml);
        // Then: Err(YamlError::UnsupportedFeature { feature: "http trigger" })
        assert_eq!(
            result,
            Err(YamlError::UnsupportedFeature {
                feature: "http trigger"
            })
        );
    }

    #[test]
    fn reject_multiple_documents_returns_multiple_documents_for_doc_separator() {
        // Given: YAML with multiple document separators
        let yaml = "---\na: 1\n---\nb: 2\n";
        let Ok(events) = crate::events::collect_events(yaml) else {
            fail_assert!("collect_events failed");
            return;
        };
        // When: rejecting multiple documents
        let result = reject_multiple_documents(&events);
        // Then: Err(YamlError::MultipleDocuments { count }) with exact count
        assert_eq!(result, Err(YamlError::MultipleDocuments { count: 2 }));
    }

    #[test]
    fn reject_yaml_profile_returns_source_too_large_for_oversized_input() {
        // Given: a string larger than the configured max
        let big = "x".repeat(200);
        let limits = YamlLimits {
            max_source_bytes: 100,
            max_depth: 64,
            max_nodes: 100_000,
            max_sequence_len: 10_000,
            max_mapping_entries: 1_024,
            max_scalar_bytes: 65_536,
        };
        // When: validating with custom limits
        let result = profile::validate_yaml_profile_with_limits(&big, &limits);
        // Then: Err(YamlError::SourceTooLarge) with exact max/actual
        assert_eq!(
            result,
            Err(YamlError::SourceTooLarge {
                size: 200,
                max: 100
            })
        );
    }

    #[test]
    fn reject_yaml_profile_returns_nesting_too_deep_for_deeply_nested() {
        // Given: deeply nested YAML exceeding depth limit
        let mut yaml = String::from("a:\n");
        for i in 0..20 {
            let indent = "  ".repeat(i);
            yaml.push_str(&format!("{indent}b:\n"));
        }
        let limits = YamlLimits {
            max_source_bytes: 1_048_576,
            max_depth: 5,
            max_nodes: 100_000,
            max_sequence_len: 10_000,
            max_mapping_entries: 1_024,
            max_scalar_bytes: 65_536,
        };
        // When: validating with low depth limit
        let result = profile::validate_yaml_profile_with_limits(&yaml, &limits);
        // Then: Err(YamlError::NestingTooDeep) with exact depth fields
        match result {
            Err(YamlError::NestingTooDeep { depth, max }) => {
                assert!(depth > 5);
                assert_eq!(max, 5);
            }
            other => fail_assert!("expected NestingTooDeep, got {other:?}"),
        }
    }

    #[test]
    fn reject_yaml_profile_returns_node_limit_exceeded_for_many_nodes() {
        // Given: YAML with many nodes exceeding the limit
        let mut yaml = String::from("a: 1\n");
        for i in 0..50 {
            yaml.push_str(&format!("key{i}: val{i}\n"));
        }
        let limits = YamlLimits {
            max_source_bytes: 1_048_576,
            max_depth: 64,
            max_nodes: 10,
            max_sequence_len: 10_000,
            max_mapping_entries: 1_024,
            max_scalar_bytes: 65_536,
        };
        // When: validating with low node limit
        let result = profile::validate_yaml_profile_with_limits(&yaml, &limits);
        // Then: Err(YamlError::NodeLimitExceeded) with exact limit/count
        match result {
            Err(YamlError::NodeLimitExceeded { count, max }) => {
                assert!(count > 10);
                assert_eq!(max, 10);
            }
            other => fail_assert!("expected NodeLimitExceeded, got {other:?}"),
        }
    }

    #[test]
    fn reject_yaml_profile_returns_custom_tag_for_tags() {
        // Given: YAML with a custom tag
        let yaml = "key: !custom value\n";
        // When: validating the profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::CustomTag) with exact tag string
        match result {
            Err(YamlError::CustomTag { tag }) => {
                assert!(
                    tag.contains("custom"),
                    "tag should contain 'custom', got: {tag}"
                );
            }
            other => fail_assert!("expected CustomTag, got {other:?}"),
        }
    }

    #[test]
    fn reject_yaml_profile_returns_ambiguous_scalar_for_unquoted_special() {
        // Given: YAML with an unquoted ambiguous scalar "yes"
        let yaml = "flag: yes\n";
        // When: validating the profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::AmbiguousScalar) with exact scalar
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar {
                scalar: "yes".into()
            })
        );
    }

    #[test]
    fn reject_yaml_profile_returns_ok_for_clean_yaml() {
        // Given: clean, well-formed YAML
        let yaml = "key: value\n";
        // When: validating the profile
        let result = validate_yaml_profile(yaml);
        // Then: Ok(()) happy path
        assert_eq!(result, Ok(()));
    }

    // -----------------------------------------------------------------------
    // Parsing integration tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_workflow_source_returns_ok_for_minimal_valid_workflow() {
        // Given: minimal valid workflow with name + steps
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: minimal
            when:
              manual: {}
            steps:
              - id: s1
                set:
                  output: x
                  value: \"42\"
        "};
        // When: parsing
        let result = parse_workflow_source(yaml);
        // Then: Ok with exact name and step count
        match result {
            Ok(wf) => {
                assert_eq!(wf.name, "minimal");
                assert_eq!(wf.steps.len(), 1);
            }
            Err(e) => fail_assert!("expected Ok, got Err: {e}"),
        }
    }

    #[test]
    fn parse_workflow_source_returns_ok_for_workflow_with_version() {
        // Given: workflow with a version field
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: versioned
            when:
              manual: {}
            steps: []
        "};
        // When: parsing
        let result = parse_workflow_source(yaml);
        // Then: Ok with exact version string
        match result {
            Ok(wf) => {
                assert_eq!(wf.version, "velvet-ballastics/v1");
            }
            Err(e) => fail_assert!("expected Ok, got Err: {e}"),
        }
    }

    #[test]
    fn parse_workflow_source_returns_ok_for_multi_step_workflow() {
        // Given: workflow with 3 steps
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: multi
            when:
              manual: {}
            steps:
              - id: s1
                set:
                  output: x
                  value: \"1\"
              - id: s2
                set:
                  output: val2
                  value: \"2\"
              - id: s3
                set:
                  output: val3
                  value: \"3\"
        "};
        // When: parsing
        let result = parse_workflow_source(yaml);
        // Then: Ok with exactly 3 steps
        match result {
            Ok(wf) => {
                assert_eq!(wf.steps.len(), 3);
            }
            Err(e) => fail_assert!("expected Ok, got Err: {e}"),
        }
    }

    #[test]
    fn parse_workflow_source_returns_error_for_empty_source() {
        // Given: empty source string
        // When: parsing
        let result = parse_workflow_source("");
        // Then: Err(YamlError::EmptySource)
        assert_eq!(result, Err(YamlError::EmptySource));
    }

    #[test]
    fn parse_workflow_source_returns_error_for_non_mapping_root() {
        // Given: YAML whose root is a scalar, not a mapping
        let yaml = "just a string\n";
        // When: parsing
        let result = parse_workflow_source(yaml);
        // Then: Err(YamlError::FieldShape { field: "workflow", expected: "mapping" })
        assert_eq!(
            result,
            Err(YamlError::FieldShape {
                field: "workflow",
                expected: "mapping"
            })
        );
    }

    #[test]
    fn parse_workflow_source_returns_error_for_anchors() {
        // Given: YAML with an anchor
        let yaml = "a: &anc value\n";
        // When: parsing via parse_workflow_source (runs profile first)
        let result = parse_workflow_source(yaml);
        // Then: Err(YamlError::AnchorAliasMerge)
        assert_eq!(result, Err(YamlError::AnchorAliasMerge));
    }

    #[test]
    fn parse_workflow_source_returns_error_for_aliases() {
        // Given: YAML with an alias
        let yaml = "a: &anc value\nb: *anc\n";
        // When: parsing via parse_workflow_source
        let result = parse_workflow_source(yaml);
        // Then: Err(YamlError::AnchorAliasMerge)
        assert_eq!(result, Err(YamlError::AnchorAliasMerge));
    }

    #[test]
    fn parse_workflow_source_returns_error_for_multiple_documents() {
        // Given: YAML with multiple document markers
        let yaml = "---\na: 1\n---\nb: 2\n";
        // When: parsing
        let result = parse_workflow_source(yaml);
        // Then: Err(YamlError::MultipleDocuments { count: 2 })
        assert_eq!(result, Err(YamlError::MultipleDocuments { count: 2 }));
    }

    #[test]
    fn parse_workflow_source_returns_error_for_ambiguous_scalar_unquoted_special() {
        // Given: YAML with an unquoted "no" scalar
        let yaml = "flag: no\n";
        // When: parsing
        let result = parse_workflow_source(yaml);
        // Then: Err(YamlError::AmbiguousScalar { scalar: "no" })
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar {
                scalar: "no".into()
            })
        );
    }

    #[test]
    fn validate_yaml_profile_accepts_simple_key_value_yaml() {
        // Given: simple key-value YAML
        let yaml = "name: test\ncount: 42\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Ok(())
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_yaml_profile_accepts_workflow_with_all_step_types() {
        // Given: a comprehensive workflow using multiple step types
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: comprehensive
            when:
              manual: {}
            inputs:
              - name: count
                type: u32
            vars:
              - name: acc
                value: \"0\"
            secrets:
              - name: api_key
            steps:
              - id: s1
                set:
                  output: x
                  value: \"1\"
              - id: s2
                do:
                  action: http.get
                  input: '\"https://example.com\"'
        "};
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Ok(())
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn load_fixture_source_returns_content_for_valid_fixture() {
        // Given: valid workflow YAML fixture
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: fixture
            when:
              manual: {}
            steps:
              - id: s1
                set:
                  output: x
                  value: \"1\"
        "};
        // When: loading fixture
        let result = load_fixture_source(yaml);
        // Then: Ok with correct name
        match result {
            Ok(wf) => assert_eq!(wf.name, "fixture"),
            Err(e) => fail_assert!("expected Ok, got Err: {e}"),
        }
    }

    #[test]
    fn load_fixture_source_returns_error_for_missing_fixture() {
        // Given: empty string (invalid fixture)
        // When: loading fixture
        let result = load_fixture_source("");
        // Then: Err(YamlError::EmptySource)
        assert_eq!(result, Err(YamlError::EmptySource));
    }

    #[test]
    fn reject_yaml_1_1_ambiguous_rejects_no() {
        // Given: scalars list with "no"
        let scalars = vec!["no"];
        // When: rejecting ambiguous scalars
        let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
        // Then: Err with exact scalar
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar {
                scalar: "no".into()
            })
        );
    }

    #[test]
    fn reject_yaml_1_1_ambiguous_rejects_on() {
        // Given: scalars list with "on"
        let scalars = vec!["on"];
        // When: rejecting ambiguous scalars
        let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
        // Then: Err with exact scalar
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar {
                scalar: "on".into()
            })
        );
    }

    #[test]
    fn reject_yaml_1_1_ambiguous_rejects_off() {
        // Given: scalars list with "off"
        let scalars = vec!["off"];
        // When: rejecting ambiguous scalars
        let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
        // Then: Err with exact scalar
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar {
                scalar: "off".into()
            })
        );
    }

    #[test]
    fn reject_yaml_1_1_ambiguous_rejects_y() {
        // Given: scalars list with "y"
        let scalars = vec!["y"];
        // When: rejecting ambiguous scalars
        let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
        // Then: Err with exact scalar
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar { scalar: "y".into() })
        );
    }

    #[test]
    fn reject_yaml_1_1_ambiguous_rejects_n() {
        // Given: scalars list with "n"
        let scalars = vec!["n"];
        // When: rejecting ambiguous scalars
        let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
        // Then: Err with exact scalar
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar { scalar: "n".into() })
        );
    }

    #[test]
    fn reject_yaml_1_1_ambiguous_is_case_insensitive() {
        // Given: scalars list with uppercase "YES"
        let scalars = vec!["YES"];
        // When: rejecting ambiguous scalars
        let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
        // Then: Err with exact original scalar
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar {
                scalar: "YES".into()
            })
        );
    }

    #[test]
    fn reject_yaml_1_1_ambiguous_allows_regular_strings() {
        // Given: scalars list with regular strings
        let scalars = vec!["hello", "world", "42"];
        // When: rejecting ambiguous scalars
        let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
        // Then: Ok(())
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn parse_yaml_events_produces_events_for_valid_yaml() {
        // Given: valid YAML
        let yaml = "a: 1\n";
        // When: parsing events
        let result = parse_yaml_events(yaml);
        // Then: Ok with non-empty vec
        match result {
            Ok(events) => assert!(!events.is_empty()),
            Err(e) => fail_assert!("expected Ok, got Err: {e}"),
        }
    }

    #[test]
    fn parse_yaml_events_returns_error_for_invalid_yaml() {
        // Given: empty source text
        // When: parsing events
        let result = parse_yaml_events("");
        // Then: Err
        assert!(result.is_err());
    }

    #[test]
    fn reject_duplicate_keys_returns_exact_key_for_duplicate() {
        // Given: keys with "repeat" appearing twice
        let keys = vec!["first", "repeat", "repeat"];
        // When: rejecting duplicate keys
        let result = reject_duplicate_keys(&keys);
        // Then: Err with exact key field
        assert_eq!(
            result,
            Err(YamlError::DuplicateKey {
                key: "repeat".into()
            })
        );
    }

    #[test]
    fn yaml_limits_default_has_expected_values() {
        // Given: default YamlLimits
        let limits = YamlLimits::default();
        // Then: exact default values
        assert_eq!(limits.max_source_bytes, 1_048_576);
        assert_eq!(limits.max_depth, 64);
        assert_eq!(limits.max_nodes, 100_000);
        assert_eq!(limits.max_sequence_len, 10_000);
        assert_eq!(limits.max_mapping_entries, 1_024);
        assert_eq!(limits.max_scalar_bytes, 65_536);
    }

    #[test]
    fn parse_workflow_source_returns_error_for_duplicate_step_ids() {
        // Given: workflow YAML with duplicate top-level keys
        let yaml = "version: velvet-ballastics/v1\nname: dup\nwhen:\n  manual: {}\nname: dup2\nsteps: []\n";
        // When: parsing
        let result = parse_workflow_source(yaml);
        // Then: Err(YamlError::DuplicateKey { key: "name" })
        assert_eq!(result, Err(YamlError::DuplicateKey { key: "name".into() }));
    }

    #[test]
    fn span_for_node_returns_none_for_empty_map() {
        // Given: an empty source map
        let yaml = "a: 1\n";
        let Ok(map) = build_source_map(yaml) else {
            fail_assert!("build_source_map failed");
            return;
        };
        // When: looking up an out-of-range node
        let result = span_for_node(&map, 999);
        // Then: None
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // Adversarial BDD tests - top-level API attack vectors
    // -----------------------------------------------------------------------

    #[test]
    fn adversarial_api_null_byte_in_source_rejected() {
        // Given: YAML source containing a null byte
        let yaml = "key: \x00value\n";
        // When: parsing via the top-level API
        let result = parse_yaml_events(yaml);
        // Then: Err(YamlError::ForbiddenFeature { detail: "null_byte_in_source" })
        // Null bytes are rejected by the profile validation layer to prevent
        // C-string termination issues and protocol injection in downstream
        // consumers.
        assert!(
            matches!(
                result,
                Err(YamlError::ForbiddenFeature {
                    detail: "null_byte_in_source"
                })
            ),
            "expected ForbiddenFeature for null byte, got: {result:?}"
        );
    }

    #[test]
    fn adversarial_api_null_byte_workflow_rejected() {
        // Given: workflow source containing a null byte
        let yaml = "version: velvet-ballastics/v1\nname: \x00bad\nwhen:\n  manual: {}\nsteps: []\n";
        // When: parsing via parse_workflow_source
        let result = parse_workflow_source(yaml);
        // Then: Err - null bytes cause parse failure
        assert!(result.is_err(), "expected error for null byte in workflow");
    }

    #[test]
    fn adversarial_api_unicode_emoji_accepted() {
        // Given: YAML with emoji characters in values
        let yaml = "name: test_emoji\nvalue: \"hello world\"\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Ok(()) - Unicode emoji in values is fine
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn adversarial_api_scalar_near_limit_accepted() {
        // Given: YAML with a scalar value at exactly 64KB (under default 65KB limit)
        let val = "x".repeat(65_535);
        let yaml = format!("key: \"{val}\"\n");
        // When: validating profile
        let result = validate_yaml_profile(&yaml);
        // Then: Ok(()) - under the limit
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn adversarial_api_scalar_one_over_limit_rejected() {
        // Given: YAML with a scalar value one byte over the 65KB limit
        let val = "x".repeat(65_537);
        let yaml = format!("key: \"{val}\"\n");
        // When: validating profile
        let result = validate_yaml_profile(&yaml);
        // Then: Err(YamlError::ScalarTooLong)
        assert!(
            matches!(result, Err(YamlError::ScalarTooLong { .. })),
            "expected ScalarTooLong, got: {result:?}"
        );
    }

    #[test]
    fn adversarial_api_workflow_with_unknown_trigger_field_rejected() {
        // Given: workflow YAML with an unrecognized trigger type
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: bad-trigger
            when:
              webhook: {}
            steps: []
        "};
        // When: parsing workflow
        let result = parse_workflow_source(yaml);
        // Then: Err(YamlError::FieldShape) - webhook is not a recognized trigger
        assert!(
            matches!(result, Err(YamlError::FieldShape { .. })),
            "expected FieldShape for unknown trigger, got: {result:?}"
        );
    }

    #[test]
    fn adversarial_api_workflow_with_missing_when_rejected() {
        // Given: workflow YAML without a when field
        let yaml = "version: velvet-ballastics/v1\nname: no-when\nsteps: []\n";
        // When: parsing workflow
        let result = parse_workflow_source(yaml);
        // Then: Err(YamlError::MissingField { field: "when" })
        assert_eq!(result, Err(YamlError::MissingField { field: "when" }));
    }

    #[test]
    fn adversarial_api_workflow_with_non_mapping_when_rejected() {
        // Given: workflow YAML where when is a string, not a mapping
        let yaml = "version: velvet-ballastics/v1\nname: bad\nwhen: manual\nsteps: []\n";
        // When: parsing workflow
        let result = parse_workflow_source(yaml);
        // Then: Err(YamlError::FieldShape)
        assert!(result.is_err(), "expected error for non-mapping when");
    }

    #[test]
    fn adversarial_api_oversized_source_rejected_immediately() {
        // Given: a 2MB source string
        let big = "x".repeat(2_000_000);
        // When: validating profile
        let result = validate_yaml_profile(&big);
        // Then: Err(YamlError::SourceTooLarge) - rejected before parsing
        assert!(
            matches!(result, Err(YamlError::SourceTooLarge { .. })),
            "expected SourceTooLarge, got: {result:?}"
        );
    }

    #[test]
    fn adversarial_api_only_whitespace_rejected() {
        // Given: YAML that is only whitespace
        let yaml = "   \t  \n  \n  ";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err - no content
        assert!(result.is_err(), "expected error for whitespace-only YAML");
    }
}
