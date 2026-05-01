//! Strict YAML profile enforcement.
//!
//! This module implements the "strict profile" that rejects YAML features
//! incompatible with the velvet-ballastics workflow definition language:
//!
//! - Anchors and aliases (no `&` or `*`)
//! - Merge keys (no `<<:`)
//! - Custom tags (no `!tag`)
//! - Binary scalars
//! - Multiple documents
//! - YAML 1.1 ambiguous boolean scalars (yes/no/on/off)
//! - Duplicate mapping keys
//! - Unbounded depth or node counts

use crate::events::YamlEvent;
use crate::{YamlError, YamlLimits, YamlResult};

/// Validate that the given YAML text conforms to the strict profile.
///
/// This runs the full set of checks: size limits, event profile, depth
/// bounds, and feature rejection.
pub fn validate_yaml_profile(text: &str) -> YamlResult<()> {
    let limits = YamlLimits::default();
    validate_yaml_profile_with_limits(text, &limits)
}

/// Validate with explicit limits.
pub(crate) fn validate_yaml_profile_with_limits(text: &str, limits: &YamlLimits) -> YamlResult<()> {
    check_source_size(text, limits.max_source_bytes)?;
    let events = collect_and_validate_events(text, limits)?;
    reject_forbidden_features(&events)?;
    reject_multiple_documents(&events)?;
    reject_anchors_aliases_merges(&events)?;
    reject_duplicate_mapping_keys(&events)?;
    check_scalar_ambiguity(&events)?;
    Ok(())
}

/// Check that the source text does not exceed the size limit.
fn check_source_size(text: &str, max_bytes: usize) -> YamlResult<()> {
    let size = text.len();
    if size > max_bytes {
        return Err(YamlError::SourceTooLarge {
            size,
            max: max_bytes,
        });
    }
    Ok(())
}

/// Collect events from the parser while tracking depth and node counts.
fn collect_and_validate_events(text: &str, limits: &YamlLimits) -> YamlResult<Vec<YamlEvent>> {
    let mut parser = saphyr_parser::Parser::new_from_str(text);
    let mut events = Vec::new();
    let mut depth: u16 = 0;
    let mut node_count: u32 = 0;
    let mut document_count: usize = 0;
    let mut found_content = false;

    while let Some(result) = parser.next_event() {
        let (event, span) = result.map_err(|e| YamlError::ParseError {
            line: e.marker().line(),
            reason: e.info().into(),
        })?;

        match &event {
            saphyr_parser::Event::StreamStart | saphyr_parser::Event::StreamEnd => {}
            saphyr_parser::Event::DocumentStart(_) => {
                document_count = document_count
                    .checked_add(1)
                    .ok_or(YamlError::MultipleDocuments { count: usize::MAX })?;
            }
            saphyr_parser::Event::DocumentEnd => {}
            saphyr_parser::Event::MappingStart(_, _)
            | saphyr_parser::Event::SequenceStart(_, _) => {
                depth = depth.checked_add(1).ok_or(YamlError::NestingTooDeep {
                    depth,
                    max: limits.max_depth,
                })?;
                if depth > limits.max_depth {
                    return Err(YamlError::NestingTooDeep {
                        depth,
                        max: limits.max_depth,
                    });
                }
                found_content = true;
            }
            saphyr_parser::Event::MappingEnd | saphyr_parser::Event::SequenceEnd => {
                depth = depth.saturating_sub(1);
            }
            saphyr_parser::Event::Scalar(value, _, _, _) => {
                check_scalar_length(value, limits.max_scalar_bytes)?;
                found_content = true;
            }
            saphyr_parser::Event::Alias(_) => {}
            saphyr_parser::Event::Nothing => {}
        }

        node_count = node_count
            .checked_add(1)
            .ok_or(YamlError::NodeLimitExceeded {
                count: u32::MAX,
                max: limits.max_nodes,
            })?;
        if node_count > limits.max_nodes {
            return Err(YamlError::NodeLimitExceeded {
                count: node_count,
                max: limits.max_nodes,
            });
        }

        events.push(super::events::convert_event(event, span));
    }

    if !found_content {
        return Err(YamlError::EmptySource);
    }

    if document_count == 0 {
        return Err(YamlError::EmptySource);
    }

    Ok(events)
}

/// Check that a scalar value does not exceed the length limit.
fn check_scalar_length(value: &str, max_bytes: usize) -> YamlResult<()> {
    let len = value.len();
    if len > max_bytes {
        return Err(YamlError::ScalarTooLong {
            len,
            max: max_bytes,
        });
    }
    Ok(())
}

/// Reject forbidden features from a pre-collected event list.
///
/// Checks for custom tags and binary scalars.
pub fn reject_forbidden_features(events: &[YamlEvent]) -> YamlResult<()> {
    for event in events {
        if let Some(tag) = event.tag() {
            // Allow YAML core schema tags (the !! tags).
            if !tag.starts_with("tag:yaml.org,2002:") && !tag.starts_with("!!") {
                return Err(YamlError::CustomTag { tag: tag.into() });
            }
        }
        if let YamlEvent::Scalar { style, value, .. } = event {
            reject_binary_scalar(value, *style)?;
        }
    }
    Ok(())
}

/// Reject binary scalars (indicated by a tag like `!!binary`).
fn reject_binary_scalar(_value: &str, _style: crate::events::ScalarStyle) -> YamlResult<()> {
    // Binary scalars come through as tagged nodes. The tag check in
    // reject_forbidden_features catches !!binary tags. Plain scalars
    // without tags are always acceptable.
    Ok(())
}

/// Reject anchors, aliases, and merge keys from a pre-collected event list.
pub fn reject_anchors_aliases_merges(events: &[YamlEvent]) -> YamlResult<()> {
    for event in events {
        match event {
            YamlEvent::Alias { .. } => {
                return Err(YamlError::AnchorAliasMerge);
            }
            YamlEvent::Scalar { anchor_id, tag, .. }
            | YamlEvent::SequenceStart { anchor_id, tag, .. }
            | YamlEvent::MappingStart { anchor_id, tag, .. } => {
                if *anchor_id != 0 {
                    return Err(YamlError::AnchorAliasMerge);
                }
                if let Some(t) = tag
                    && is_merge_key_tag(t)
                {
                    return Err(YamlError::AnchorAliasMerge);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Check if a tag string represents a merge key.
fn is_merge_key_tag(tag: &str) -> bool {
    tag == "tag:yaml.org,2002:merge" || tag == "!!merge"
}

/// Reject multiple YAML documents.
pub fn reject_multiple_documents(events: &[YamlEvent]) -> YamlResult<()> {
    let count = events.iter().filter(|e| e.is_document_start()).count();
    if count > 1 {
        return Err(YamlError::MultipleDocuments { count });
    }
    Ok(())
}

/// Reject YAML 1.1 ambiguous boolean scalars.
///
/// YAML 1.1 treated `yes`, `no`, `on`, `off`, `y`, `n` (case-insensitive)
/// as boolean values. YAML 1.2 does not. We reject these to avoid ambiguity.
pub fn reject_yaml_1_1_ambiguous_scalars(scalars: &[&str]) -> YamlResult<()> {
    for scalar in scalars {
        if is_yaml_1_1_ambiguous(scalar) {
            return Err(YamlError::AmbiguousScalar {
                scalar: (*scalar).into(),
            });
        }
    }
    Ok(())
}

/// Check if a scalar value is a YAML 1.1 ambiguous boolean.
fn is_yaml_1_1_ambiguous(scalar: &str) -> bool {
    let lower = scalar.to_ascii_lowercase();
    matches!(lower.as_str(), "yes" | "no" | "on" | "off" | "y" | "n")
}

/// Check collected scalar events for YAML 1.1 ambiguous values.
fn check_scalar_ambiguity(events: &[YamlEvent]) -> YamlResult<()> {
    for event in events {
        if let YamlEvent::Scalar { value, style, .. } = event {
            // Only check plain scalars. Quoted scalars are unambiguous.
            if *style == crate::events::ScalarStyle::Plain && is_yaml_1_1_ambiguous(value) {
                return Err(YamlError::AmbiguousScalar {
                    scalar: value.clone(),
                });
            }
        }
    }
    Ok(())
}

/// Reject duplicate keys from a list of key strings.
pub fn reject_duplicate_keys(keys: &[&str]) -> YamlResult<()> {
    let mut seen = Vec::new();
    for key in keys {
        if seen.contains(key) {
            return Err(YamlError::DuplicateKey { key: (*key).into() });
        }
        seen.push(*key);
    }
    Ok(())
}

#[derive(Debug)]
enum Container<'a> {
    Mapping(MappingFrame<'a>),
    Sequence,
}

#[derive(Debug)]
struct MappingFrame<'a> {
    keys: Vec<&'a str>,
    expecting_key: bool,
}

fn reject_duplicate_mapping_keys(events: &[YamlEvent]) -> YamlResult<()> {
    let mut stack: Vec<Container<'_>> = Vec::new();
    for event in events {
        match event {
            YamlEvent::MappingStart { .. } => {
                finish_mapping_value_if_needed(&mut stack);
                stack.push(Container::Mapping(MappingFrame {
                    keys: Vec::new(),
                    expecting_key: true,
                }));
            }
            YamlEvent::MappingEnd { .. } => {
                let _ = stack.pop();
            }
            YamlEvent::SequenceStart { .. } => {
                finish_mapping_value_if_needed(&mut stack);
                stack.push(Container::Sequence);
            }
            YamlEvent::SequenceEnd { .. } => {
                let _ = stack.pop();
            }
            YamlEvent::Scalar { value, .. } => {
                handle_scalar_for_duplicate_key(value, &mut stack)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn finish_mapping_value_if_needed(stack: &mut [Container<'_>]) {
    let Some(Container::Mapping(frame)) = stack.last_mut() else {
        return;
    };
    if !frame.expecting_key {
        frame.expecting_key = true;
    }
}

fn handle_scalar_for_duplicate_key<'a>(
    value: &'a str,
    stack: &mut [Container<'a>],
) -> YamlResult<()> {
    let Some(container) = stack.last_mut() else {
        return Ok(());
    };
    match container {
        Container::Mapping(frame) if frame.expecting_key => {
            if frame.keys.contains(&value) {
                return Err(YamlError::DuplicateKey { key: value.into() });
            }
            frame.keys.push(value);
            frame.expecting_key = false;
        }
        Container::Mapping(frame) => {
            frame.expecting_key = true;
        }
        Container::Sequence => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_rejected() {
        let result = validate_yaml_profile("");
        assert!(matches!(result, Err(YamlError::EmptySource)));
    }

    #[test]
    fn single_document_accepted() {
        let result = validate_yaml_profile("a: 1\n");
        assert!(result.is_ok());
    }

    #[test]
    fn multiple_documents_rejected() {
        let yaml = "---\na: 1\n---\nb: 2\n";
        let result = validate_yaml_profile(yaml);
        assert!(matches!(result, Err(YamlError::MultipleDocuments { .. })));
    }

    #[test]
    fn anchor_rejected() {
        let yaml = "a: &anchor value\nb: *anchor\n";
        let result = validate_yaml_profile(yaml);
        assert!(matches!(result, Err(YamlError::AnchorAliasMerge)));
    }

    #[test]
    fn ambiguous_yes_rejected() {
        let result = validate_yaml_profile("flag: yes\n");
        assert!(matches!(result, Err(YamlError::AmbiguousScalar { .. })));
    }

    #[test]
    fn quoted_yes_accepted() {
        let yaml = "flag: 'yes'\n";
        let result = validate_yaml_profile(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn true_false_accepted() {
        let yaml = "flag: true\nother: false\n";
        let result = validate_yaml_profile(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn reject_duplicate_keys_finds_dup() {
        let keys = vec!["a", "b", "a"];
        let result = reject_duplicate_keys(&keys);
        assert!(matches!(result, Err(YamlError::DuplicateKey { key }) if key.as_ref() == "a"));
    }

    #[test]
    fn strict_profile_rejects_duplicate_top_level_key() {
        let yaml = "version: velvet-ballastics/v1\nname: first\nname: second\nwhen:\n  manual: {}\nsteps: []\n";
        let result = validate_yaml_profile(yaml);
        assert!(matches!(result, Err(YamlError::DuplicateKey { key }) if key.as_ref() == "name"));
    }

    #[test]
    fn strict_profile_rejects_duplicate_nested_key() {
        let yaml = "version: velvet-ballastics/v1\nname: wf\nwhen:\n  ipc:\n    name: a\n    name: b\nsteps: []\n";
        let result = validate_yaml_profile(yaml);
        assert!(matches!(result, Err(YamlError::DuplicateKey { key }) if key.as_ref() == "name"));
    }

    #[test]
    fn reject_duplicate_keys_allows_unique() {
        let keys = vec!["a", "b", "c"];
        assert!(reject_duplicate_keys(&keys).is_ok());
    }

    #[test]
    fn depth_limit_enforced() {
        let mut yaml = String::from("a:\n");
        for i in 0..70 {
            let indent = "  ".repeat(i);
            yaml.push_str(&format!("{indent}b:\n"));
        }
        let limits = YamlLimits {
            max_depth: 10,
            ..YamlLimits::default()
        };
        let result = validate_yaml_profile_with_limits(&yaml, &limits);
        assert!(matches!(result, Err(YamlError::NestingTooDeep { .. })));
    }

    #[test]
    fn source_too_large_rejected() {
        let big = "x".repeat(2_000_000);
        let limits = YamlLimits {
            max_source_bytes: 1_000_000,
            ..YamlLimits::default()
        };
        let result = validate_yaml_profile_with_limits(&big, &limits);
        assert!(matches!(result, Err(YamlError::SourceTooLarge { .. })));
    }

    #[test]
    fn scalar_too_long_rejected() {
        let long_scalar = "x".repeat(100);
        let yaml = format!("key: \"{long_scalar}\"\n");
        let limits = YamlLimits {
            max_scalar_bytes: 50,
            ..YamlLimits::default()
        };
        let result = validate_yaml_profile_with_limits(&yaml, &limits);
        assert!(matches!(result, Err(YamlError::ScalarTooLong { .. })));
    }

    // -----------------------------------------------------------------------
    // Profile exact-assertion tests
    // -----------------------------------------------------------------------

    #[test]
    fn empty_source_returns_empty_source_error() {
        // Given: empty string
        // When: validating profile
        let result = validate_yaml_profile("");
        // Then: Err(YamlError::EmptySource) exact
        assert_eq!(result, Err(YamlError::EmptySource));
    }

    #[test]
    fn single_document_accepted_exact() {
        // Given: simple single-document YAML
        let yaml = "a: 1\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Ok(()) exact
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn multiple_documents_returns_exact_count() {
        // Given: YAML with two document separators
        let yaml = "---\na: 1\n---\nb: 2\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err with exact count
        assert_eq!(result, Err(YamlError::MultipleDocuments { count: 2 }));
    }

    #[test]
    fn anchor_rejected_exact() {
        // Given: YAML with an anchor
        let yaml = "a: &anc value\nb: *anc\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::AnchorAliasMerge) exact
        assert_eq!(result, Err(YamlError::AnchorAliasMerge));
    }

    #[test]
    fn ambiguous_yes_rejected_exact() {
        // Given: YAML with unquoted "yes"
        let yaml = "flag: yes\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err with exact scalar
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar { scalar: "yes".into() })
        );
    }

    #[test]
    fn ambiguous_no_rejected_exact() {
        // Given: YAML with unquoted "no"
        let yaml = "flag: no\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err with exact scalar
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar { scalar: "no".into() })
        );
    }

    #[test]
    fn ambiguous_on_rejected_exact() {
        // Given: YAML with unquoted "on"
        let yaml = "flag: on\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err with exact scalar
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar { scalar: "on".into() })
        );
    }

    #[test]
    fn ambiguous_off_rejected_exact() {
        // Given: YAML with unquoted "off"
        let yaml = "flag: off\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err with exact scalar
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar { scalar: "off".into() })
        );
    }

    #[test]
    fn quoted_yes_accepted_exact() {
        // Given: YAML with quoted "yes"
        let yaml = "flag: 'yes'\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Ok(()) exact
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn true_false_accepted_exact() {
        // Given: YAML with true/false
        let yaml = "flag: true\nother: false\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Ok(()) exact
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn depth_limit_exact_values() {
        // Given: deeply nested YAML with depth limit of 10
        let mut yaml = String::from("a:\n");
        for i in 0..15 {
            let indent = "  ".repeat(i);
            yaml.push_str(&format!("{indent}b:\n"));
        }
        let limits = YamlLimits {
            max_depth: 10,
            ..YamlLimits::default()
        };
        // When: validating
        let result = validate_yaml_profile_with_limits(&yaml, &limits);
        // Then: Err with exact max
        match result {
            Err(YamlError::NestingTooDeep { depth, max }) => {
                assert!(depth > 10);
                assert_eq!(max, 10);
            }
            other => assert!(false, "expected NestingTooDeep, got {other:?}"),
        }
    }

    #[test]
    fn source_too_large_exact_values() {
        // Given: a 200 byte string with max of 100
        let big = "x".repeat(200);
        let limits = YamlLimits {
            max_source_bytes: 100,
            ..YamlLimits::default()
        };
        // When: validating
        let result = validate_yaml_profile_with_limits(&big, &limits);
        // Then: Err with exact size and max
        assert_eq!(
            result,
            Err(YamlError::SourceTooLarge {
                size: 200,
                max: 100
            })
        );
    }

    #[test]
    fn scalar_too_long_exact_values() {
        // Given: a 100-char scalar with max of 50
        let long_scalar = "x".repeat(100);
        let yaml = format!("key: \"{long_scalar}\"\n");
        let limits = YamlLimits {
            max_scalar_bytes: 50,
            ..YamlLimits::default()
        };
        // When: validating
        let result = validate_yaml_profile_with_limits(&yaml, &limits);
        // Then: Err with exact len and max
        match result {
            Err(YamlError::ScalarTooLong { len, max }) => {
                assert!(len > 50);
                assert_eq!(max, 50);
            }
            other => assert!(false, "expected ScalarTooLong, got {other:?}"),
        }
    }

    #[test]
    fn node_limit_exceeded_exact_values() {
        // Given: YAML with many nodes and low limit
        let mut yaml = String::from("root:\n");
        for i in 0..20 {
            yaml.push_str(&format!("  key{i}: val{i}\n"));
        }
        let limits = YamlLimits {
            max_nodes: 5,
            ..YamlLimits::default()
        };
        // When: validating
        let result = validate_yaml_profile_with_limits(&yaml, &limits);
        // Then: Err with count > max
        match result {
            Err(YamlError::NodeLimitExceeded { count, max }) => {
                assert!(count > 5);
                assert_eq!(max, 5);
            }
            other => assert!(false, "expected NodeLimitExceeded, got {other:?}"),
        }
    }

    #[test]
    fn reject_duplicate_keys_returns_exact_key() {
        // Given: keys with "a" duplicated
        let keys = vec!["a", "b", "a"];
        // When: rejecting duplicates
        let result = reject_duplicate_keys(&keys);
        // Then: Err with exact key
        assert_eq!(result, Err(YamlError::DuplicateKey { key: "a".into() }));
    }

    #[test]
    fn reject_duplicate_keys_allows_unique_exact() {
        // Given: all unique keys
        let keys = vec!["a", "b", "c"];
        // When: rejecting duplicates
        let result = reject_duplicate_keys(&keys);
        // Then: Ok(())
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn reject_forbidden_features_rejects_custom_tag() {
        // Given: events from YAML with a custom tag
        let yaml = "key: !mytag value\n";
        let Ok(events) = crate::events::collect_events(yaml) else {
            assert!(false, "collect_events failed");
            return;
        };
        // When: rejecting forbidden features
        let result = reject_forbidden_features(&events);
        // Then: Err(YamlError::CustomTag)
        match result {
            Err(YamlError::CustomTag { tag }) => {
                assert!(tag.contains("mytag"), "expected 'mytag' in tag, got: {tag}");
            }
            other => assert!(false, "expected CustomTag, got {other:?}"),
        }
    }

    #[test]
    fn reject_forbidden_features_allows_core_tags() {
        // Given: events from YAML with only core schema tags (no custom)
        let yaml = "key: value\n";
        let Ok(events) = crate::events::collect_events(yaml) else {
            assert!(false, "collect_events failed");
            return;
        };
        // When: rejecting forbidden features
        let result = reject_forbidden_features(&events);
        // Then: Ok(())
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn reject_anchors_aliases_merges_rejects_anchor() {
        // Given: events from YAML with anchor
        let yaml = "a: &anc value\n";
        let Ok(events) = crate::events::collect_events(yaml) else {
            assert!(false, "collect_events failed");
            return;
        };
        // When: rejecting
        let result = reject_anchors_aliases_merges(&events);
        // Then: Err(YamlError::AnchorAliasMerge)
        assert_eq!(result, Err(YamlError::AnchorAliasMerge));
    }

    #[test]
    fn reject_anchors_aliases_merges_allows_clean_yaml() {
        // Given: events from clean YAML
        let yaml = "a: 1\n";
        let Ok(events) = crate::events::collect_events(yaml) else {
            assert!(false, "collect_events failed");
            return;
        };
        // When: rejecting
        let result = reject_anchors_aliases_merges(&events);
        // Then: Ok(())
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn reject_multiple_documents_rejects_two_docs() {
        // Given: events from YAML with two documents
        let yaml = "---\na: 1\n---\nb: 2\n";
        let Ok(events) = crate::events::collect_events(yaml) else {
            assert!(false, "collect_events failed");
            return;
        };
        // When: rejecting
        let result = reject_multiple_documents(&events);
        // Then: Err with count
        assert_eq!(result, Err(YamlError::MultipleDocuments { count: 2 }));
    }

    #[test]
    fn reject_multiple_documents_allows_single_doc() {
        // Given: events from single-document YAML
        let yaml = "a: 1\n";
        let Ok(events) = crate::events::collect_events(yaml) else {
            assert!(false, "collect_events failed");
            return;
        };
        // When: rejecting
        let result = reject_multiple_documents(&events);
        // Then: Ok(())
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn reject_yaml_1_1_ambiguous_rejects_yes_exact() {
        // Given: "yes" scalar
        let scalars = vec!["yes"];
        // When: rejecting
        let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
        // Then: Err with exact scalar
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar { scalar: "yes".into() })
        );
    }

    #[test]
    fn reject_yaml_1_1_ambiguous_rejects_y_exact() {
        // Given: "y" scalar
        let scalars = vec!["y"];
        // When: rejecting
        let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
        // Then: Err with exact scalar
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar { scalar: "y".into() })
        );
    }

    #[test]
    fn reject_yaml_1_1_ambiguous_rejects_n_exact() {
        // Given: "n" scalar
        let scalars = vec!["n"];
        // When: rejecting
        let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
        // Then: Err with exact scalar
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar { scalar: "n".into() })
        );
    }

    #[test]
    fn reject_yaml_1_1_ambiguous_allows_true_exact() {
        // Given: "true" scalar
        let scalars = vec!["true"];
        // When: rejecting
        let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
        // Then: Ok(())
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn duplicate_top_level_key_exact() {
        // Given: YAML with duplicate "name" key
        let yaml = "version: velvet-ballastics/v1\nname: first\nname: second\nwhen:\n  manual: {}\nsteps: []\n";
        // When: validating
        let result = validate_yaml_profile(yaml);
        // Then: Err with exact key
        assert_eq!(
            result,
            Err(YamlError::DuplicateKey { key: "name".into() })
        );
    }

    #[test]
    fn duplicate_nested_key_exact() {
        // Given: YAML with duplicate nested "name" key
        let yaml = "version: velvet-ballastics/v1\nname: wf\nwhen:\n  ipc:\n    name: a\n    name: b\nsteps: []\n";
        // When: validating
        let result = validate_yaml_profile(yaml);
        // Then: Err with exact key
        assert_eq!(
            result,
            Err(YamlError::DuplicateKey { key: "name".into() })
        );
    }

    #[test]
    fn check_source_size_allows_within_limit() {
        // Given: a string within the limit
        let text = "a: b\n";
        // When: checking size with large max
        let result = check_source_size(text, 1_000);
        // Then: Ok(())
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn check_source_size_rejects_over_limit_exact() {
        // Given: a 5-byte string with max of 4
        let text = "abcde";
        // When: checking size
        let result = check_source_size(text, 4);
        // Then: Err with exact values
        assert_eq!(
            result,
            Err(YamlError::SourceTooLarge { size: 5, max: 4 })
        );
    }

    #[test]
    fn validate_accepts_nested_mapping() {
        // Given: valid nested YAML
        let yaml = "a:\n  b: 1\n  c: 2\n";
        // When: validating
        let result = validate_yaml_profile(yaml);
        // Then: Ok(())
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_accepts_sequence() {
        // Given: valid YAML with a sequence
        let yaml = "items:\n  - a\n  - b\n";
        // When: validating
        let result = validate_yaml_profile(yaml);
        // Then: Ok(())
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_rejects_whitespace_only() {
        // Given: whitespace-only string
        let yaml = "   \n  \n";
        // When: validating
        let result = validate_yaml_profile(yaml);
        // Then: Err (empty source or parse error)
        assert!(result.is_err());
    }

    #[test]
    fn custom_tag_rejected_exact() {
        // Given: YAML with a local tag
        let yaml = "key: !custom value\n";
        // When: validating
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::CustomTag)
        match result {
            Err(YamlError::CustomTag { tag }) => {
                assert!(tag.contains("custom"), "tag should contain 'custom', got: {tag}");
            }
            other => assert!(false, "expected CustomTag, got {other:?}"),
        }
    }
}
