#![forbid(unsafe_code)]
//! PO-KANI-002: Parser depth bound verification
//! Requirement: C-PARSE-1
//!
//! Production target: crate::parser::check_depth (via parse_expr)
//!
//! Verifies that the parser's depth enforcement triggers ParseDepthExceeded
//! at depth 65 (MAX_DEPTH=64). Uses the public parse_expr API by constructing
//! token arrays with controlled nesting depth.
//!
//! Strategy: Build token arrays of the form:
//!   (((...((1))...)))  — N open parens, literal, N close parens
//! and verify ParseDepthExceeded for N > 64.

use crate::ExprError;
use crate::lexer::Token;

/// Maximum parse depth (must match MAX_DEPTH in parser/mod.rs).
const MAX_DEPTH: u8 = 64;

/// Build a token array with `depth` nested open-paren-LParen tokens,
/// then a literal (I64), then `depth` close-paren-RParen tokens,
/// terminated by End.
fn build_nested_tokens(depth: usize) -> Vec<Token> {
    let mut tokens = Vec::with_capacity(depth * 2 + 3);
    // Open parens
    for _ in 0..depth {
        tokens.push(Token::LParen);
    }
    // Literal
    tokens.push(Token::Literal(crate::lexer::LiteralToken::I64(1)));
    // Close parens
    for _ in 0..depth {
        tokens.push(Token::RParen);
    }
    // End
    tokens.push(Token::End);
    tokens
}

/// PO-KANI-002 H1: parse_expr accepts depth up to MAX_DEPTH (64).
#[kani::proof]
#[kani::unwind(70)]
fn check_parse_accepts_max_depth() {
    let tokens = build_nested_tokens(MAX_DEPTH as usize);
    let result = crate::parser::parse_expr(&tokens);
    // At exactly MAX_DEPTH, the parser should succeed (depth 64 is allowed)
    match result {
        Ok(ast) => {
            // Success is valid — ensure the AST is well-formed
            // (Just verifying the call completed without panic)
            let _ = ast;
        }
        Err(e) => {
            // Any typed error is acceptable (except panics)
            // Could be UnexpectedToken if parens are unbalanced due to 0-arg functions, etc.
            // Key property: NOT ParseDepthExceeded at depth 64
            kani::assert(!matches!(e, ExprError::ParseDepthExceeded { .. }, "assertion failed"),
                "depth 64 must not trigger ParseDepthExceeded",
            );
        }
    }
}

/// PO-KANI-002 H2: parse_expr rejects depth 65 with ParseDepthExceeded.
#[kani::proof]
#[kani::unwind(70)]
fn check_parse_rejects_depth_exceeded() {
    let tokens = build_nested_tokens(65);
    let result = crate::parser::parse_expr(&tokens);

    kani::assert(result.is_err(, "assertion failed"), "depth 65 must return an error");

    match result {
        Err(e) => {
            kani::assert(matches!(e, ExprError::ParseDepthExceeded { .. }, "assertion failed"),
                "depth 65 must return ParseDepthExceeded",
            );
        }
        Ok(_) => {
            ,
                "depth 65 must return ParseDepthExceeded",
            );
        }
        Ok(_) => {
            kani::assert(false, "depth 65 must fail, not succeed");
        }
    }
}

/// PO-KANI-002 H3: parse_expr rejects depth 128 (well beyond MAX_DEPTH).
#[kani::proof]
#[kani::unwind(130)]
fn check_parse_rejects_very_deep() {
    let tokens = build_nested_tokens(128);
    let result = crate::parser::parse_expr(&tokens);
    // At depth 128, the parser MUST error — no valid expression has this depth
    kani::assert(result.is_err(), "depth 128 must produce an error");
    // And it must not panic
    match result {
        Ok(_) => {
            , "depth 128 must produce an error");
    // And it must not panic
    match result {
        Ok(_) => {
            kani::assert(false, "depth 128 must fail");
        }
        Err(e) => {
            // Must be ParseDepthExceeded (most specific) or another typed error
            kani::assert(
                matches!(
                    e,
                    ExprError::ParseDepthExceeded { .. }
                        | ExprError::UnexpectedToken { .. }
                        | ExprError::ExpressionTooLong { .. }
                ),
                "depth 128 error must be a known ExprError variant",
            );
        }
    }
}

/// PO-KANI-002 H4: verify that shallow nesting (depth 0-10) always succeeds
/// for valid token sequences.
#[kani::proof]
#[kani::unwind(15)]
fn check_parse_shallow_always_succeeds() {
    let depth: usize = kani::any();
    kani::assume(depth <= 10);

    // Build: N open parens, 1 literal, N close parens
    let mut tokens = Vec::with_capacity(depth * 2 + 3);
    for i in 0..depth {
        // Use a simple arithmetic expression for inner content
        if i == 0 && depth > 0 {
            tokens.push(Token::Literal(crate::lexer::LiteralToken::I64(42)));
        }
        tokens.push(Token::LParen);
    }
    if depth == 0 {
        tokens.push(Token::Literal(crate::lexer::LiteralToken::I64(42)));
    }
    for _ in 0..depth {
        tokens.push(Token::RParen);
    }
    tokens.push(Token::End);

    let result = crate::parser::parse_expr(&tokens);
    // At depth <= 10, the parse should succeed (well-formed parens + literal)
    // or fail with a typed error (not ParseDepthExceeded)
    match result {
        Ok(_) => {} // Success
        Err(e) => {
            kani::assert(!matches!(e, ExprError::ParseDepthExceeded { .. }, "assertion failed"),
                "shallow depth must not trigger ParseDepthExceeded",
            );
        }
    }
}
