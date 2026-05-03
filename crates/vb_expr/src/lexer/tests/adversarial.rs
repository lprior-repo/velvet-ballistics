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
