#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports, clippy::approximate_const, clippy::absurd_extreme_comparisons, clippy::expect_fun_call)]

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

// --- Comma token ---

#[test]
fn lexes_comma_token_between_integers() -> crate::ExprResult<()> {
    let tokens = lex_expr("1, 2")?;
    let expected = vec![
        Token::Literal(LiteralToken::I64(1)),
        Token::Comma,
        Token::Literal(LiteralToken::I64(2)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_comma_separated_multiple_values() -> crate::ExprResult<()> {
    let tokens = lex_expr("1, 2, 3")?;
    let expected = vec![
        Token::Literal(LiteralToken::I64(1)),
        Token::Comma,
        Token::Literal(LiteralToken::I64(2)),
        Token::Comma,
        Token::Literal(LiteralToken::I64(3)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

// --- Paren tokens individually ---

#[test]
fn lexes_lparen_token_alone() -> crate::ExprResult<()> {
    let tokens = lex_expr("(")?;
    let expected = vec![Token::LParen, Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_rparen_token_alone() -> crate::ExprResult<()> {
    let tokens = lex_expr(")")?;
    let expected = vec![Token::RParen, Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

// --- Integer boundaries ---

#[test]
fn lexes_integer_zero() -> crate::ExprResult<()> {
    let tokens = lex_expr("0")?;
    let expected = vec![Token::Literal(LiteralToken::I64(0)), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_integer_with_leading_zeros() -> crate::ExprResult<()> {
    let tokens = lex_expr("00042")?;
    let expected = vec![Token::Literal(LiteralToken::I64(42)), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_negative_integer_as_minus_operator_then_integer() -> crate::ExprResult<()> {
    let tokens = lex_expr("-5")?;
    let expected = vec![
        Token::Operator(BinaryOp::Sub),
        Token::Literal(LiteralToken::I64(5)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

// --- Float boundaries ---

#[test]
fn lexes_float_zero_point_zero() -> crate::ExprResult<()> {
    let tokens = lex_expr("0.0")?;
    let expected = vec![
        Token::Literal(LiteralToken::F64(vb_core::FiniteF64::new(0.0)?)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

// --- String boundaries ---

#[test]
fn lexes_empty_string_literal() -> crate::ExprResult<()> {
    let tokens = lex_expr("\"\"")?;
    let expected = vec![
        Token::Literal(LiteralToken::Text(Box::from(""))),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_string_literal_with_special_characters_inside() -> crate::ExprResult<()> {
    let tokens = lex_expr("\"hello, world! $ref @test\"")?;
    let expected = vec![
        Token::Literal(LiteralToken::Text(Box::from("hello, world! $ref @test"))),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

// --- Identifiers ---

#[test]
fn lexes_single_char_identifier() -> crate::ExprResult<()> {
    let tokens = lex_expr("x")?;
    let expected = vec![Token::Identifier(Box::from("x")), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_identifier_with_numbers() -> crate::ExprResult<()> {
    let tokens = lex_expr("abc123def")?;
    let expected = vec![Token::Identifier(Box::from("abc123def")), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_identifier_starting_with_underscore() -> crate::ExprResult<()> {
    let tokens = lex_expr("_private")?;
    let expected = vec![Token::Identifier(Box::from("_private")), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_identifier_that_looks_like_keyword() -> crate::ExprResult<()> {
    let tokens = lex_expr("truex falsely nullify android oracle noted")?;
    let expected = vec![
        Token::Identifier(Box::from("truex")),
        Token::Identifier(Box::from("falsely")),
        Token::Identifier(Box::from("nullify")),
        Token::Identifier(Box::from("android")),
        Token::Identifier(Box::from("oracle")),
        Token::Identifier(Box::from("noted")),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

// --- Keywords in expression context ---

#[test]
fn lexes_not_keyword_before_boolean() -> crate::ExprResult<()> {
    let tokens = lex_expr("not true")?;
    let expected = vec![
        Token::Unary(UnaryOp::Not),
        Token::Literal(LiteralToken::Bool(true)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_and_in_boolean_expression() -> crate::ExprResult<()> {
    let tokens = lex_expr("true and false")?;
    let expected = vec![
        Token::Literal(LiteralToken::Bool(true)),
        Token::Operator(BinaryOp::And),
        Token::Literal(LiteralToken::Bool(false)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_or_in_boolean_expression() -> crate::ExprResult<()> {
    let tokens = lex_expr("true or false")?;
    let expected = vec![
        Token::Literal(LiteralToken::Bool(true)),
        Token::Operator(BinaryOp::Or),
        Token::Literal(LiteralToken::Bool(false)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

// --- Dollar token contexts ---

#[test]
fn lexes_dollar_token_between_integers() -> crate::ExprResult<()> {
    let tokens = lex_expr("1 $ 2")?;
    let expected = vec![
        Token::Literal(LiteralToken::I64(1)),
        Token::Dollar,
        Token::Literal(LiteralToken::I64(2)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_dollar_followed_by_operator() -> crate::ExprResult<()> {
    let tokens = lex_expr("$ + 1")?;
    let expected = vec![
        Token::Dollar,
        Token::Operator(BinaryOp::Add),
        Token::Literal(LiteralToken::I64(1)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

// --- Reference variants ---

#[test]
fn lexes_reference_single_letter() -> crate::ExprResult<()> {
    let tokens = lex_expr("$x")?;
    let expected = vec![Token::Reference(Box::from("$x")), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_reference_with_underscores_and_numbers() -> crate::ExprResult<()> {
    let tokens = lex_expr("$my_var_123")?;
    let expected = vec![Token::Reference(Box::from("$my_var_123")), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_reference_with_multiple_dots() -> crate::ExprResult<()> {
    let tokens = lex_expr("$a.b.c.d")?;
    let expected = vec![Token::Reference(Box::from("$a.b.c.d")), Token::End];
    assert_eq!(tokens, expected);
    Ok(())
}

// --- Complex expressions ---

#[test]
fn lexes_complex_expression_with_nesting() -> crate::ExprResult<()> {
    let tokens = lex_expr("$x > 0 and (y < 10 or not z)")?;
    let expected = vec![
        Token::Reference(Box::from("$x")),
        Token::Operator(BinaryOp::Gt),
        Token::Literal(LiteralToken::I64(0)),
        Token::Operator(BinaryOp::And),
        Token::LParen,
        Token::Identifier(Box::from("y")),
        Token::Operator(BinaryOp::Lt),
        Token::Literal(LiteralToken::I64(10)),
        Token::Operator(BinaryOp::Or),
        Token::Unary(UnaryOp::Not),
        Token::Identifier(Box::from("z")),
        Token::RParen,
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

#[test]
fn lexes_operators_without_whitespace() -> crate::ExprResult<()> {
    let tokens = lex_expr("1+2")?;
    let expected = vec![
        Token::Literal(LiteralToken::I64(1)),
        Token::Operator(BinaryOp::Add),
        Token::Literal(LiteralToken::I64(2)),
        Token::End,
    ];
    assert_eq!(tokens, expected);
    Ok(())
}

// --- Spanned tokens ---

#[test]
fn lex_expr_spanned_end_token_has_zero_width_span() -> crate::ExprResult<()> {
    let tokens = lex_expr_spanned("42")?;
    let end = tokens.last().ok_or(ExprError::UnexpectedEof)?;
    assert_eq!(end.token, Token::End);
    assert_eq!(end.span.start, end.span.end);
    assert_eq!(end.span.start, 2);
    Ok(())
}

#[test]
fn lex_expr_spanned_covers_multiple_token_spans() -> crate::ExprResult<()> {
    let tokens = lex_expr_spanned("1 + 2")?;
    let expected = vec![
        SpannedToken {
            token: Token::Literal(LiteralToken::I64(1)),
            span: TokenSpan { start: 0, end: 1 },
        },
        SpannedToken {
            token: Token::Operator(BinaryOp::Add),
            span: TokenSpan { start: 2, end: 3 },
        },
        SpannedToken {
            token: Token::Literal(LiteralToken::I64(2)),
            span: TokenSpan { start: 4, end: 5 },
        },
        SpannedToken {
            token: Token::End,
            span: TokenSpan { start: 5, end: 5 },
        },
    ];
    assert_eq!(tokens, expected);
    Ok(())
}
