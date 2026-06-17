#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports, clippy::approximate_const, clippy::absurd_extreme_comparisons)]

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
