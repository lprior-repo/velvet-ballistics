#![forbid(unsafe_code)]
//! Strict YAML profile enforcement.
//!
//! This module implements the "strict profile" that rejects YAML features
//! incompatible with the velvet-ballistics workflow definition language:
//!
//! - Anchors and aliases (no `&` or `*`)
//! - Merge keys (no `<<:`)
//! - Custom tags (no `!tag`)
//! - Binary scalars
//! - Multiple documents
//! - YAML 1.1 ambiguous boolean scalars (yes/no/on/off)
//! - Duplicate mapping keys
//! - Unbounded depth or node counts

use crate::yaml_error::{YamlError, YamlResult};
use crate::yaml_events::YamlEvent;
use crate::yaml_limits::YamlLimits;

use super::profile_dupkeys::reject_duplicate_mapping_keys;

mod event_collect;
pub(crate) use event_collect::collect_and_validate_events;

/// Validate that the given YAML text conforms to the strict profile.
///
/// This runs the full set of checks: size limits, event profile, depth
/// bounds, and feature rejection.
pub fn validate_yaml_profile(text: &str) -> YamlResult<()> {
    let limits = YamlLimits::default();
    validate_yaml_profile_with_limits(text, &limits)
}

/// Strip one leading UTF-8 byte order mark, if present.
pub(crate) fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// Reject UTF-16-looking sources: a UTF-16 BOM decoded as Latin, or ASCII
/// text interleaved with NUL bytes.
pub(crate) fn check_encoding(text: &str) -> YamlResult<()> {
    if text.starts_with('\u{fffd}')
        || text.as_bytes().starts_with(b"\xff\xfe")
        || text.as_bytes().starts_with(b"\xfe\xff")
    {
        return Err(YamlError::UnsupportedEncoding);
    }
    if text.contains('\x00') {
        let nul = text.bytes().filter(|b| *b == 0).count();
        // UTF-16LE/BE ASCII has a NUL in roughly every second byte.
        if nul.saturating_mul(4) >= text.len() && nul >= 2 {
            return Err(YamlError::UnsupportedEncoding);
        }
    }
    Ok(())
}

/// Reject YAML directives (`%YAML`, `%TAG`, any `%...`) ahead of the first
/// document content; the strict profile defines no directive semantics.
pub(crate) fn reject_directives(text: &str) -> YamlResult<()> {
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('%') {
            return Err(YamlError::ForbiddenFeature {
                detail: "yaml_directive",
            });
        }
        break;
    }
    Ok(())
}

/// Validate the strict profile before allocating an AST, using caller limits.
pub fn validate_yaml_profile_with_limits(text: &str, limits: &YamlLimits) -> YamlResult<()> {
    check_source_size(text, limits.max_source_bytes)?;
    let text = strip_bom(text);
    check_encoding(text)?;
    reject_directives(text)?;
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
pub(crate) fn reject_binary_scalar(_value: &str, _style: crate::yaml_events::ScalarStyle) {
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
            && *style == crate::yaml_events::ScalarStyle::Plain
            && is_yaml_1_1_ambiguous(value)
        {
            return Err(YamlError::AmbiguousScalar {
                scalar: value.clone(),
            });
        }
    }
    Ok(())
}
