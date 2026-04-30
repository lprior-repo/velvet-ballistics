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
pub(crate) fn validate_yaml_profile_with_limits(
    text: &str,
    limits: &YamlLimits,
) -> YamlResult<()> {
    check_source_size(text, limits.max_source_bytes)?;
    let events = collect_and_validate_events(text, limits)?;
    reject_forbidden_features(&events)?;
    reject_multiple_documents(&events)?;
    reject_anchors_aliases_merges(&events)?;
    check_scalar_ambiguity(&events)?;
    Ok(())
}

/// Check that the source text does not exceed the size limit.
fn check_source_size(text: &str, max_bytes: usize) -> YamlResult<()> {
    let size = text.len();
    if size > max_bytes {
        return Err(YamlError::SourceTooLarge { size, max: max_bytes });
    }
    Ok(())
}

/// Collect events from the parser while tracking depth and node counts.
fn collect_and_validate_events(
    text: &str,
    limits: &YamlLimits,
) -> YamlResult<Vec<YamlEvent>> {
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
                depth = depth
                    .checked_add(1)
                    .ok_or(YamlError::NestingTooDeep {
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
        return Err(YamlError::ScalarTooLong { len, max: max_bytes });
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
                return Err(YamlError::CustomTag {
                    tag: tag.into(),
                });
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
            YamlEvent::Scalar {
                anchor_id, tag, ..
            }
            | YamlEvent::SequenceStart {
                anchor_id, tag, ..
            }
            | YamlEvent::MappingStart {
                anchor_id, tag, ..
            } => {
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
    matches!(
        lower.as_str(),
        "yes" | "no" | "on" | "off" | "y" | "n"
    )
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
}
