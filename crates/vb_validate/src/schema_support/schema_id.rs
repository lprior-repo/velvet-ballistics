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

pub(crate) use crate::schema::is_valid_id;

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
    "together",
    "collect",
    "reduce",
    "repeat",
    "wait",
    "ask",
    "try_again",
    "on_error",
    "then",
    "finish",
];

#[cfg(test)]
#[path = "schema_id/id_tests.rs"]
mod id_tests;
