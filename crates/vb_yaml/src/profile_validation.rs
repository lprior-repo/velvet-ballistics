#![forbid(unsafe_code)]
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

use super::profile_dupkeys::reject_duplicate_mapping_keys;

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
pub(crate) fn check_source_size(text: &str, max_bytes: usize) -> YamlResult<()> {
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
pub(crate) fn collect_and_validate_events(
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
            saphyr_parser::Event::StreamStart
            | saphyr_parser::Event::StreamEnd
            | saphyr_parser::Event::DocumentEnd
            | saphyr_parser::Event::Alias(_)
            | saphyr_parser::Event::Nothing => {}
            saphyr_parser::Event::DocumentStart(_) => {
                document_count = document_count
                    .checked_add(1)
                    .ok_or(YamlError::MultipleDocuments { count: usize::MAX })?;
            }
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

        events.push(crate::events::convert_event(event, span));
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
pub(crate) fn check_scalar_length(value: &str, max_bytes: usize) -> YamlResult<()> {
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
pub(crate) fn check_null_bytes(value: &str) -> YamlResult<()> {
    if value.contains('\x00') {
        return Err(YamlError::ForbiddenFeature {
            detail: "null_byte_in_scalar",
        });
    }
    Ok(())
}

/// Check that the source text does not contain null bytes.
pub(crate) fn check_null_bytes_in_source(text: &str) -> YamlResult<()> {
    if text.contains('\x00') {
        return Err(YamlError::ForbiddenFeature {
            detail: "null_byte_in_source",
        });
    }
    Ok(())
}

/// Reject forbidden features from a pre-collected event list.
pub fn reject_forbidden_features(events: &[YamlEvent]) -> YamlResult<()> {
    for event in events {
        if let Some(tag) = event.tag()
            && !is_allowed_tag(tag)
        {
            return Err(YamlError::CustomTag { tag: tag.into() });
        }
        if let YamlEvent::Scalar { style, value, .. } = event {
            reject_binary_scalar(value, *style);
        }
    }
    Ok(())
}

/// The set of allowed YAML core schema tag suffixes.
const ALLOWED_CORE_TAG_SUFFIXES: &[&str] = &["str", "int", "float", "bool", "null", "seq", "map"];

/// Check whether a tag string is one of the allowed YAML core schema types.
pub(crate) fn is_allowed_tag(tag: &str) -> bool {
    if let Some(suffix) = tag.strip_prefix("tag:yaml.org,2002:") {
        return ALLOWED_CORE_TAG_SUFFIXES.contains(&suffix);
    }
    if let Some(suffix) = tag.strip_prefix("!!") {
        return ALLOWED_CORE_TAG_SUFFIXES.contains(&suffix);
    }
    false
}

/// Reject binary scalars (indicated by a tag like `!!binary`).
pub(crate) fn reject_binary_scalar(_value: &str, _style: crate::events::ScalarStyle) {
    // No-op: binary scalars are caught by the tag check in reject_forbidden_features.
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
pub(crate) fn is_merge_key_tag(tag: &str) -> bool {
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
pub(crate) fn check_scalar_ambiguity(events: &[YamlEvent]) -> YamlResult<()> {
    for event in events {
        if let YamlEvent::Scalar { value, style, .. } = event
            && *style == crate::events::ScalarStyle::Plain
            && is_yaml_1_1_ambiguous(value)
        {
            return Err(YamlError::AmbiguousScalar {
                scalar: value.clone(),
            });
        }
    }
    Ok(())
}
