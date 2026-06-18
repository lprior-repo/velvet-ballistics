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
    unused_variables
)]
#![forbid(unsafe_code)]
//! Verified edge-case boundary tests for lexer, parser, and evaluator.
//!
//! Each test targets a specific verified gap in the existing test coverage:
//!
//! - Parser depth boundary: `MAX_DEPTH = 64`
//! - Lexer source length boundary: `MAX_SOURCE_BYTES = 4096`
//! - Lexer token count boundary: `MAX_TOKENS = 256`
//! - Empty expression program evaluation

use crate::ExprError;
use crate::lexer::{Token, lex_expr};
use crate::parser::parse_expr;
use vb_core::ExprProgram;

// =========================================================================
// Parser depth boundary — MAX_DEPTH = 64
// =========================================================================

/// Parsing with 63 levels of nesting succeeds (depth 63 < MAX_DEPTH 64).
#[test]
fn parser_depth_63_nested_parens_succeeds() -> crate::ExprResult<()> {
    let depth = 63u8;
    let open = "(".repeat(usize::from(depth));
    let close = ")".repeat(usize::from(depth));
    let source = format!("{open}42{close}");
    let tokens = lex_expr(&source)?;
    let result = parse_expr(&tokens);
    // depth 63 < MAX_DEPTH(64), check_depth passes
    let expr = result?;
    assert_eq!(
        expr,
        crate::parser::ExprAst::Literal(crate::parser::ExprLiteral::I64(42))
    );
    Ok(())
}

/// Parsing with 64 levels of nesting succeeds (depth 64 == MAX_DEPTH).
///
/// The guard is `depth > MAX_DEPTH`, so depth 64 is the last allowed value.
#[test]
fn parser_depth_64_nested_parens_succeeds_at_boundary() -> crate::ExprResult<()> {
    let depth = 64u8;
    let open = "(".repeat(usize::from(depth));
    let close = ")".repeat(usize::from(depth));
    let source = format!("{open}42{close}");
    let tokens = lex_expr(&source)?;
    let result = parse_expr(&tokens);
    // depth 64 == MAX_DEPTH(64), check_depth(depth=64) => 64 > 64 is false
    let expr = result?;
    assert_eq!(
        expr,
        crate::parser::ExprAst::Literal(crate::parser::ExprLiteral::I64(42))
    );
    Ok(())
}

/// Parsing with 65 levels of nesting fails with ParseDepthExceeded { max: 64 }.
#[test]
fn parser_depth_65_nested_parens_fails() {
    let depth = 65u8;
    let source = format!(
        "({}42{})",
        "(".repeat(usize::from(depth)),
        ")".repeat(usize::from(depth))
    );

    let tokens = match lex_expr(&source) {
        Ok(t) => t,
        Err(e) => panic!("lexing well-formed parentheses should succeed, got {e:?}"),
    };

    match parse_expr(&tokens) {
        Err(ExprError::ParseDepthExceeded { max }) => {
            assert_eq!(max, 64, "error should report max=64, got max={max}");
        }
        other => panic!("expected ParseDepthExceeded {{ max: 64 }}, got {other:?}"),
    }
}

// =========================================================================
// Lexer source length boundary — MAX_SOURCE_BYTES = 4096
// =========================================================================

/// Lexing a 4095-byte source string succeeds (4095 < MAX_SOURCE_BYTES).
///
/// Uses a single very long identifier of exactly 4095 characters. The lexer
/// produces one Identifier token + End sentinel (2 tokens total).
#[test]
fn lexer_source_4095_bytes_succeeds() -> crate::ExprResult<()> {
    // Single 4095-char identifier → 1 Identifier token + End = 2 tokens
    let source = "x".repeat(4095);
    assert_eq!(source.len(), 4095);
    let tokens = lex_expr(&source)?;
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0], Token::Identifier(source.into_boxed_str()));
    assert_eq!(tokens[1], Token::End);
    Ok(())
}

/// Lexing a 4096-byte source string succeeds (4096 == MAX_SOURCE_BYTES).
///
/// The guard is `input.len() > MAX_SOURCE_BYTES`, so exactly 4096 bytes is
/// the largest acceptable input.
#[test]
fn lexer_source_4096_bytes_succeeds_at_boundary() -> crate::ExprResult<()> {
    let source = "x".repeat(4096);
    assert_eq!(source.len(), 4096);
    let tokens = lex_expr(&source)?;
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0], Token::Identifier(source.into_boxed_str()));
    assert_eq!(tokens[1], Token::End);
    Ok(())
}

/// Lexing a 4097-byte source string fails with ExpressionTooLong.
#[test]
fn lexer_source_4097_bytes_fails() {
    let source = "1".repeat(4097);

    match lex_expr(&source) {
        Err(ExprError::ExpressionTooLong { len, max }) => {
            assert_eq!(len, 4097, "reported len should be the actual input length");
            assert_eq!(max, 4096, "max should be MAX_SOURCE_BYTES");
        }
        other => panic!("expected ExpressionTooLong {{ len: 4097, max: 4096 }}, got {other:?}"),
    }
}

// =========================================================================
// Lexer token count boundary — MAX_TOKENS = 256
// =========================================================================

/// Lexing 256 integer tokens succeeds (256 == MAX_TOKENS).
///
/// Each `"1 "` fragment produces one integer token; the lexer appends an End
/// sentinel token which does not count against MAX_TOKENS.
#[test]
fn lexer_256_integers_succeeds() -> crate::ExprResult<()> {
    // 256 occurrences of "1 " → 256 integer tokens
    let source = ("1 ").repeat(256);
    let tokens = lex_expr(&source)?;
    // 256 data tokens + End sentinel
    assert_eq!(tokens.len(), 257);
    assert_eq!(
        tokens.last(),
        Some(&Token::End),
        "last token must be the End sentinel"
    );
    Ok(())
}

/// Lexing 257 integers fails with ExpressionTooLong.
#[test]
fn lexer_257_integers_fails() {
    // 257 occurrences of "1 " → 257 integer tokens, exceeding MAX_TOKENS(256)
    let source = ("1 ").repeat(257);

    match lex_expr(&source) {
        Err(ExprError::ExpressionTooLong { len, max }) => {
            assert_eq!(len, 257, "reported len should be the offending token count");
            assert_eq!(max, 256, "max should be MAX_TOKENS");
        }
        other => panic!("expected ExpressionTooLong {{ len: 257, max: 256 }}, got {other:?}"),
    }
}

// =========================================================================
// Empty expression program — StackUnderflow on zero ops
// =========================================================================

/// An `ExprProgram` with zero operations is rejected at construction time
/// because `check_expr_stack_bound` requires the final stack to be depth 1
/// (single result) or at least depth 0 with a non-empty program.
///
/// Empty programs fail `validate_expr_final_depth(0)` → `ExpressionStackUnderflow`.
#[test]
fn empty_program_rejected_at_construction() {
    // Build an empty program — 0 ops
    let result = ExprProgram::try_from_ops(Vec::new().into_boxed_slice());
    // Empty program → validate_expr_final_depth(0) → ExpressionStackUnderflow
    assert!(
        matches!(result, Err(vb_core::CoreError::ExpressionStackUnderflow)),
        "empty program should be rejected at construction"
    );
}
