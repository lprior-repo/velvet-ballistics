//! Pure validation checks for closure evidence fields.

use chrono::{DateTime, Utc};

pub(crate) fn is_valid_sha(s: &str) -> bool {
    if s.len() < 7 || s.len() > 64 {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

pub(crate) fn is_valid_timestamp(s: &str) -> bool {
    if !s.ends_with('Z') {
        return false;
    }
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
        .is_some()
}

pub(crate) fn is_integer_value(v: &serde_json::Value) -> bool {
    v.is_i64() || v.is_u64()
}

pub(crate) fn exit_code_as_i64(data: &serde_json::Map<String, serde_json::Value>) -> Option<i64> {
    match data.get("exit_code") {
        Some(v) if v.is_i64() => v.as_i64(),
        Some(v) if v.is_u64() => v.as_u64().and_then(|u| i64::try_from(u).ok()),
        _ => None,
    }
}

pub(crate) fn exit_code_is_zero(data: &serde_json::Map<String, serde_json::Value>) -> bool {
    exit_code_as_i64(data) == Some(0)
}

pub(crate) fn is_value_missing(v: &serde_json::Value) -> bool {
    v.is_null() || (v.is_string() && v.as_str().is_some_and(|s| s.trim().is_empty()))
}

pub(crate) fn string_field(
    data: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Option<String> {
    data.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
#[path = "checks_tests.rs"]
mod checks_tests;
