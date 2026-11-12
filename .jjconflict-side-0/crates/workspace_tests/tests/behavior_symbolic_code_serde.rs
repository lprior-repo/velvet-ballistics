//! Behavior tests: SymbolicCode serde integration.
//!
//! B-011, B-012, B-013: SymbolicCode Serialize/Deserialize
//! - Serialize produces a JSON string of the symbolic name
//! - Deserialize accepts registered symbolic names and rejects unknown names
//! - Deserialize rejects malformed JSON

use vb_core::diagnostic::{CODE_REGISTRY, SymbolicCode};

// ---------------------------------------------------------------------------
// B-011: Serialize
// ---------------------------------------------------------------------------

#[test]
fn symbolic_code_serialize_produces_json_string_of_symbolic_name() {
    for entry in CODE_REGISTRY.iter().take(30) {
        let code = SymbolicCode::from_static(entry.symbolic).expect("registered");
        let json = serde_json::to_string(&code).expect("serialize should succeed");
        let expected = format!("\"{}\"", entry.symbolic);
        assert_eq!(
            json, expected,
            "Serialize for '{}' must produce JSON string",
            entry.symbolic
        );
    }
}

#[test]
fn symbolic_code_serialize_never_outputs_numeric_format() {
    let code = SymbolicCode::from_static("DUPLICATE_KEY").expect("registered");
    let json = serde_json::to_string(&code).expect("serialize");
    let num = code
        .numeric_code()
        .expect("DUPLICATE_KEY must be registered");
    let numeric_fmt = format!("\"E{:04X}\"", num);
    assert_ne!(
        json, numeric_fmt,
        "Serialize must NOT produce numeric format '{}'",
        numeric_fmt
    );
}

// ---------------------------------------------------------------------------
// B-012: Deserialize — accepts registered names
// ---------------------------------------------------------------------------

#[test]
fn symbolic_code_deserialize_accepts_registered_name() {
    let result: SymbolicCode =
        serde_json::from_str("\"DUPLICATE_KEY\"").expect("should deserialize registered name");
    assert_eq!(result.as_str(), "DUPLICATE_KEY");
}

#[test]
fn symbolic_code_deserialize_accepts_all_registered_names_sample() {
    for entry in CODE_REGISTRY.iter().take(50) {
        let json = format!("\"{}\"", entry.symbolic);
        let result: SymbolicCode = serde_json::from_str(&json).expect(&format!(
            "should deserialize registered name '{}'",
            entry.symbolic
        ));
        assert_eq!(result.as_str(), entry.symbolic);
    }
}

#[test]
fn symbolic_code_serde_round_trip_all_registered_names() {
    for entry in CODE_REGISTRY {
        let code = SymbolicCode::from_static(entry.symbolic).expect("registered");
        let json = serde_json::to_string(&code).unwrap();
        let deserialized: SymbolicCode =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(
            deserialized, code,
            "serde round-trip must preserve identity for '{}'",
            entry.symbolic
        );
    }
}

// ---------------------------------------------------------------------------
// B-013: Deserialize — rejects unknown/malformed
// ---------------------------------------------------------------------------

#[test]
fn symbolic_code_deserialize_rejects_unknown_code_name() {
    let result: Result<SymbolicCode, _> = serde_json::from_str("\"BOGUS_NOT_A_CODE\"");
    assert!(result.is_err(), "should reject unknown code name");
}

#[test]
fn symbolic_code_deserialize_rejects_empty_string() {
    let result: Result<SymbolicCode, _> = serde_json::from_str("\"\"");
    assert!(result.is_err(), "should reject empty string");
}

#[test]
fn symbolic_code_deserialize_rejects_non_string_json() {
    for input in &["123", "null", "true", "false", "[]", "{}"] {
        let result: Result<SymbolicCode, _> = serde_json::from_str(input);
        assert!(
            result.is_err(),
            "deserialize should reject non-string JSON input: {input}"
        );
    }
}

#[test]
fn symbolic_code_deserialize_rejects_wrong_case() {
    let result: Result<SymbolicCode, _> = serde_json::from_str("\"duplicate_key\"");
    assert!(result.is_err(), "should reject lowercase variant");
}

#[test]
fn symbolic_code_deserialize_rejects_whitespace_variant() {
    let result: Result<SymbolicCode, _> = serde_json::from_str("\" DUPLICATE_KEY \"");
    assert!(result.is_err(), "should reject whitespace-padded name");
}

#[test]
fn symbolic_code_deserialize_rejects_leading_trailing_spaces_in_json() {
    // JSON string with spaces - the spaces are part of the string value
    let result: Result<SymbolicCode, _> = serde_json::from_str("\"  DUPLICATE_KEY  \"");
    assert!(result.is_err(), "should reject string with extra spaces");
}
