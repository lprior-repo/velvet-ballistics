#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::panic)]
//! Property tests for SymbolicCode serde round-trip.
//!
//! Compensates: BLOCKED PO-009 H1 (serde_roundtrip).
//! Invariant: For any registered SymbolicCode, serialize → deserialize preserves identity.
//! For arbitrary unregistered strings, deserialize returns Err.
//! For malformed JSON, deserialize returns Err.

use vb_core::diagnostic::{CODE_REGISTRY, SymbolicCode};

// ---------------------------------------------------------------------------
// Round-trip property tests
// ---------------------------------------------------------------------------

#[test]
fn serde_round_trip_preserves_code_for_all_registered_strings() {
    for entry in CODE_REGISTRY {
        // Skip duplicate symbolic names (cross-category duplicates return
        // the first match's SymbolicCode, so round-trip may not be identity
        // for later entries with the same symbolic name).
        let code = SymbolicCode::from_static(entry.symbolic).expect("registered");
        let json = serde_json::to_string(&code).expect("serialize should succeed");
        let expected_json = format!("\"{}\"", entry.symbolic);
        assert_eq!(
            json, expected_json,
            "serialized JSON for '{}' must be '\"{}\"'",
            entry.symbolic, entry.symbolic
        );
        let deserialized: SymbolicCode =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(
            deserialized, code,
            "serde round-trip must preserve identity for '{}'",
            entry.symbolic
        );
        assert_eq!(deserialized.as_str(), entry.symbolic);
    }
}

#[test]
fn serialize_produces_json_string_of_symbolic_name() {
    // Spot-check a few entries.
    for name in &["DUPLICATE_KEY", "TYPE_MISMATCH", "LIMIT_EXCEEDED"] {
        let code = SymbolicCode::from_static(name).expect("registered");
        let json = serde_json::to_string(&code).expect("serialize");
        assert_eq!(
            json,
            format!("\"{name}\""),
            "serialize for {name} must produce JSON string \"{name}\""
        );
    }
}

// ---------------------------------------------------------------------------
// Rejection property tests
// ---------------------------------------------------------------------------

#[test]
fn deserialize_rejects_unregistered_code_name() {
    let result: Result<SymbolicCode, _> = serde_json::from_str("\"BOGUS_NOT_A_CODE\"");
    assert!(
        matches!(result, Err(_)),
        "deserialize should reject unknown code name"
    );
}

#[test]
fn deserialize_rejects_non_string_json_types() {
    for input in &["123", "null", "[]", "{}", "true", "false"] {
        let result: Result<SymbolicCode, _> = serde_json::from_str(input);
        assert!(
            matches!(result, Err(_)),
            "deserialize should reject non-string JSON: {input}"
        );
    }
}

#[test]
fn deserialize_rejects_empty_json_string() {
    let result: Result<SymbolicCode, _> = serde_json::from_str("\"\"");
    assert!(
        matches!(result, Err(_)),
        "deserialize should reject empty string"
    );
}

#[test]
fn deserialize_rejects_wrong_case_registered_name() {
    let result: Result<SymbolicCode, _> = serde_json::from_str("\"duplicate_key\"");
    assert!(
        matches!(result, Err(_)),
        "deserialize should reject lowercase variant"
    );
}

#[test]
fn deserialize_rejects_number_instead_of_string() {
    let result: Result<SymbolicCode, _> = serde_json::from_str("42");
    assert!(matches!(result, Err(_)), "deserialize should reject number");
}

#[test]
fn deserialize_rejects_null() {
    let result: Result<SymbolicCode, _> = serde_json::from_str("null");
    assert!(matches!(result, Err(_)), "deserialize should reject null");
}

#[test]
fn deserialize_rejects_empty_object() {
    let result: Result<SymbolicCode, _> = serde_json::from_str("{}");
    assert!(matches!(result, Err(_)), "deserialize should reject object");
}

#[test]
fn deserialize_rejects_empty_array() {
    let result: Result<SymbolicCode, _> = serde_json::from_str("[]");
    assert!(matches!(result, Err(_)), "deserialize should reject array");
}
