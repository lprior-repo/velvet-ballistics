//! Extract list max values from declared input schemas.

use crate::AuthorValue;

/// Extract the list max value from a declared input schema.
///
/// Returns None for non-list schemas. Returns Some(max) for list schemas,
/// including those without an explicit max (defaults to u32::MAX).
pub(super) fn list_max_from_schema(value: &AuthorValue) -> Option<u32> {
    match value {
        AuthorValue::Text(s) => {
            if s.starts_with("list<") {
                Some(u32::MAX)
            } else {
                None
            }
        }
        AuthorValue::Mapping(entries) => {
            let is_list = entries.iter().any(|e| e.key == "is" && matches!(&e.value, AuthorValue::Text(t) if t == "list"));
            if !is_list {
                return None;
            }
            // Look for explicit max
            for entry in entries {
                if entry.key == "max" {
                    if let AuthorValue::I64(max) = &entry.value {
                        return Some(u32::try_from(*max).unwrap_or(u32::MAX));
                    }
                }
            }
            // No explicit max
            Some(u32::MAX)
        }
        _ => None,
    }
}

/// Check if a list input has a finite (bounded) max value.
pub(super) fn has_finite_list_max(value: &AuthorValue) -> bool {
    list_max_from_schema(value).map(|m| m != u32::MAX).unwrap_or(false)
}
