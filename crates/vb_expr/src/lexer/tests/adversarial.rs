//! Adversarial lexer tests.

use crate::lexer::{lex_expr, SpannedToken, Token, TokenSpan};
use crate::ExprError;

#[test]
fn lex_expr_rejects_empty_string_as_only_end_token() -> crate::ExprResult<()> {
    let tokens = lex_expr("")?;
    assert_eq!(
        tokens.len(),
        1,
        "empty input should produce exactly one End token"
    );
    assert_eq!(tokens.first(), Some(&Token::End));
    Ok(())
}

#[test]
fn lex_expr_rejects_whitespace_only_input_as_only_end_token() -> crate::ExprResult<()> {
    let tokens = lex_expr("   \t\n  ")?;
    assert_eq!(
        tokens.len(),
        1,
        "whitespace-only input should produce exactly one End token"
    );
    assert_eq!(tokens.first(), Some(&Token::End));
    Ok(())
}

#[test]
fn lex_expr_rejects_unexpected_unicode_character() -> crate::ExprResult<()> {
    let result = lex_expr("\u{00F7}");
    let Err(ExprError::UnexpectedChar { ch }) = result else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected UnexpectedChar for unicode division sign".into(),
        });
    };
    assert_eq!(ch, '\u{00F7}');
    Ok(())
}

#[test]
fn lex_expr_rejects_unexpected_at_sign() -> crate::ExprResult<()> {
    let result = lex_expr("@");
    let Err(ExprError::UnexpectedChar { ch }) = result else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected UnexpectedChar for @".into(),
        });
    };
    assert_eq!(ch, '@');
    Ok(())
}

#[test]
fn lex_expr_handles_max_i64_literal() -> crate::ExprResult<()> {
    let tokens = lex_expr("9223372036854775807")?;
    let expected = vec![Token::Literal(crate::lexer::LiteralToken::I64(i64::MAX)), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lex_expr_rejects_i64_overflow_literal() -> crate::ExprResult<()> {
    let result = lex_expr("9223372036854775808");
    assert!(
        matches!(result, Err(ExprError::IntegerOutOfRange)),
        "expected IntegerOutOfRange for value exceeding i64::MAX"
    );
    Ok(())
}

#[test]
fn lex_expr_tokenizes_deeply_nested_parentheses() -> crate::ExprResult<()> {
    let tokens = lex_expr("((((1))))")?;
    assert_eq!(tokens.first(), Some(&Token::LParen));
    assert_eq!(tokens.last(), Some(&Token::End));
    let rparen_count = tokens.iter().filter(|t| matches!(t, Token::RParen)).count();
    assert_eq!(rparen_count, 4);
    Ok(())
}

#[test]
fn lex_expr_lone_dollar_after_whitespace_is_dollar_token() -> crate::ExprResult<()> {
    let tokens = lex_expr("$ + 1")?;
    assert_eq!(tokens.first(), Some(&Token::Dollar));
    Ok(())
}

#[test]
fn lex_expr_rejects_bare_exclamation_mark() -> crate::ExprResult<()> {
    let result = lex_expr("!");
    let Err(ExprError::UnexpectedChar { ch }) = result else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected UnexpectedChar for bare !".into(),
        });
    };
    assert_eq!(ch, '!');
    Ok(())
}

#[test]
fn lex_expr_rejects_bare_equals_sign() -> crate::ExprResult<()> {
    let result = lex_expr("=");
    let Err(ExprError::UnexpectedChar { ch }) = result else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected UnexpectedChar for bare =".into(),
        });
    };
    assert_eq!(ch, '=');
    Ok(())
}

#[test]
fn lex_expr_handles_string_with_spaces() -> crate::ExprResult<()> {
    let tokens = lex_expr("\"a b c\"")?;
    let expected = vec![
        Token::Literal(crate::lexer::LiteralToken::Text(Box::from("a b c"))),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lex_expr_rejects_unterminated_string_immediately() -> crate::ExprResult<()> {
    let result = lex_expr("\"");
    assert!(matches!(result, Err(ExprError::UnterminatedString)));
    Ok(())
}

#[test]
fn lex_expr_reference_with_dots_allows_path_access() -> crate::ExprResult<()> {
    let tokens = lex_expr("$input.field1.field2.field3")?;
    let expected = vec![
        Token::Reference(Box::from("$input.field1.field2.field3")),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

// =========================================================================
// BLACKHAT security regression tests -- lexer
// =========================================================================

/// BH-LX-001: Integer literal overflow rejected.
///
/// The lexer must reject integer literals that exceed i64::MAX.
#[test]
fn blackhat_lx_001_integer_overflow_rejected() -> crate::ExprResult<()> {
    let r = lex_expr("9223372036854775808"); // i64::MAX + 1
    assert!(
        matches!(r, Err(ExprError::IntegerOutOfRange)),
        "BH-LX-001: value exceeding i64::MAX must be rejected"
    );
    Ok(())
}

/// BH-LX-002: Unterminated string rejected.
#[test]
fn blackhat_lx_002_unterminated_string_rejected() -> crate::ExprResult<()> {
    let r = lex_expr("\"unterminated string without closing quote");
    assert!(
        matches!(r, Err(ExprError::UnterminatedString)),
        "BH-LX-002: unterminated string must be rejected"
    );
    Ok(())
}

/// BH-LX-003: Unexpected characters rejected with typed error.
#[test]
fn blackhat_lx_003_unexpected_chars_rejected() -> crate::ExprResult<()> {
    for ch in ['@', '#', '^', '~', '`', '\u{00F7}'] {
        let r = lex_expr(&ch.to_string());
        assert!(
            matches!(r, Err(ExprError::UnexpectedChar { .. })),
            "BH-LX-003: character '{ch}' should be UnexpectedChar"
        );
    }
    Ok(())
}

/// BH-LX-004: Source length boundary -- exactly at limit is accepted.
#[test]
fn blackhat_lx_004_source_length_boundary_accepted() -> crate::ExprResult<()> {
    // Exactly MAX_SOURCE_BYTES should be accepted (single digit "1" repeated)
    let source = "1".repeat(crate::lexer::MAX_SOURCE_BYTES);
    let r = lex_expr(&source);
    assert!(r.is_ok(), "BH-LX-004: source at exact limit should be accepted");
    Ok(())
}

/// BH-LX-005: Source length boundary -- one over limit is rejected.
#[test]
fn blackhat_lx_005_source_length_one_over_rejected() -> crate::ExprResult<()> {
    let source = "1".repeat(crate::lexer::MAX_SOURCE_BYTES.saturating_add(1));
    let r = lex_expr(&source);
    assert!(
        matches!(r, Err(ExprError::ExpressionTooLong { .. })),
        "BH-LX-005: source one byte over limit should be rejected"
    );
    Ok(())
}

/// BH-LX-006: Lone dollar sign produces Dollar token, not crash.
#[test]
fn blackhat_lx_006_lone_dollar_no_crash() -> crate::ExprResult<()> {
    let tokens = lex_expr("$")?;
    assert_eq!(tokens.first(), Some(&Token::Dollar));
    assert_eq!(tokens.get(1), Some(&Token::End));
    Ok(())
}

/// BH-LX-007: Bare equals sign rejected.
#[test]
fn blackhat_lx_007_bare_equals_rejected() -> crate::ExprResult<()> {
    let r = lex_expr("=");
    assert!(
        matches!(r, Err(ExprError::UnexpectedChar { ch: '=' })),
        "BH-LX-007: bare '=' should be UnexpectedChar"
    );
    Ok(())
}

/// BH-LX-008: Bare exclamation mark rejected.
#[test]
fn blackhat_lx_008_bare_exclamation_rejected() -> crate::ExprResult<()> {
    let r = lex_expr("!");
    assert!(
        matches!(r, Err(ExprError::UnexpectedChar { ch: '!' })),
        "BH-LX-008: bare '!' should be UnexpectedChar"
    );
    Ok(())
}

/// BH-LX-009: i64::MAX literal accepted.
#[test]
fn blackhat_lx_009_i64_max_accepted() -> crate::ExprResult<()> {
    let tokens = lex_expr("9223372036854775807")?;
    assert_eq!(tokens.first(), Some(&Token::Literal(crate::lexer::LiteralToken::I64(i64::MAX))));
    Ok(())
}

/// BH-LX-010: Negative integer literals cannot be lexed directly.
///
/// `-9223372036854775808` is lexed as Minus + i64::MAX overflow.
/// The lexer does not produce negative literals; negation is handled by
/// the parser as a unary operator.
#[test]
fn blackhat_lx_010_negative_literal_is_unary_op() -> crate::ExprResult<()> {
    let tokens = lex_expr("-5")?;
    assert_eq!(tokens.first(), Some(&Token::Operator(crate::lexer::BinaryOp::Sub)));
    assert_eq!(tokens.get(1), Some(&Token::Literal(crate::lexer::LiteralToken::I64(5))));
    Ok(())
}
