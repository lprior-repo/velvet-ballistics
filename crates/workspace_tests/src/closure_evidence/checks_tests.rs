//! Pure unit tests for closure evidence checks.

use super::*;

#[test]
fn valid_whole_second_timestamp_passes() {
    assert!(is_valid_timestamp("2026-01-01T00:00:00Z"));
}

#[test]
fn valid_fractional_second_timestamp_passes() {
    assert!(is_valid_timestamp("2026-01-01T00:00:00.123456Z"));
}

#[test]
fn non_utc_offset_timestamp_fails() {
    assert!(!is_valid_timestamp("2026-01-01T00:00:00+00:00"));
}

#[test]
fn invalid_calendar_timestamp_fails() {
    assert!(!is_valid_timestamp("2026-02-30T00:00:00Z"));
}

#[test]
fn valid_leap_day_timestamp_passes() {
    assert!(is_valid_timestamp("2024-02-29T00:00:00Z"));
}

#[test]
fn exit_code_zero_is_zero() {
    let mut data = serde_json::Map::new();
    data.insert("exit_code".to_string(), serde_json::json!(0));
    assert!(exit_code_is_zero(&data));
}

#[test]
fn exit_code_negative_is_nonzero() {
    let mut data = serde_json::Map::new();
    data.insert("exit_code".to_string(), serde_json::json!(-1));
    assert!(!exit_code_is_zero(&data));
}

#[test]
fn exit_code_u64_max_wraps() {
    let mut data = serde_json::Map::new();
    data.insert("exit_code".to_string(), serde_json::json!(u64::MAX));
    assert!(!exit_code_is_zero(&data));
}

#[test]
fn exit_code_bool_rejected() {
    let mut data = serde_json::Map::new();
    data.insert("exit_code".to_string(), serde_json::json!(true));
    assert!(!exit_code_is_zero(&data));
}

#[test]
fn exit_code_string_rejected() {
    let mut data = serde_json::Map::new();
    data.insert("exit_code".to_string(), serde_json::json!("0"));
    assert!(!exit_code_is_zero(&data));
}

#[test]
fn exit_code_as_i64_returns_zero() {
    let mut data = serde_json::Map::new();
    data.insert("exit_code".to_string(), serde_json::json!(0));
    assert_eq!(exit_code_as_i64(&data), Some(0));
}

#[test]
fn exit_code_as_i64_returns_negative() {
    let mut data = serde_json::Map::new();
    data.insert("exit_code".to_string(), serde_json::json!(-42));
    assert_eq!(exit_code_as_i64(&data), Some(-42));
}

#[test]
fn exit_code_as_i64_returns_large_u64() {
    let mut data = serde_json::Map::new();
    data.insert("exit_code".to_string(), serde_json::json!(u64::MAX));
    assert!(exit_code_as_i64(&data).is_none());
}

#[test]
fn is_integer_value_accepts_i64() {
    assert!(is_integer_value(&serde_json::json!(0)));
}

#[test]
fn is_integer_value_accepts_u64() {
    assert!(is_integer_value(&serde_json::json!(42)));
}

#[test]
fn is_integer_value_rejects_string() {
    assert!(!is_integer_value(&serde_json::json!("0")));
}

#[test]
fn is_integer_value_rejects_bool() {
    assert!(!is_integer_value(&serde_json::json!(true)));
}

#[test]
fn is_value_missing_null() {
    assert!(is_value_missing(&serde_json::Value::Null));
}

#[test]
fn is_value_missing_empty_string() {
    assert!(is_value_missing(&serde_json::json!("")));
}

#[test]
fn is_value_missing_whitespace_string() {
    assert!(is_value_missing(&serde_json::json!("  ")));
}

#[test]
fn is_value_missing_non_empty_string() {
    assert!(!is_value_missing(&serde_json::json!("hello")));
}

#[test]
fn is_value_missing_non_string() {
    assert!(!is_value_missing(&serde_json::json!(42)));
}
