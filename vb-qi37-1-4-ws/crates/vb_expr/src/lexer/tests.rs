#![forbid(unsafe_code)]
//! Lexer tests.

#[allow(unused_imports)]
use crate::ExprError;
#[allow(unused_imports)]
use crate::lexer::{
    BinaryOp, LiteralToken, SpannedToken, Token, TokenSpan, UnaryOp, lex_expr, lex_expr_spanned,
};

mod adversarial;

#[test]
fn lexes_integer_literal() -> crate::ExprResult<()> {
    let tokens = lex_expr("42")?;
    let expected = vec![Token::Literal(LiteralToken::I64(42)), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_boolean_and_null_literals() -> crate::ExprResult<()> {
    let tokens = lex_expr("true false null")?;
    let expected = vec![
        Token::Literal(LiteralToken::Bool(true)),
        Token::Literal(LiteralToken::Bool(false)),
        Token::Literal(LiteralToken::Null),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_string_literal() -> crate::ExprResult<()> {
    let tokens = lex_expr("\"hello\"")?;
    let expected = vec![
        Token::Literal(LiteralToken::Text(Box::from("hello"))),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_reference() -> crate::ExprResult<()> {
    let tokens = lex_expr("$input.value")?;
    let expected = vec![Token::Reference(Box::from("$input.value")), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_operators() -> crate::ExprResult<()> {
    let tokens = lex_expr("+ - * / == != < <= > >=")?;
    let expected = vec![
        Token::Operator(BinaryOp::Add),
        Token::Operator(BinaryOp::Sub),
        Token::Operator(BinaryOp::Mul),
        Token::Operator(BinaryOp::Div),
        Token::Operator(BinaryOp::Eq),
        Token::Operator(BinaryOp::NotEq),
        Token::Operator(BinaryOp::Lt),
        Token::Operator(BinaryOp::Lte),
        Token::Operator(BinaryOp::Gt),
        Token::Operator(BinaryOp::Gte),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_keywords() -> crate::ExprResult<()> {
    let tokens = lex_expr("and or not")?;
    let expected = vec![
        Token::Operator(BinaryOp::And),
        Token::Operator(BinaryOp::Or),
        Token::Unary(UnaryOp::Not),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_helper_identifiers() -> crate::ExprResult<()> {
    let tokens = lex_expr("contains starts_with")?;
    let expected = vec![
        Token::Identifier(Box::from("contains")),
        Token::Identifier(Box::from("starts_with")),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lex_expr_spanned_preserves_exact_byte_spans() -> crate::ExprResult<()> {
    let tokens = lex_expr_spanned("$foo + 12")?;
    let expected = vec![
        SpannedToken {
            token: Token::Reference(Box::from("$foo")),
            span: TokenSpan { start: 0, end: 4 },
        },
        SpannedToken {
            token: Token::Operator(BinaryOp::Add),
            span: TokenSpan { start: 5, end: 6 },
        },
        SpannedToken {
            token: Token::Literal(LiteralToken::I64(12)),
            span: TokenSpan { start: 7, end: 9 },
        },
        SpannedToken {
            token: Token::End,
            span: TokenSpan { start: 9, end: 9 },
        },
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lex_expr_accepts_max_tokens_plus_end_sentinel() -> crate::ExprResult<()> {
    let source = "1 ".repeat(crate::lexer::MAX_TOKENS);
    let tokens = lex_expr(&source)?;
    let last = tokens.last().ok_or(ExprError::UnexpectedEof)?;
    assert_eq!(tokens.len(), crate::lexer::MAX_TOKENS.saturating_add(1));
    assert_eq!(last, &Token::End);
    Ok(())
}

#[test]
fn rejects_token_limit() {
    let source = "1 + ".repeat(crate::lexer::MAX_TOKENS);
    let result = lex_expr(&source);
    assert!(matches!(result, Err(ExprError::ExpressionTooLong { .. })));
}

#[test]
fn rejects_source_length_limit() {
    let source = "1".repeat(crate::lexer::MAX_SOURCE_BYTES.saturating_add(1));
    let result = lex_expr(&source);
    assert!(matches!(result, Err(ExprError::ExpressionTooLong { .. })));
}

#[test]
fn rejects_unterminated_string() {
    let result = lex_expr("\"unterminated");
    assert!(matches!(result, Err(ExprError::UnterminatedString)));
}

#[test]
fn lone_dollar_produces_dollar_token() -> crate::ExprResult<()> {
    let tokens = lex_expr("$")?;
    let expected = vec![Token::Dollar, Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn rejects_unexpected_character() {
    let result = lex_expr("@");
    assert!(matches!(result, Err(ExprError::UnexpectedChar { ch: '@' })));
}
// --- BDD lexer tests ---
#[test]
fn lex_expr_tokenizes_addition_expression() -> crate::ExprResult<()> {
    let tokens = lex_expr("3 + 5")?;
    let expected = vec![
        Token::Literal(LiteralToken::I64(3)),
        Token::Operator(BinaryOp::Add),
        Token::Literal(LiteralToken::I64(5)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lex_expr_tokenizes_subtraction_expression() -> crate::ExprResult<()> {
    let tokens = lex_expr("10 - 4")?;
    let expected = vec![
        Token::Literal(LiteralToken::I64(10)),
        Token::Operator(BinaryOp::Sub),
        Token::Literal(LiteralToken::I64(4)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lex_expr_tokenizes_multiplication_expression() -> crate::ExprResult<()> {
    let tokens = lex_expr("6 * 7")?;
    let expected = vec![
        Token::Literal(LiteralToken::I64(6)),
        Token::Operator(BinaryOp::Mul),
        Token::Literal(LiteralToken::I64(7)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lex_expr_tokenizes_division_expression() -> crate::ExprResult<()> {
    let tokens = lex_expr("20 / 4")?;
    let expected = vec![
        Token::Literal(LiteralToken::I64(20)),
        Token::Operator(BinaryOp::Div),
        Token::Literal(LiteralToken::I64(4)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lex_expr_tokenizes_parenthesized_expression() -> crate::ExprResult<()> {
    let tokens = lex_expr("(1 + 2)")?;
    let expected = vec![
        Token::LParen,
        Token::Literal(LiteralToken::I64(1)),
        Token::Operator(BinaryOp::Add),
        Token::Literal(LiteralToken::I64(2)),
        Token::RParen,
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lex_expr_tokenizes_string_literal() -> crate::ExprResult<()> {
    let tokens = lex_expr("\"hello world\"")?;
    let expected = vec![
        Token::Literal(LiteralToken::Text(Box::from("hello world"))),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lex_expr_tokenizes_variable_reference() -> crate::ExprResult<()> {
    let tokens = lex_expr("$my_var")?;
    let expected = vec![Token::Reference(Box::from("$my_var")), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lex_expr_tokenizes_boolean_literals() -> crate::ExprResult<()> {
    let tokens = lex_expr("true false")?;
    let expected = vec![
        Token::Literal(LiteralToken::Bool(true)),
        Token::Literal(LiteralToken::Bool(false)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lex_expr_returns_error_for_unrecognized_character() -> crate::ExprResult<()> {
    let result = lex_expr("#");
    let Err(ExprError::UnexpectedChar { ch }) = result else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected UnexpectedChar".into(),
        });
    };
    assert_eq!(ch, '#');
    Ok(())
}

#[test]
fn lex_expr_tokenizes_comparison_operators() -> crate::ExprResult<()> {
    let tokens = lex_expr("== != < <= > >=")?;
    let expected = vec![
        Token::Operator(BinaryOp::Eq),
        Token::Operator(BinaryOp::NotEq),
        Token::Operator(BinaryOp::Lt),
        Token::Operator(BinaryOp::Lte),
        Token::Operator(BinaryOp::Gt),
        Token::Operator(BinaryOp::Gte),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lex_expr_tokenizes_null_literal() -> crate::ExprResult<()> {
    let tokens = lex_expr("null")?;
    let expected = vec![Token::Literal(LiteralToken::Null), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lex_expr_tokenizes_not_keyword() -> crate::ExprResult<()> {
    let tokens = lex_expr("not")?;
    let expected = vec![Token::Unary(UnaryOp::Not), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

// --- F64 literal lexer tests ---

#[test]
fn lexes_float_literal() -> crate::ExprResult<()> {
    let tokens = lex_expr("3.14")?;
    let expected = vec![
        Token::Literal(LiteralToken::F64(vb_core::FiniteF64::new(3.14)?)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_float_literal_with_leading_zero() -> crate::ExprResult<()> {
    let tokens = lex_expr("0.5")?;
    let expected = vec![
        Token::Literal(LiteralToken::F64(vb_core::FiniteF64::new(0.5)?)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_float_literal_large_value() -> crate::ExprResult<()> {
    let tokens = lex_expr("123.456")?;
    let expected = vec![
        Token::Literal(LiteralToken::F64(vb_core::FiniteF64::new(123.456)?)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_float_literal_in_expression() -> crate::ExprResult<()> {
    let tokens = lex_expr("1.5 + 2.5")?;
    let expected = vec![
        Token::Literal(LiteralToken::F64(vb_core::FiniteF64::new(1.5)?)),
        Token::Operator(BinaryOp::Add),
        Token::Literal(LiteralToken::F64(vb_core::FiniteF64::new(2.5)?)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}
