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
