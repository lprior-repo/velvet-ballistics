//! Schema validation constants and ID grammar rules.

use crate::ValidationResult;

/// Canonical version string for v1 workflows.
pub const CANONICAL_VERSION: &str = "velvet-ballastics/v1";

/// Required top-level fields in a workflow document.
pub const REQUIRED_TOP_LEVEL_FIELDS: &[&str] = &["version", "name", "when", "steps"];

/// Allowed top-level fields in a workflow document.
pub const ALLOWED_TOP_LEVEL_FIELDS: &[&str] = &[
    "version",
    "name",
    "when",
    "inputs",
    "vars",
    "secrets",
    "result",
    "examples",
    "steps",
];

/// Allowed step-level fields.
pub const ALLOWED_STEP_FIELDS: &[&str] = &[
    "id", "name", "if", "with", "then", "set", "choose", "for_each", "together", "collect",
    "reduce", "repeat", "wait", "ask", "finish", "do", "on_error", "try_again",
];

/// Primitive step kinds.
pub const STEP_PRIMITIVES: &[&str] = &[
    "set", "do", "choose", "for_each", "together", "collect", "reduce", "repeat", "wait", "ask",
    "finish",
];

/// Reserved IDs that cannot be used as step or workflow names.
pub const RESERVED_IDS: &[&str] = &[
    "now", "random", "runtime", "null", "true", "false", "input", "inputs", "vars", "secrets",
    "steps", "error", "attempt", "total", "result", "when", "item", "do", "set", "choose",
    "for_each", "together", "collect", "reduce", "repeat", "wait", "ask", "try_again", "on_error",
    "then", "finish",
];

/// Validates that an ID conforms to the grammar rules.
///
/// Grammar: lowercase ASCII letters, digits, and underscores.
/// Must start with a lowercase letter. Max 64 characters.
pub fn is_valid_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 64 {
        return false;
    }
    let Some(&first_byte) = id.as_bytes().first() else {
        return false;
    };
    if !first_byte.is_ascii_lowercase() {
        return false;
    }
    for &byte in id.as_bytes() {
        if !byte.is_ascii_lowercase() && !byte.is_ascii_digit() && byte != b'_' {
            return false;
        }
    }
    true
}

/// Returns true if the ID is in the reserved words list.
pub fn is_reserved_id(id: &str) -> bool {
    RESERVED_IDS.contains(&id)
}

/// Validates a single ID (not in context of duplicates).
pub fn validate_id(field: &str, id: &str) -> ValidationResult<()> {
    if !is_valid_id(id) {
        return Err(crate::ValidationError::InvalidId {
            id: format!("{field}: {id}"),
        });
    }
    if is_reserved_id(id) {
        return Err(crate::ValidationError::ReservedId {
            id: format!("{field}: {id}"),
        });
    }
    Ok(())
}
