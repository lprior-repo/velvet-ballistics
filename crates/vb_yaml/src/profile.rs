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
    check_null_bytes_in_source(text)?;
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
                check_null_bytes(value)?;
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

/// Check that a scalar value does not contain null bytes.
///
/// Null bytes (\x00) are not valid in YAML 1.2 scalar content and can cause
/// issues in downstream processing (C string termination, protocol injection).
fn check_null_bytes(value: &str) -> YamlResult<()> {
    if value.contains('\x00') {
        return Err(YamlError::ForbiddenFeature {
            detail: "null_byte_in_scalar",
        });
    }
    Ok(())
}

/// Check that the source text does not contain null bytes.
///
/// Null bytes may be silently stripped or reinterpreted by the parser, so
/// we check the raw source text before parsing begins. This prevents null
/// bytes from reaching any downstream consumer via any path.
fn check_null_bytes_in_source(text: &str) -> YamlResult<()> {
    if text.contains('\x00') {
        return Err(YamlError::ForbiddenFeature {
            detail: "null_byte_in_source",
        });
    }
    Ok(())
}

/// Reject forbidden features from a pre-collected event list.
///
/// Checks for custom tags and binary scalars. Only the seven YAML core schema
/// types are allowed: str, int, float, bool, null, seq, map. Any other `!!`
/// tag (e.g. `!!timestamp`, `!!binary`, `!!set`, `!!omap`) is rejected.
pub fn reject_forbidden_features(events: &[YamlEvent]) -> YamlResult<()> {
    for event in events {
        if let Some(tag) = event.tag()
            && !is_allowed_tag(tag)
        {
            return Err(YamlError::CustomTag { tag: tag.into() });
        }
        if let YamlEvent::Scalar { style, value, .. } = event {
            reject_binary_scalar(value, *style)?;
        }
    }
    Ok(())
}

/// The set of allowed YAML core schema tag suffixes.
const ALLOWED_CORE_TAG_SUFFIXES: &[&str] = &["str", "int", "float", "bool", "null", "seq", "map"];

/// Check whether a tag string is one of the allowed YAML core schema types.
///
/// Accepts both shorthand forms (`!!str`, `!!int`, ...) and full URI forms
/// (`tag:yaml.org,2002:str`, ...). Everything else is rejected.
fn is_allowed_tag(tag: &str) -> bool {
    // Full URI form: tag:yaml.org,2002:<suffix>
    if let Some(suffix) = tag.strip_prefix("tag:yaml.org,2002:") {
        return ALLOWED_CORE_TAG_SUFFIXES.contains(&suffix);
    }
    // Shorthand form: !!<suffix>
    if let Some(suffix) = tag.strip_prefix("!!") {
        return ALLOWED_CORE_TAG_SUFFIXES.contains(&suffix);
    }
    // Tags without !! or tag:yaml.org,2002: prefix are handled by the
    // caller — they are custom tags and should be rejected.
    false
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
                pop_container(&mut stack, "mapping end without matching start")?;
            }
            YamlEvent::SequenceStart { .. } => {
                finish_mapping_value_if_needed(&mut stack);
                stack.push(Container::Sequence);
            }
            YamlEvent::SequenceEnd { .. } => {
                pop_container(&mut stack, "sequence end without matching start")?;
            }
            YamlEvent::Scalar { value, .. } => {
                handle_scalar_for_duplicate_key(value, &mut stack)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn pop_container(stack: &mut Vec<Container<'_>>, reason: &'static str) -> YamlResult<()> {
    match stack.pop() {
        Some(_) => Ok(()),
        None => Err(YamlError::ParseError {
            line: 0,
            reason: reason.into(),
        }),
    }
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

    fn assertion_failed(_message: std::fmt::Arguments<'_>) -> bool {
        false
    }

    macro_rules! fail_assert {
        ($($arg:tt)*) => {
            assert!(assertion_failed(format_args!($($arg)*)), $($arg)*)
        };
    }

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
            Err(YamlError::AmbiguousScalar {
                scalar: "yes".into()
            })
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
            Err(YamlError::AmbiguousScalar {
                scalar: "no".into()
            })
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
            Err(YamlError::AmbiguousScalar {
                scalar: "on".into()
            })
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
            Err(YamlError::AmbiguousScalar {
                scalar: "off".into()
            })
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
            other => fail_assert!("expected NestingTooDeep, got {other:?}"),
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
            other => fail_assert!("expected ScalarTooLong, got {other:?}"),
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
            other => fail_assert!("expected NodeLimitExceeded, got {other:?}"),
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
            fail_assert!("collect_events failed");
            return;
        };
        // When: rejecting forbidden features
        let result = reject_forbidden_features(&events);
        // Then: Err(YamlError::CustomTag)
        match result {
            Err(YamlError::CustomTag { tag }) => {
                assert!(tag.contains("mytag"), "expected 'mytag' in tag, got: {tag}");
            }
            other => fail_assert!("expected CustomTag, got {other:?}"),
        }
    }

    #[test]
    fn reject_forbidden_features_allows_core_tags() {
        // Given: events from YAML with only core schema tags (no custom)
        let yaml = "key: value\n";
        let Ok(events) = crate::events::collect_events(yaml) else {
            fail_assert!("collect_events failed");
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
            fail_assert!("collect_events failed");
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
            fail_assert!("collect_events failed");
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
            fail_assert!("collect_events failed");
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
            fail_assert!("collect_events failed");
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
            Err(YamlError::AmbiguousScalar {
                scalar: "yes".into()
            })
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
        assert_eq!(result, Err(YamlError::DuplicateKey { key: "name".into() }));
    }

    #[test]
    fn duplicate_nested_key_exact() {
        // Given: YAML with duplicate nested "name" key
        let yaml = "version: velvet-ballastics/v1\nname: wf\nwhen:\n  ipc:\n    name: a\n    name: b\nsteps: []\n";
        // When: validating
        let result = validate_yaml_profile(yaml);
        // Then: Err with exact key
        assert_eq!(result, Err(YamlError::DuplicateKey { key: "name".into() }));
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
        assert_eq!(result, Err(YamlError::SourceTooLarge { size: 5, max: 4 }));
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
                assert!(
                    tag.contains("custom"),
                    "tag should contain 'custom', got: {tag}"
                );
            }
            other => fail_assert!("expected CustomTag, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Adversarial BDD tests - attack vector validation
    // -----------------------------------------------------------------------

    #[test]
    fn adversarial_duplicate_keys_nested_deep_mapping_rejected() {
        // Given: YAML with duplicate keys in a deeply nested mapping
        let yaml = "a:\n  b:\n    c: 1\n    c: 2\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::DuplicateKey { key: "c" })
        assert_eq!(result, Err(YamlError::DuplicateKey { key: "c".into() }));
    }

    #[test]
    fn adversarial_duplicate_keys_top_level_rejected() {
        // Given: YAML with duplicate top-level keys
        let yaml = "x: 1\nx: 2\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::DuplicateKey { key: "x" })
        assert_eq!(result, Err(YamlError::DuplicateKey { key: "x".into() }));
    }

    #[test]
    fn adversarial_alias_without_anchor_rejected() {
        // Given: YAML with an alias reference (saphyr may reject this at parse,
        // but we verify our rejection layer also catches it)
        let yaml = "a: &anc value\nb: *anc\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::AnchorAliasMerge)
        assert_eq!(result, Err(YamlError::AnchorAliasMerge));
    }

    #[test]
    fn adversarial_anchor_on_sequence_rejected() {
        // Given: YAML with anchor on a sequence node
        let yaml = "items: &seq\n  - a\n  - b\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::AnchorAliasMerge)
        assert_eq!(result, Err(YamlError::AnchorAliasMerge));
    }

    #[test]
    fn adversarial_anchor_on_mapping_rejected() {
        // Given: YAML with anchor on a mapping node
        let yaml = "base: &map\n  k: v\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::AnchorAliasMerge)
        assert_eq!(result, Err(YamlError::AnchorAliasMerge));
    }

    #[test]
    fn adversarial_custom_tag_double_bang_timestamp_rejected() {
        // Given: YAML with !!timestamp tag (not a core schema type)
        let yaml = "date: !!timestamp 2024-01-01\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::CustomTag) - only the seven core schema types
        // (str, int, float, bool, null, seq, map) are allowed.
        match result {
            Err(YamlError::CustomTag { tag }) => {
                assert!(
                    tag.contains("timestamp"),
                    "expected 'timestamp' in tag, got: {tag}"
                );
            }
            other => fail_assert!("expected CustomTag, got {other:?}"),
        }
    }

    #[test]
    fn adversarial_custom_tag_local_bang_rejected() {
        // Given: YAML with a local tag !myapp/special
        let yaml = "val: !myapp/special data\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::CustomTag)
        match result {
            Err(YamlError::CustomTag { tag }) => {
                assert!(tag.contains("myapp"), "expected 'myapp' in tag, got: {tag}");
            }
            other => fail_assert!("expected CustomTag, got {other:?}"),
        }
    }

    #[test]
    fn adversarial_multi_document_with_explicit_markers_rejected() {
        // Given: YAML with explicit --- separators for two documents
        let yaml = "---\na: 1\n...\n---\nb: 2\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::MultipleDocuments)
        assert!(
            matches!(result, Err(YamlError::MultipleDocuments { count }) if count >= 2),
            "expected MultipleDocuments, got: {result:?}"
        );
    }

    #[test]
    fn adversarial_yaml_11_yes_mixed_case_rejected() {
        // Given: YAML with mixed-case "Yes" (YAML 1.1 boolean)
        let yaml = "flag: Yes\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::AmbiguousScalar { scalar: "Yes" })
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar {
                scalar: "Yes".into()
            })
        );
    }

    #[test]
    fn adversarial_yaml_11_no_uppercase_rejected() {
        // Given: YAML with uppercase "NO"
        let yaml = "flag: NO\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::AmbiguousScalar { scalar: "NO" })
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar {
                scalar: "NO".into()
            })
        );
    }

    #[test]
    fn adversarial_yaml_11_on_uppercase_rejected() {
        // Given: YAML with uppercase "ON"
        let yaml = "flag: ON\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::AmbiguousScalar { scalar: "ON" })
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar {
                scalar: "ON".into()
            })
        );
    }

    #[test]
    fn adversarial_yaml_11_off_mixed_case_rejected() {
        // Given: YAML with mixed-case "Off"
        let yaml = "flag: Off\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::AmbiguousScalar { scalar: "Off" })
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar {
                scalar: "Off".into()
            })
        );
    }

    #[test]
    fn adversarial_yaml_11_y_lowercase_rejected() {
        // Given: YAML with single-letter "y" (YAML 1.1 boolean)
        let yaml = "flag: y\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::AmbiguousScalar)
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar { scalar: "y".into() })
        );
    }

    #[test]
    fn adversarial_yaml_11_n_lowercase_rejected() {
        // Given: YAML with single-letter "n"
        let yaml = "flag: n\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::AmbiguousScalar)
        assert_eq!(
            result,
            Err(YamlError::AmbiguousScalar { scalar: "n".into() })
        );
    }

    #[test]
    fn adversarial_yaml_11_boolean_quoted_accepted() {
        // Given: YAML with quoted "yes" (not ambiguous when quoted)
        let yaml = "flag: 'yes'\nother: \"no\"\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Ok(()) - quoted values are not ambiguous
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn adversarial_comments_only_rejected_as_empty() {
        // Given: YAML with only comments, no content
        let yaml = "# just a comment\n# another comment\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err - no actual content
        assert!(result.is_err(), "expected error for comments-only YAML");
    }

    #[test]
    fn adversarial_empty_string_rejected() {
        // Given: empty string
        // When: validating profile
        let result = validate_yaml_profile("");
        // Then: Err(YamlError::EmptySource)
        assert_eq!(result, Err(YamlError::EmptySource));
    }

    #[test]
    fn adversarial_scalar_over_limit_rejected() {
        // Given: YAML with a scalar exceeding the 65KB default limit
        let long_val = "x".repeat(70_000);
        let yaml = format!("key: \"{long_val}\"\n");
        // When: validating profile with default limits
        let result = validate_yaml_profile(&yaml);
        // Then: Err(YamlError::ScalarTooLong)
        match result {
            Err(YamlError::ScalarTooLong { len, max }) => {
                assert!(len > 65_536, "expected len > 65536, got {len}");
                assert_eq!(max, 65_536);
            }
            other => fail_assert!("expected ScalarTooLong, got {other:?}"),
        }
    }

    #[test]
    fn adversarial_node_limit_exceeded() {
        // Given: YAML with many nodes exceeding default limit
        let mut yaml = String::from("root:\n");
        for i in 0..5_000 {
            yaml.push_str(&format!("  k{i}: v{i}\n"));
        }
        let limits = YamlLimits {
            max_nodes: 100,
            ..YamlLimits::default()
        };
        // When: validating with low node limit
        let result = validate_yaml_profile_with_limits(&yaml, &limits);
        // Then: Err(YamlError::NodeLimitExceeded)
        match result {
            Err(YamlError::NodeLimitExceeded { count, max }) => {
                assert!(count > 100);
                assert_eq!(max, 100);
            }
            other => fail_assert!("expected NodeLimitExceeded, got {other:?}"),
        }
    }

    #[test]
    fn adversarial_duplicate_key_in_sequence_context_rejected() {
        // Given: YAML with duplicate keys inside a mapping within a sequence
        let yaml = "items:\n  - name: a\n    name: b\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::DuplicateKey { key: "name" })
        assert_eq!(result, Err(YamlError::DuplicateKey { key: "name".into() }));
    }

    #[test]
    fn adversarial_depth_limit_exact_boundary_accepted() {
        // Given: YAML nested exactly to the depth limit
        let mut yaml = String::from("a:\n");
        for i in 0..9 {
            let indent = "  ".repeat(i);
            yaml.push_str(&format!("{indent}b:\n"));
        }
        let limits = YamlLimits {
            max_depth: 10,
            ..YamlLimits::default()
        };
        // When: validating
        let result = validate_yaml_profile_with_limits(&yaml, &limits);
        // Then: Ok(()) - exactly at the limit is fine
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn adversarial_depth_limit_one_over_rejected() {
        // Given: YAML nested one level deeper than the limit
        // Each nested mapping increases indentation by 2 spaces
        let mut yaml = String::new();
        for i in 0..11 {
            let indent = "  ".repeat(i);
            yaml.push_str(&format!("{indent}a:\n"));
        }
        let limits = YamlLimits {
            max_depth: 10,
            ..YamlLimits::default()
        };
        // When: validating
        let result = validate_yaml_profile_with_limits(&yaml, &limits);
        // Then: Err(YamlError::NestingTooDeep) - 11 levels > max 10
        assert!(
            matches!(result, Err(YamlError::NestingTooDeep { depth, max }) if depth > 10 && max == 10),
            "expected NestingTooDeep, got: {result:?}"
        );
    }

    #[test]
    fn adversarial_tag_on_sequence_rejected() {
        // Given: YAML with a custom tag on a sequence
        let yaml = "items: !seq\n  - a\n  - b\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::CustomTag)
        match result {
            Err(YamlError::CustomTag { tag }) => {
                assert!(tag.contains("seq"), "expected 'seq' in tag, got: {tag}");
            }
            other => fail_assert!("expected CustomTag, got {other:?}"),
        }
    }

    #[test]
    fn adversarial_tag_on_mapping_rejected() {
        // Given: YAML with a custom tag on a mapping
        let yaml = "data: !map\n  k: v\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::CustomTag)
        match result {
            Err(YamlError::CustomTag { tag }) => {
                assert!(tag.contains("map"), "expected 'map' in tag, got: {tag}");
            }
            other => fail_assert!("expected CustomTag, got {other:?}"),
        }
    }

    #[test]
    fn adversarial_source_exactly_at_size_limit_accepted() {
        // Given: YAML source exactly at the size limit
        let base = "a: b\n"; // 6 bytes
        let max = base.len();
        // When: validating
        let result = check_source_size(base, max);
        // Then: Ok(())
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn adversarial_source_one_byte_over_limit_rejected() {
        // Given: YAML source one byte over the limit
        let text = "a: bcd\n"; // 7 bytes
        // When: checking size with max=6
        let result = check_source_size(text, 6);
        // Then: Err(YamlError::SourceTooLarge { size: 7, max: 6 })
        assert_eq!(result, Err(YamlError::SourceTooLarge { size: 7, max: 6 }));
    }

    #[test]
    fn adversarial_three_documents_rejected_with_count() {
        // Given: YAML with three document separators
        let yaml = "---\na: 1\n---\nb: 2\n---\nc: 3\n";
        // When: validating profile
        let result = validate_yaml_profile(yaml);
        // Then: Err(YamlError::MultipleDocuments { count: 3 })
        assert_eq!(result, Err(YamlError::MultipleDocuments { count: 3 }));
    }
}
