#![forbid(unsafe_code)]
//! Checked JSON value access helpers for the CLI.
//!
//! Per bead vb-hwkqa, CLI JSON parsing MUST NOT use unchecked indexing
//! (`value["key"]`) because a missing key panics. Every JSON access in
//! the CLI must go through `Map::get(...).and_then(...)` so a missing
//! key surfaces as a typed `Option` and is handled by the caller.
//!
//! This module exposes `value_get` and `value_get_str` for the common
//! patterns so future code does not regress to indexing.

use serde_json::Value;

/// Look up a key in a JSON object, returning `None` if the value is missing
/// or the root is not an object.
#[must_use]
pub fn value_get<'a>(root: &'a Value, key: &str) -> Option<&'a Value> {
    root.as_object().and_then(|map| map.get(key))
}

/// Look up a key in a JSON object and try to extract its string value.
#[must_use]
pub fn value_get_str<'a>(root: &'a Value, key: &str) -> Option<&'a str> {
    value_get(root, key).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn value_get_returns_some_for_present_key() {
        let root = json!({ "name": "alpha", "count": 7 });
        assert_eq!(value_get_str(&root, "name"), Some("alpha"));
        assert!(value_get(&root, "count").is_some());
    }

    #[test]
    fn value_get_returns_none_for_missing_key() {
        let root = json!({ "name": "alpha" });
        assert_eq!(value_get_str(&root, "missing"), None);
        assert!(value_get(&root, "missing").is_none());
    }

    #[test]
    fn value_get_returns_none_for_non_object_root() {
        let root = json!([1, 2, 3]);
        assert!(value_get(&root, "anything").is_none());
        let scalar = json!("plain");
        assert!(value_get_str(&scalar, "x").is_none());
    }
}
