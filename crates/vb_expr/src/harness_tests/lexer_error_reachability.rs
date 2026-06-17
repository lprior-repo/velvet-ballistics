#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables,
)]

#![forbid(unsafe_code)]
//! Lexer-error reachability tests (Category B).
//!
//! Verifies that the fuzz harness pipeline produces the correct `ExprError`
//! variants at the lexer stage for boundary-violating and invalid input.

use crate::ExprError;
use crate::lexer::lex_expr;

// ── Helper: run just the lex stage like the harness does ──

fn harness_lex_stage(source: &str) -> Result<(), ExprError> {
    lex_expr(source)?;
    Ok(())
}

// ── B-1: Source exceeding 4096 bytes → ExpressionTooLong ──

#[test]
fn harness_returns_expression_too_long_for_source_exceeding_4096_bytes() {
    // Given: source text of 4097 bytes
    let source = "x".repeat(4097);
    // When: lex stage runs
    let result = harness_lex_stage(&source);
    // Then: ExpressionTooLong with exact len and max
    match result {
        Err(ExprError::ExpressionTooLong { len, max }) => {
            assert_eq!(len, 4097, "len must match actual source length");
            assert_eq!(max, 4096, "max must be 4096 for source bytes");
        }
        other => panic!("expected ExpressionTooLong, got {:?}", other),
    }
}

// ── B-2: Tokens exceeding 256 → ExpressionTooLong ──

#[test]
fn harness_returns_expression_too_long_for_exceeding_256_tokens() {
    // Given: source text with 257 tokens: "1+" repeated 128 times + "1"
    // = 128 numbers + 128 pluses + 1 number = 257 tokens
    let source = "1+".repeat(128) + "1";
    // When: lex stage runs
    let result = harness_lex_stage(&source);
    // Then: ExpressionTooLong with exact token count
    match result {
        Err(ExprError::ExpressionTooLong { len, max }) => {
            assert_eq!(len, 257, "len must match actual token count");
            assert_eq!(max, 256, "max must be 256 for token count");
        }
        other => panic!("expected ExpressionTooLong, got {:?}", other),
    }
}

// ── B-3: Unterminated string → UnterminatedString ──

#[test]
fn harness_returns_unterminated_string_for_open_quote_without_close() {
    // Given: opening double-quote without closing quote
    let source = "\"hello";
    // When: lex stage runs
    let result = harness_lex_stage(source);
    // Then: UnterminatedString error
    match result {
        Err(ExprError::UnterminatedString) => {} // expected
        other => panic!("expected UnterminatedString, got {:?}", other),
    }
}

#[test]
fn harness_returns_unterminated_string_for_single_open_quote() {
    let source = "\"";
    let result = harness_lex_stage(source);
    match result {
        Err(ExprError::UnterminatedString) => {}
        other => panic!("expected UnterminatedString, got {:?}", other),
    }
}

#[test]
fn harness_returns_unterminated_string_for_open_quote_then_identifier() {
    let source = "\"abc";
    let result = harness_lex_stage(source);
    match result {
        Err(ExprError::UnterminatedString) => {}
        other => panic!("expected UnterminatedString, got {:?}", other),
    }
}

// ── B-4: Unexpected characters → UnexpectedChar ──

#[test]
fn harness_returns_unexpected_char_for_at_sign() {
    let source = "@";
    let result = harness_lex_stage(source);
    match result {
        Err(ExprError::UnexpectedChar { ch }) => assert_eq!(ch, '@'),
        other => panic!("expected UnexpectedChar with '@', got {:?}", other),
    }
}

#[test]
fn harness_returns_unexpected_char_for_hash_sign() {
    let source = "#";
    let result = harness_lex_stage(source);
    match result {
        Err(ExprError::UnexpectedChar { ch }) => assert_eq!(ch, '#'),
        other => panic!("expected UnexpectedChar with '#', got {:?}", other),
    }
}

#[test]
fn harness_returns_unexpected_char_for_tilde() {
    let source = "~";
    let result = harness_lex_stage(source);
    match result {
        Err(ExprError::UnexpectedChar { ch }) => assert_eq!(ch, '~'),
        other => panic!("expected UnexpectedChar with '~', got {:?}", other),
    }
}

#[test]
fn harness_returns_unexpected_char_for_backtick() {
    let source = "`";
    let result = harness_lex_stage(source);
    match result {
        Err(ExprError::UnexpectedChar { ch }) => assert_eq!(ch, '`'),
        other => panic!("expected UnexpectedChar with '`', got {:?}", other),
    }
}

#[test]
fn harness_returns_unexpected_char_for_unicode_division_sign() {
    let source = "\u{00F7}"; // ÷
    let result = harness_lex_stage(source);
    match result {
        Err(ExprError::UnexpectedChar { ch }) => assert_eq!(ch, '\u{00F7}'),
        other => panic!("expected UnexpectedChar with '÷', got {:?}", other),
    }
}

// ── B-5: Integer literal exceeding i64 range → IntegerOutOfRange ──

#[test]
fn harness_returns_integer_out_of_range_for_20_digit_number() {
    let source = "99999999999999999999"; // 20 digits, exceeds i64::MAX
    let result = harness_lex_stage(source);
    match result {
        Err(ExprError::IntegerOutOfRange) => {}
        other => panic!("expected IntegerOutOfRange, got {:?}", other),
    }
}

#[test]
fn harness_returns_integer_out_of_range_for_exceeding_i64_max() {
    // i64::MAX = 9223372036854775807, this exceeds it
    let source = "9223372036854775808";
    let result = harness_lex_stage(source);
    match result {
        Err(ExprError::IntegerOutOfRange) => {}
        other => panic!("expected IntegerOutOfRange, got {:?}", other),
    }
}

#[test]
fn harness_returns_integer_out_of_range_for_way_above_i64_max() {
    let source = "99999999999999999999999999999999999";
    let result = harness_lex_stage(source);
    match result {
        Err(ExprError::IntegerOutOfRange) => {}
        other => panic!("expected IntegerOutOfRange, got {:?}", other),
    }
}

// ── B-6: Boundary: exactly 4096-byte source accepted ──

#[test]
fn harness_accepts_source_at_exactly_4096_bytes_boundary() {
    // Given: valid expression padded to exactly 4096 bytes
    let expr = "true";
    let padding = 4096 - expr.len();
    let source = format!("{}{}", expr, " ".repeat(padding));
    assert_eq!(source.len(), 4096, "source must be exactly 4096 bytes");
    // When: lex stage runs
    let result = harness_lex_stage(&source);
    // Then: must succeed (no ExpressionTooLong)
    assert!(
        result.is_ok(),
        "4096-byte source must be accepted, got {:?}",
        result
    );
}

// ── B-6: Boundary: exactly 256 tokens accepted ──

#[test]
fn harness_accepts_source_with_exactly_256_tokens_boundary() {
    // Given: "1+" repeated 128 times = 128 numbers + 128 pluses = 256 tokens
    // The trailing "+" is fine — lex just counts tokens, parse handles validity.
    let source = "1+".repeat(128);
    let result = harness_lex_stage(&source);
    assert!(
        result.is_ok(),
        "256 tokens must be accepted, got {:?}",
        result
    );
}
