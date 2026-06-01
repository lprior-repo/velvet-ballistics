#![forbid(unsafe_code)]
//! ID validation for schema validation.

#![allow(unreachable_pub)]
use crate::{ValidationError, ValidationResult};

pub fn validate_single_id(id: &str, seen: &[&str]) -> ValidationResult<()> {
    if !is_valid_id(id) {
        return Err(ValidationError::InvalidId { id: id.to_owned() });
    }
    if is_reserved_id(id) {
        return Err(ValidationError::ReservedId { id: id.to_owned() });
    }
    if seen.contains(&id) {
        return Err(ValidationError::DuplicateId { id: id.to_owned() });
    }
    Ok(())
}

pub fn is_valid_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 64 {
        return false;
    }
    let Some(&byte) = id.as_bytes().first() else {
        return false;
    };
    if !byte.is_ascii_lowercase() {
        return false;
    }
    for byte in id.as_bytes() {
        if !byte.is_ascii_lowercase() && !byte.is_ascii_digit() && *byte != b'_' {
            return false;
        }
    }
    true
}

pub fn is_reserved_id(id: &str) -> bool {
    RESERVED_IDS.contains(&id)
}

const RESERVED_IDS: &[&str] = &[
    "now",
    "random",
    "runtime",
    "null",
    "true",
    "false",
    "input",
    "inputs",
    "vars",
    "secrets",
    "steps",
    "error",
    "attempt",
    "total",
    "result",
    "when",
    "item",
    "do",
    "set",
    "choose",
    "for_each",
    "parallel",
    "collect",
    "aggregate",
    "repeat",
    "wait",
    "ask",
    "try_again",
    "on_error",
    "then",
    "finish",
];

mod tests;
