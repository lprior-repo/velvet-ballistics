#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::cmp_owned,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::derivable_impls,
    clippy::duplicated_attributes,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::io_other_error,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::manual_unwrap_or_default,
    clippy::map_clone,
    clippy::map_flatten,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::new_without_default,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_sort_by,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_asref,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]

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
