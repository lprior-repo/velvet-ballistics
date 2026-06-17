#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports, clippy::approximate_const, clippy::absurd_extreme_comparisons)]
#![allow(dead_code, unused_imports)]

#![forbid(unsafe_code)]
//! Adversarial lexer tests.

use crate::ExprError;
use crate::lexer::{SpannedToken, Token, TokenSpan, lex_expr};

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
    let expected = vec![
        Token::Literal(crate::lexer::LiteralToken::I64(i64::MAX)),
        Token::End,
    ];
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
    // A single token "1" repeated to fill MAX_SOURCE_BYTES but still produce
    // only one token. We use spaces to separate tokens and stay within
    // both the source byte limit and the token limit.
    // 256 tokens * 2 bytes ("1 ") = 512 bytes, well under 4096.
    let source = "1 ".repeat(255); // 255 tokens of "1" + final End
    let r = lex_expr(&source.trim_end());
    assert!(
        r.is_ok(),
        "BH-LX-004: source within limits should be accepted"
    );
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
    assert_eq!(
        tokens.first(),
        Some(&Token::Literal(crate::lexer::LiteralToken::I64(i64::MAX)))
    );
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
    assert_eq!(
        tokens.first(),
        Some(&Token::Operator(crate::lexer::BinaryOp::Sub))
    );
    assert_eq!(
        tokens.get(1),
        Some(&Token::Literal(crate::lexer::LiteralToken::I64(5)))
    );
    Ok(())
}

// =========================================================================
// Adversarial: extended boundary and character coverage
// =========================================================================

/// Rejects every ASCII punctuation character not in the grammar.
#[test]
fn lex_expr_rejects_punctuation_characters() -> crate::ExprResult<()> {
    let punct: &[char] = &[
        '%', '&', '{', '}', '[', ']', ';', ':', '\\', '|', '?', '~', '`', '^',
    ];
    for ch in punct {
        let result = lex_expr(&ch.to_string());
        assert!(
            matches!(result, Err(ExprError::UnexpectedChar { .. })),
            "character '{ch}' should be rejected as UnexpectedChar"
        );
    }
    Ok(())
}

/// Rejects a CJK character as unexpected.
#[test]
fn lex_expr_rejects_cjk_character() -> crate::ExprResult<()> {
    let result = lex_expr("\u{4E2D}");
    let Err(ExprError::UnexpectedChar { ch }) = result else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected UnexpectedChar for CJK".into(),
        });
    };
    assert_eq!(ch, '\u{4E2D}');
    Ok(())
}

/// Rejects an emoji (multi-byte UTF-8) as unexpected.
#[test]
fn lex_expr_rejects_emoji_character() -> crate::ExprResult<()> {
    let result = lex_expr("\u{1F600}");
    let Err(ExprError::UnexpectedChar { ch }) = result else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected UnexpectedChar for emoji".into(),
        });
    };
    assert_eq!(ch, '\u{1F600}');
    Ok(())
}

/// Accepts source exactly at the byte limit with a single long identifier.
#[test]
fn lex_expr_accepts_source_exactly_at_byte_limit() -> crate::ExprResult<()> {
    let ident = "a".repeat(crate::lexer::MAX_SOURCE_BYTES);
    let tokens = lex_expr(&ident)?;
    assert_eq!(tokens.len(), 2);
    assert!(matches!(tokens.first(), Some(Token::Identifier(_))));
    assert_eq!(tokens.last(), Some(&Token::End));
    Ok(())
}

/// Accepts a string literal whose inner content fills the remaining byte budget.
#[test]
fn lex_expr_accepts_max_length_string_literal() -> crate::ExprResult<()> {
    let inner_len = crate::lexer::MAX_SOURCE_BYTES.saturating_sub(2);
    let inner = "x".repeat(inner_len);
    let source = format!("\"{}\"", inner);
    let tokens = lex_expr(&source)?;
    assert!(matches!(
        tokens.first(),
        Some(Token::Literal(crate::lexer::LiteralToken::Text(_)))
    ));
    assert_eq!(tokens.get(1), Some(&Token::End));
    Ok(())
}

/// String containing numeric characters is a text token, not a number.
#[test]
fn lex_expr_handles_string_with_numeric_content() -> crate::ExprResult<()> {
    let tokens = lex_expr("\"42\"")?;
    let expected = vec![
        Token::Literal(crate::lexer::LiteralToken::Text(Box::from("42"))),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

/// Operators lex correctly without whitespace separation.
#[test]
fn lex_expr_handles_mixed_operators_without_whitespace() -> crate::ExprResult<()> {
    let tokens = lex_expr("1+2*3")?;
    let expected = vec![
        Token::Literal(crate::lexer::LiteralToken::I64(1)),
        Token::Operator(crate::lexer::BinaryOp::Add),
        Token::Literal(crate::lexer::LiteralToken::I64(2)),
        Token::Operator(crate::lexer::BinaryOp::Mul),
        Token::Literal(crate::lexer::LiteralToken::I64(3)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

/// References adjacent to operators lex correctly without whitespace.
#[test]
fn lex_expr_handles_references_adjacent_to_operators() -> crate::ExprResult<()> {
    let tokens = lex_expr("$a+$b")?;
    let expected = vec![
        Token::Reference(Box::from("$a")),
        Token::Operator(crate::lexer::BinaryOp::Add),
        Token::Reference(Box::from("$b")),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

/// Dollar followed by a number is a valid reference, not Dollar + Integer.
#[test]
fn lex_expr_handles_dollar_followed_by_number_is_reference() -> crate::ExprResult<()> {
    let tokens = lex_expr("$1")?;
    let expected = vec![Token::Reference(Box::from("$1")), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

/// Expression with minus-before-minus is two consecutive Sub operators.
#[test]
fn lex_expr_handles_consecutive_minus_tokens() -> crate::ExprResult<()> {
    let tokens = lex_expr("1 - - 5")?;
    let expected = vec![
        Token::Literal(crate::lexer::LiteralToken::I64(1)),
        Token::Operator(crate::lexer::BinaryOp::Sub),
        Token::Operator(crate::lexer::BinaryOp::Sub),
        Token::Literal(crate::lexer::LiteralToken::I64(5)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

/// Verifies that trying to lex i64::MIN as a positive literal is rejected.
#[test]
fn lex_expr_rejects_i64_min_positive_magnitude() -> crate::ExprResult<()> {
    let result = lex_expr("9223372036854775808");
    assert!(
        matches!(result, Err(ExprError::IntegerOutOfRange)),
        "positive magnitude of i64::MIN exceeds i64::MAX"
    );
    Ok(())
}

/// An identifier consisting solely of underscores is valid.
#[test]
fn lex_expr_handles_underscore_only_identifier() -> crate::ExprResult<()> {
    let tokens = lex_expr("___")?;
    let expected = vec![Token::Identifier(Box::from("___")), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

/// Single-character string literal containing just a space.
#[test]
fn lex_expr_handles_string_with_single_space() -> crate::ExprResult<()> {
    let tokens = lex_expr("\" \"")?;
    let expected = vec![
        Token::Literal(crate::lexer::LiteralToken::Text(Box::from(" "))),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

/// String literal containing operator-like characters.
#[test]
fn lex_expr_handles_string_with_operator_characters() -> crate::ExprResult<()> {
    let tokens = lex_expr("\"+ - * / == != < <= > >=\"")?;
    let expected = vec![
        Token::Literal(crate::lexer::LiteralToken::Text(Box::from(
            "+ - * / == != < <= > >=",
        ))),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}
