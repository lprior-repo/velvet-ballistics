#![forbid(unsafe_code)]
//! Duplicate key detection for YAML profile.

use crate::yaml_error::{YamlError, YamlResult};
use crate::yaml_events::YamlEvent;

#[derive(Debug, Clone)]
enum Container<'a> {
    Mapping(MappingFrame<'a>),
    Sequence,
}

#[derive(Debug, Clone)]
struct MappingFrame<'a> {
    keys: Vec<&'a str>,
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

/// Reject duplicate mapping keys by tracking state through the event stream.
pub fn reject_duplicate_mapping_keys(events: &[YamlEvent]) -> YamlResult<()> {
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
