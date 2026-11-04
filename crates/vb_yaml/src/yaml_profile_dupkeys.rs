#![forbid(unsafe_code)]
//! Duplicate key detection for YAML profile.

use crate::yaml_error::{YamlError, YamlResult};
use crate::yaml_events::{ScalarStyle, YamlEvent};

#[derive(Debug, Clone)]
enum Container {
    Mapping(MappingFrame),
    Sequence,
}

#[derive(Debug, Clone)]
struct MappingFrame {
    keys: Vec<String>,
    expecting_key: bool,
}

/// Reject duplicate keys in a list of key strings.
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

/// Identity of a mapping key: quoted scalars stay distinct from plain
/// scalars with the same text, plain integers/booleans/nulls normalize to
/// their typed value, and tags are part of the identity.
fn key_identity(value: &str, style: ScalarStyle, tag: Option<&str>) -> String {
    if let Some(tag) = tag {
        return format!("tag:{tag}:{value}");
    }
    if !matches!(style, ScalarStyle::Plain) {
        return format!("str:{value}");
    }
    if let Some(int) = parse_plain_int(value) {
        return format!("int:{int}");
    }
    match value {
        "true" | "True" | "TRUE" => return "bool:true".to_string(),
        "false" | "False" | "FALSE" => return "bool:false".to_string(),
        "~" | "null" | "Null" | "NULL" | "" => return "null:".to_string(),
        _ => {}
    }
    format!("str:{value}")
}

fn parse_plain_int(value: &str) -> Option<i64> {
    // Manual checked parse: no Result is discarded and no lossy conversion
    // hides an overflow (out-of-range keys simply keep string identity).
    let (negative, digits, radix) = if let Some(hex) = value.strip_prefix("0x") {
        (false, hex, 16_u32)
    } else if let Some(oct) = value.strip_prefix("0o") {
        (false, oct, 8_u32)
    } else if let Some(neg) = value.strip_prefix('-') {
        (true, neg, 10_u32)
    } else {
        (false, value, 10_u32)
    };
    if digits.is_empty() {
        return None;
    }
    let magnitude = digits.chars().try_fold(0_i64, |acc, c| {
        let digit = i64::from(c.to_digit(radix)?);
        acc.checked_mul(i64::from(radix))?.checked_add(digit)
    })?;
    if negative {
        magnitude.checked_neg()
    } else {
        Some(magnitude)
    }
}

/// Reject duplicate mapping keys by tracking state through the event stream.
pub fn reject_duplicate_mapping_keys(events: &[YamlEvent]) -> YamlResult<()> {
    let mut stack: Vec<Container> = Vec::new();
    for event in events {
        match event {
            YamlEvent::MappingStart { span, .. } => {
                reject_container_key(&mut stack, *span)?;
                stack.push(Container::Mapping(MappingFrame {
                    keys: Vec::new(),
                    expecting_key: true,
                }));
            }
            YamlEvent::MappingEnd { .. } => {
                pop_container(&mut stack, "mapping end without matching start")?;
            }
            YamlEvent::SequenceStart { span, .. } => {
                reject_container_key(&mut stack, *span)?;
                stack.push(Container::Sequence);
            }
            YamlEvent::SequenceEnd { .. } => {
                pop_container(&mut stack, "sequence end without matching start")?;
            }
            YamlEvent::Scalar {
                value,
                style,
                tag,
                span,
                ..
            } => {
                handle_scalar_key(value, *style, tag.as_deref(), *span, &mut stack)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn reject_container_key(
    stack: &mut [Container],
    _span: crate::yaml_events::EventSpan,
) -> YamlResult<()> {
    if let Some(Container::Mapping(frame)) = stack.last_mut() {
        if frame.expecting_key {
            return Err(YamlError::FieldShape {
                field: "mapping key",
                expected: "string",
            });
        }
        // Container in value position completes the pending pair.
        frame.expecting_key = true;
    }
    Ok(())
}

fn pop_container(stack: &mut Vec<Container>, reason: &'static str) -> YamlResult<()> {
    match stack.pop() {
        Some(_) => Ok(()),
        None => Err(YamlError::ParseError {
            line: 0,
            reason: reason.into(),
        }),
    }
}

fn handle_scalar_key(
    value: &str,
    style: ScalarStyle,
    tag: Option<&str>,
    span: crate::yaml_events::EventSpan,
    stack: &mut [Container],
) -> YamlResult<()> {
    let Some(container) = stack.last_mut() else {
        return Ok(());
    };
    match container {
        Container::Mapping(frame) if frame.expecting_key => {
            if value == "<<" && tag.is_none_or(|t| t == "tag:yaml.org,2002:merge" || t == "!!merge")
            {
                return Err(YamlError::MergeKeyForbidden {
                    line: span.line,
                    column: span.column,
                });
            }
            let identity = key_identity(value, style, tag);
            if frame.keys.contains(&identity) {
                return Err(YamlError::DuplicateKeyAt {
                    key: value.into(),
                    line: span.line,
                    column: span.column,
                });
            }
            frame.keys.push(identity);
            frame.expecting_key = false;
        }
        Container::Mapping(frame) => {
            frame.expecting_key = true;
        }
        Container::Sequence => {}
    }
    Ok(())
}
