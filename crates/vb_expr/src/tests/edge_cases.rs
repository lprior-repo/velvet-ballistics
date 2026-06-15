#![forbid(unsafe_code)]
//! Verified edge-case boundary tests for lexer, parser, and evaluator.
//!
//! Each test targets a specific verified gap in the existing test coverage:
//!
//! - Parser depth boundary: `MAX_DEPTH = 64`
//! - Lexer source length boundary: `MAX_SOURCE_BYTES = 4096`
//! - Lexer token count boundary: `MAX_TOKENS = 256`
//! - Empty expression program evaluation

use crate::eval::eval_expr_program;
use crate::lexer::{lex_expr, LiteralToken, Token};
use crate::parser::parse_expr;
use crate::ExprError;
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
        };
        other => panic!("expected ParseDepthExceeded {{ max: 64 }}, got {other:?}"),
    }
}

// =========================================================================
// Lexer source length boundary — MAX_SOURCE_BYTES = 4096
// =========================================================================

/// Lexing a 4095-byte source string succeeds (4095 < MAX_SOURCE_BYTES).
#[test]
fn lexer_source_4095_bytes_succeeds() -> crate::ExprResult<()> {
    let source = "1".repeat(4095);
    let tokens = lex_expr(&source)?;
    // Single integer "1" plus the End sentinel token
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0], Token::Literal(LiteralToken::I64(1)));
    assert_eq!(tokens[1], Token::End);
    Ok(());
}

/// Lexing a 4096-byte source string succeeds (4096 == MAX_SOURCE_BYTES).
///
/// The guard is `input.len() > MAX_SOURCE_BYTES`, so exactly 4096 bytes is
/// the largest acceptable input.
#[test]
fn lexer_source_4096_bytes_succeeds_at_boundary() -> crate::ExprResult<()> {
    let source = "1".repeat(4096);
    let tokens = lex_expr(&source)?;
    // Single integer "1" plus the End sentinel token
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0], Token::Literal(LiteralToken::I64(1)));
    assert_eq!(tokens[1], Token::End);
    Ok(());
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

/// An `ExprProgram` with zero operations and an empty evaluation stack
/// returns `StackUnderflow` at the finish_stack gate.
///
/// The evaluator's `finish_stack` returns `StackUnderflow` when the stack is
/// empty after executing all ops.  With zero ops, no value is ever pushed.
#[test]
fn empty_program_returns_stack_underflow() {
    // Build an empty program — 0 ops, max_stack = 0
    let program = match ExprProgram::try_from_ops(Vec::new().into_boxed_slice()) {
        Ok(p) => p,
        Err(e) => panic!("empty program should be valid, got error: {e:?}"),
    };

    // Evaluate with empty slots and constants
    match eval_expr_program(&program, &[], &[]) {
        Err(ExprError::StackUnderflow) => {
            // expected: empty stack → StackUnderflow
        }
        other => panic!("empty program evaluation should yield StackUnderflow, got {other:?}"),
    }
}
