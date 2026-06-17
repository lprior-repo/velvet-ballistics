#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
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
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
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

#![forbid(unsafe_code)]
//! Input-boundary behavior tests (Category A).
//!
//! Tests that the fuzz harness pipeline handles invalid UTF-8, empty bytes,
//! whitespace-only input, and arbitrary byte sequences without panicking.

use crate::ExprError;
use crate::lexer::lex_expr;

/// Simulates the first step of `fuzz_expression`: UTF-8 validation.
/// Returns Ok(()) if the pipeline should proceed, or an early-return signal.
fn validate_utf8_for_harness(data: &[u8]) -> bool {
    std::str::from_utf8(data).is_ok()
}

/// Full pipeline simulator matching `fuzz_expression` behavior exactly.
fn simulate_harness_pipeline(data: &[u8]) -> Result<(), ExprError> {
    let text = std::str::from_utf8(data).map_err(|_| ExprError::UnexpectedEof)?;
    // Stage 1: Lex
    crate::lexer::lex_expr(text)?;
    // (If we get here, lex succeeded. In the real harness, parse/compile/eval
    // would follow. For input boundary tests, we only care that no panic occurs.)
    Ok(())
}

// ── A-1: Invalid UTF-8 causes silent early return ──

#[test]
fn harness_returns_silently_for_invalid_utf8_ff_fe_fd() {
    // Given: invalid UTF-8 bytes (0xFF, 0xFE, 0xFD)
    let data: &[u8] = &[0xFF, 0xFE, 0xFD];
    // When: utf-8 validation runs
    let is_valid = validate_utf8_for_harness(data);
    // Then: it's rejected as invalid UTF-8
    assert!(
        !is_valid,
        "0xFF 0xFE 0xFD must be detected as invalid UTF-8"
    );
    // And: the full pipeline must not panic on this input
    let result = simulate_harness_pipeline(data);
    assert!(
        result.is_err(),
        "invalid UTF-8 must produce an error from pipeline"
    );
}

// ── A-2: Empty bytes produce valid UTF-8, lex returns [End] ──

#[test]
fn harness_returns_silently_for_empty_bytes() -> crate::ExprResult<()> {
    // Given: empty byte slice
    let data: &[u8] = b"";
    // When: validated as UTF-8
    assert!(
        validate_utf8_for_harness(data),
        "empty bytes are valid UTF-8"
    );
    let text = std::str::from_utf8(data).map_err(|_| ExprError::UnexpectedEof)?;
    // Then: lex returns Ok with [End] (empty input is valid for lexing)
    let tokens = lex_expr(text)?;
    assert_eq!(
        tokens.len(),
        1,
        "empty input must produce exactly one End token"
    );
    Ok(())
}

// ── A-3: Whitespace-only input is valid UTF-8, lex returns [End] ──

#[test]
fn harness_returns_silently_for_whitespace_only_input() -> crate::ExprResult<()> {
    // Given: whitespace-only input
    let data: &[u8] = b"   \t\n  ";
    // When: validated as UTF-8
    assert!(validate_utf8_for_harness(data), "whitespace is valid UTF-8");
    let text = std::str::from_utf8(data).map_err(|_| ExprError::UnexpectedEof)?;
    // Then: lex returns Ok with [End]
    let tokens = lex_expr(text)?;
    assert_eq!(
        tokens.len(),
        1,
        "whitespace-only must produce exactly one End token"
    );
    Ok(())
}

// ── A-4: Harness never panics on arbitrary byte sequences ──

#[test]
fn harness_never_panics_on_null_byte_input() {
    // Given: input containing null bytes
    let data: &[u8] = b"\x00\x00\x00";
    // When/Then: pipeline must not panic
    let _ = simulate_harness_pipeline(data);
}

#[test]
fn harness_never_panics_on_zero_length_byte() {
    // Given: single zero byte
    let data: &[u8] = &[0u8];
    // When/Then: pipeline must not panic
    let _ = simulate_harness_pipeline(data);
}

#[test]
fn harness_never_panics_on_binary_garbage() {
    // Given: arbitrary binary garbage
    let data: &[u8] = &[
        0x00, 0x01, 0x02, 0x80, 0x81, 0xFE, 0xFF, 0xC0, 0xC1, 0xF5, 0xF6, 0xF7,
    ];
    // When/Then: pipeline must not panic
    let _ = simulate_harness_pipeline(data);
}

#[test]
fn harness_never_panics_on_all_zeros_short() {
    let data: &[u8] = &[0u8; 64];
    let _ = simulate_harness_pipeline(data);
}

#[test]
fn harness_never_panics_on_all_ones() {
    let data: &[u8] = &[0xFFu8; 32];
    let _ = simulate_harness_pipeline(data);
}
