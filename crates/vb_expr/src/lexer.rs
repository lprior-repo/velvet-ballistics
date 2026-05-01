//! Expression tokenizer producing bounded token streams.
//!
//! Uses [`logos`] to generate a finite-state lexer, then converts the raw
//! `LogosToken` stream into the public [`Token`] type while enforcing
//! source-length and token-count bounds.

use crate::{ExprError, ExprResult};
use logos::Logos;

/// Maximum source bytes accepted by the lexer.
const MAX_SOURCE_BYTES: usize = 4096;
/// Maximum tokens per expression (excluding the End token).
const MAX_TOKENS: usize = 256;

/// Expression token produced by the lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// Null, boolean, or integer literal.
    Literal(LiteralToken),
    /// Identifier (keywords like `true`, `false`, `null`, `and`, `or`, `not`
    /// are emitted as their own operator/unary variants, not as identifiers).
    Identifier(Box<str>),
    /// Binary operator.
    Operator(BinaryOp),
    /// Unary operator (logical not, numeric negation).
    Unary(UnaryOp),
    /// Source reference starting with `$`.
    Reference(Box<str>),
    /// Left parenthesis.
    LParen,
    /// Right parenthesis.
    RParen,
    /// Comma separator.
    Comma,
    /// Dollar sign without a valid identifier body.
    Dollar,
    /// End-of-input sentinel.
    End,
}

/// Byte span for a token in the expression source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenSpan {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

/// Token plus exact source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedToken {
    /// Public token.
    pub token: Token,
    /// Source byte span.
    pub span: TokenSpan,
}

/// Literal value token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralToken {
    /// Null literal.
    Null,
    /// Boolean literal.
    Bool(bool),
    /// Signed 64-bit integer literal.
    I64(i64),
    /// Double-quoted string literal.
    Text(Box<str>),
}

/// Left-associative infix binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// Logical OR.
    Or,
    /// Logical AND.
    And,
    /// Equality comparison.
    Eq,
    /// Inequality comparison.
    NotEq,
    /// Less-than comparison.
    Lt,
    /// Less-than-or-equal comparison.
    Lte,
    /// Greater-than comparison.
    Gt,
    /// Greater-than-or-equal comparison.
    Gte,
    /// Addition.
    Add,
    /// Subtraction.
    Sub,
    /// Multiplication.
    Mul,
    /// Division.
    Div,
}

/// Prefix unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// Logical negation.
    Not,
    /// Numeric negation.
    Neg,
}

/// Tokenizes an expression source string into a bounded token vector.
pub fn lex_expr(input: &str) -> ExprResult<Vec<Token>> {
    let spanned = lex_expr_spanned(input)?;
    let mut tokens = Vec::with_capacity(spanned.len());
    for token in spanned {
        tokens.push(token.token);
    }
    Ok(tokens)
}

/// Tokenizes an expression source string into bounded tokens with byte spans.
pub fn lex_expr_spanned(input: &str) -> ExprResult<Vec<SpannedToken>> {
    if input.len() > MAX_SOURCE_BYTES {
        return Err(ExprError::ExpressionTooLong {
            len: input.len(),
            max: MAX_SOURCE_BYTES,
        });
    }

    let mut tokens = Vec::new();
    let mut lexer = LogosToken::lexer(input);

    while let Some(result) = lexer.next() {
        let span = lexer.span();
        match result {
            Ok(logos_tok) => {
                let tok = convert_logos_token(logos_tok, lexer.slice())?;
                push_spanned_token(&mut tokens, tok, span.start, span.end)?;
            }
            Err(()) => {
                // Logos reports an error for unrecognized input.
                // Check for unterminated string (opening " with no closing quote).
                let slice = lexer.slice();
                if slice.starts_with('"') {
                    return Err(ExprError::UnterminatedString);
                }
                let ch = slice.chars().next().ok_or(ExprError::UnexpectedEof)?;
                return Err(ExprError::UnexpectedChar { ch });
            }
        }
    }

    push_spanned_token(&mut tokens, Token::End, input.len(), input.len())?;
    Ok(tokens)
}

// ---------------------------------------------------------------------------
// Internal logos token enum
// ---------------------------------------------------------------------------

/// Internal token type produced by the logos-generated state machine.
///
/// Logos handles whitespace skipping, operator disambiguation, and basic
/// pattern matching. Keyword classification and literal parsing happen in
/// [`convert_logos_token`].
#[derive(Debug, Clone, Logos, PartialEq, Eq)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(error = ())]
enum LogosToken {
    // --- Operators ---
    #[token("==")]
    EqEq,
    #[token("!=")]
    BangEq,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,

    // --- Delimiters ---
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token(",")]
    Comma,

    // --- References: $identifier[.identifier]* ---
    #[regex(r"\$[a-zA-Z0-9_]+(?:\.[a-zA-Z0-9_]+)*")]
    Reference,

    /// Lone dollar sign (not followed by a valid reference body).
    #[token("$")]
    Dollar,

    // --- String literals (terminated) ---
    #[regex(r#""[^"]*""#)]
    StringLiteral,

    /// Opening double-quote that is NOT part of a terminated string literal.
    #[token("\"")]
    UnterminatedString,

    // --- Integers ---
    #[regex(r"[0-9]+")]
    Integer,

    // --- Identifiers / keywords ---
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Ident,
}

/// Converts a [`LogosToken`] and its source slice into a public [`Token`].
fn convert_logos_token(tok: LogosToken, slice: &str) -> ExprResult<Token> {
    match tok {
        LogosToken::EqEq => Ok(Token::Operator(BinaryOp::Eq)),
        LogosToken::BangEq => Ok(Token::Operator(BinaryOp::NotEq)),
        LogosToken::LtEq => Ok(Token::Operator(BinaryOp::Lte)),
        LogosToken::GtEq => Ok(Token::Operator(BinaryOp::Gte)),
        LogosToken::Lt => Ok(Token::Operator(BinaryOp::Lt)),
        LogosToken::Gt => Ok(Token::Operator(BinaryOp::Gt)),
        LogosToken::Plus => Ok(Token::Operator(BinaryOp::Add)),
        LogosToken::Minus => Ok(Token::Operator(BinaryOp::Sub)),
        LogosToken::Star => Ok(Token::Operator(BinaryOp::Mul)),
        LogosToken::Slash => Ok(Token::Operator(BinaryOp::Div)),
        LogosToken::LParen => Ok(Token::LParen),
        LogosToken::RParen => Ok(Token::RParen),
        LogosToken::Comma => Ok(Token::Comma),
        LogosToken::Dollar => Ok(Token::Dollar),
        LogosToken::Reference => Ok(Token::Reference(Box::from(slice))),
        LogosToken::StringLiteral => {
            // Slice includes the surrounding quotes. Validate termination
            // (guaranteed by the regex) then strip delimiters.
            let inner = strip_quotes(slice)?;
            Ok(Token::Literal(LiteralToken::Text(Box::from(inner))))
        }
        LogosToken::UnterminatedString => Err(ExprError::UnterminatedString),
        LogosToken::Integer => {
            let value = slice
                .parse::<i64>()
                .map_err(|_| ExprError::IntegerOutOfRange)?;
            Ok(Token::Literal(LiteralToken::I64(value)))
        }
        LogosToken::Ident => Ok(classify_ident(slice)),
    }
}

/// Classifies an identifier: keywords become operators/literals, everything
/// else becomes [`Token::Identifier`].
fn classify_ident(text: &str) -> Token {
    match text {
        "true" => Token::Literal(LiteralToken::Bool(true)),
        "false" => Token::Literal(LiteralToken::Bool(false)),
        "null" => Token::Literal(LiteralToken::Null),
        "and" => Token::Operator(BinaryOp::And),
        "or" => Token::Operator(BinaryOp::Or),
        "not" => Token::Unary(UnaryOp::Not),
        _ => Token::Identifier(Box::from(text)),
    }
}

/// Strips the leading and trailing double-quote from a string literal.
///
/// The caller guarantees `s` starts and ends with `"`.
fn strip_quotes(s: &str) -> ExprResult<&str> {
    // Length is at least 2 (opening + closing quote).
    let end = s.len().saturating_sub(1);
    s.get(1..end).ok_or(ExprError::UnterminatedString)
}

fn push_spanned_token(
    tokens: &mut Vec<SpannedToken>,
    token: Token,
    start: usize,
    end: usize,
) -> ExprResult<()> {
    if token != Token::End && tokens.len() >= MAX_TOKENS {
        return Err(ExprError::ExpressionTooLong {
            len: tokens.len().saturating_add(1),
            max: MAX_TOKENS,
        });
    }
    tokens.push(SpannedToken {
        token,
        span: TokenSpan { start, end },
    });
    Ok(())
}

/// Returns the infix binding power for a binary operator.
///
/// Returns `(left_bp, right_bp)` where higher values bind tighter.
pub fn infix_binding_power(op: BinaryOp) -> (u8, u8) {
    match op {
        BinaryOp::Or => (1, 2),
        BinaryOp::And => (3, 4),
        BinaryOp::Eq | BinaryOp::NotEq => (5, 6),
        BinaryOp::Lt | BinaryOp::Lte | BinaryOp::Gt | BinaryOp::Gte => (7, 8),
        BinaryOp::Add | BinaryOp::Sub => (9, 10),
        BinaryOp::Mul | BinaryOp::Div => (11, 12),
    }
}

#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
mod tests {
    use super::*;

    #[test]
    fn lexes_integer_literal() -> ExprResult<()> {
        let tokens = lex_expr("42")?;
        let expected = vec![Token::Literal(LiteralToken::I64(42)), Token::End];
        assert_eq!(tokens, expected);
        Ok(())
    }

    #[test]
    fn lexes_boolean_and_null_literals() -> ExprResult<()> {
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
    fn lexes_string_literal() -> ExprResult<()> {
        let tokens = lex_expr("\"hello\"")?;
        let expected = vec![
            Token::Literal(LiteralToken::Text(Box::from("hello"))),
            Token::End,
        ];
        assert_eq!(tokens, expected);
        Ok(())
    }

    #[test]
    fn lexes_reference() -> ExprResult<()> {
        let tokens = lex_expr("$input.value")?;
        let expected = vec![Token::Reference(Box::from("$input.value")), Token::End];
        assert_eq!(tokens, expected);
        Ok(())
    }

    #[test]
    fn lexes_operators() -> ExprResult<()> {
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
    fn lexes_keywords() -> ExprResult<()> {
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
    fn lexes_helper_identifiers() -> ExprResult<()> {
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
    fn lex_expr_spanned_preserves_exact_byte_spans() -> ExprResult<()> {
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
    fn lex_expr_accepts_max_tokens_plus_end_sentinel() -> ExprResult<()> {
        let source = "1 ".repeat(MAX_TOKENS);
        let tokens = lex_expr(&source)?;
        let last = tokens.last().ok_or(ExprError::UnexpectedEof)?;
        assert_eq!(tokens.len(), MAX_TOKENS.saturating_add(1));
        assert_eq!(last, &Token::End);
        Ok(())
    }

    #[test]
    fn rejects_token_limit() {
        let source = "1 + ".repeat(MAX_TOKENS);
        let result = lex_expr(&source);
        assert!(matches!(result, Err(ExprError::ExpressionTooLong { .. })));
    }

    #[test]
    fn rejects_source_length_limit() {
        let source = "1".repeat(MAX_SOURCE_BYTES.saturating_add(1));
        let result = lex_expr(&source);
        assert!(matches!(result, Err(ExprError::ExpressionTooLong { .. })));
    }

    #[test]
    fn rejects_unterminated_string() {
        let result = lex_expr("\"unterminated");
        assert!(matches!(result, Err(ExprError::UnterminatedString)));
    }

    #[test]
    fn lone_dollar_produces_dollar_token() -> ExprResult<()> {
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
    fn lex_expr_tokenizes_addition_expression() -> ExprResult<()> {
        // Given: the expression "3 + 5"
        // When: lex_expr is called
        // Then: the token sequence is [I64(3), Add, I64(5), End]
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
    fn lex_expr_tokenizes_subtraction_expression() -> ExprResult<()> {
        // Given: the expression "10 - 4"
        // When: lex_expr is called
        // Then: the token sequence is [I64(10), Sub, I64(4), End]
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
    fn lex_expr_tokenizes_multiplication_expression() -> ExprResult<()> {
        // Given: the expression "6 * 7"
        // When: lex_expr is called
        // Then: the token sequence is [I64(6), Mul, I64(7), End]
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
    fn lex_expr_tokenizes_division_expression() -> ExprResult<()> {
        // Given: the expression "20 / 4"
        // When: lex_expr is called
        // Then: the token sequence is [I64(20), Div, I64(4), End]
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
    fn lex_expr_tokenizes_parenthesized_expression() -> ExprResult<()> {
        // Given: the expression "(1 + 2)"
        // When: lex_expr is called
        // Then: the token sequence includes LParen and RParen tokens
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
    fn lex_expr_tokenizes_string_literal() -> ExprResult<()> {
        // Given: the expression "\"hello world\""
        // When: lex_expr is called
        // Then: the token is Text("hello world")
        let tokens = lex_expr("\"hello world\"")?;
        let expected = vec![
            Token::Literal(LiteralToken::Text(Box::from("hello world"))),
            Token::End,
        ];
        assert_eq!(tokens, expected);
        Ok(())
    }

    #[test]
    fn lex_expr_tokenizes_variable_reference() -> ExprResult<()> {
        // Given: the expression "$my_var"
        // When: lex_expr is called
        // Then: the token is Reference("$my_var")
        let tokens = lex_expr("$my_var")?;
        let expected = vec![Token::Reference(Box::from("$my_var")), Token::End];
        assert_eq!(tokens, expected);
        Ok(())
    }

    #[test]
    fn lex_expr_tokenizes_boolean_literals() -> ExprResult<()> {
        // Given: the expression "true false"
        // When: lex_expr is called
        // Then: tokens are [Bool(true), Bool(false), End]
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
    fn lex_expr_returns_error_for_unrecognized_character() -> ExprResult<()> {
        // Given: the expression "#"
        // When: lex_expr is called
        // Then: the result is Err(UnexpectedChar { ch: '#' })
        let result = lex_expr("#");
        let Err(ExprError::UnexpectedChar { ch }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected UnexpectedChar".into(),
            });
        };
        assert_eq!(ch, '#');
        Ok(())
    }

    #[test]
    fn lex_expr_tokenizes_comparison_operators() -> ExprResult<()> {
        // Given: the expression "== != < <= > >="
        // When: lex_expr is called
        // Then: all six comparison operator tokens are produced
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
    fn lex_expr_tokenizes_null_literal() -> ExprResult<()> {
        // Given: the expression "null"
        // When: lex_expr is called
        // Then: the token is Literal(Null)
        let tokens = lex_expr("null")?;
        let expected = vec![Token::Literal(LiteralToken::Null), Token::End];
        assert_eq!(tokens, expected);
        Ok(())
    }

    #[test]
    fn lex_expr_tokenizes_not_keyword() -> ExprResult<()> {
        // Given: the expression "not"
        // When: lex_expr is called
        // Then: the token is Unary(Not)
        let tokens = lex_expr("not")?;
        let expected = vec![Token::Unary(UnaryOp::Not), Token::End];
        assert_eq!(tokens, expected);
        Ok(())
    }

    // --- Adversarial BDD tests ---

    #[test]
    fn lex_expr_rejects_empty_string_as_only_end_token() -> ExprResult<()> {
        // Given: the empty expression ""
        // When: lex_expr is called
        // Then: the result is a single End token
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
    fn lex_expr_rejects_whitespace_only_input_as_only_end_token() -> ExprResult<()> {
        // Given: the expression "   \t\n  "
        // When: lex_expr is called
        // Then: the result is a single End token (whitespace is consumed)
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
    fn lex_expr_rejects_unexpected_unicode_character() -> ExprResult<()> {
        // Given: the expression "\u{00F7}" (division sign, looks like /)
        // When: lex_expr is called
        // Then: the result is Err(UnexpectedChar { ch })
        let result = lex_expr("\u{00F7}");
        let Err(ExprError::UnexpectedChar { ch }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected UnexpectedChar for unicode division sign".into(),
            });
        };
        assert_eq!(ch, '\u{00F7}');
        Ok(())
    }

    #[test]
    fn lex_expr_rejects_unexpected_at_sign() -> ExprResult<()> {
        // Given: the expression "@"
        // When: lex_expr is called
        // Then: the result is Err(UnexpectedChar { ch: '@' })
        let result = lex_expr("@");
        let Err(ExprError::UnexpectedChar { ch }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected UnexpectedChar for @".into(),
            });
        };
        assert_eq!(ch, '@');
        Ok(())
    }

    #[test]
    fn lex_expr_handles_max_i64_literal() -> ExprResult<()> {
        // Given: the expression "9223372036854775807" (i64::MAX)
        // When: lex_expr is called
        // Then: the token is I64(9223372036854775807)
        let tokens = lex_expr("9223372036854775807")?;
        let expected = vec![Token::Literal(LiteralToken::I64(i64::MAX)), Token::End];
        assert_eq!(tokens, expected);
        Ok(())
    }

    #[test]
    fn lex_expr_rejects_i64_overflow_literal() -> ExprResult<()> {
        // Given: the expression "9223372036854775808" (i64::MAX + 1)
        // When: lex_expr is called
        // Then: the result is Err(IntegerOutOfRange)
        let result = lex_expr("9223372036854775808");
        assert!(
            matches!(result, Err(ExprError::IntegerOutOfRange)),
            "expected IntegerOutOfRange for value exceeding i64::MAX"
        );
        Ok(())
    }

    #[test]
    fn lex_expr_tokenizes_deeply_nested_parentheses() -> ExprResult<()> {
        // Given: the expression "((((1))))"
        // When: lex_expr is called
        // Then: all parentheses and the literal are correctly tokenized
        let tokens = lex_expr("((((1))))")?;
        assert_eq!(tokens.first(), Some(&Token::LParen));
        assert_eq!(tokens.last(), Some(&Token::End));
        let rparen_count = tokens.iter().filter(|t| matches!(t, Token::RParen)).count();
        assert_eq!(rparen_count, 4);
        Ok(())
    }

    #[test]
    fn lex_expr_lone_dollar_after_whitespace_is_dollar_token() -> ExprResult<()> {
        // Given: the expression "$ + 1" where $ is standalone
        // When: lex_expr is called
        // Then: the first token is Dollar (not a reference)
        let tokens = lex_expr("$ + 1")?;
        assert_eq!(tokens.first(), Some(&Token::Dollar));
        Ok(())
    }

    #[test]
    fn lex_expr_rejects_bare_exclamation_mark() -> ExprResult<()> {
        // Given: the expression "!"
        // When: lex_expr is called
        // Then: the result is Err(UnexpectedChar { ch: '!' })
        let result = lex_expr("!");
        let Err(ExprError::UnexpectedChar { ch }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected UnexpectedChar for bare !".into(),
            });
        };
        assert_eq!(ch, '!');
        Ok(())
    }

    #[test]
    fn lex_expr_rejects_bare_equals_sign() -> ExprResult<()> {
        // Given: the expression "="
        // When: lex_expr is called
        // Then: the result is Err(UnexpectedChar { ch: '=' })
        let result = lex_expr("=");
        let Err(ExprError::UnexpectedChar { ch }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected UnexpectedChar for bare =".into(),
            });
        };
        assert_eq!(ch, '=');
        Ok(())
    }

    #[test]
    fn lex_expr_handles_string_with_spaces() -> ExprResult<()> {
        // Given: the expression "\"a b c\""
        // When: lex_expr is called
        // Then: the token is Text("a b c")
        let tokens = lex_expr("\"a b c\"")?;
        let expected = vec![
            Token::Literal(LiteralToken::Text(Box::from("a b c"))),
            Token::End,
        ];
        assert_eq!(tokens, expected);
        Ok(())
    }

    #[test]
    fn lex_expr_rejects_unterminated_string_immediately() -> ExprResult<()> {
        // Given: the expression "\""
        // When: lex_expr is called
        // Then: the result is Err(UnterminatedString)
        let result = lex_expr("\"");
        assert!(matches!(result, Err(ExprError::UnterminatedString)));
        Ok(())
    }

    #[test]
    fn lex_expr_reference_with_dots_allows_path_access() -> ExprResult<()> {
        // Given: the expression "$input.field1.field2.field3"
        // When: lex_expr is called
        // Then: the token is Reference("$input.field1.field2.field3")
        let tokens = lex_expr("$input.field1.field2.field3")?;
        let expected = vec![
            Token::Reference(Box::from("$input.field1.field2.field3")),
            Token::End,
        ];
        assert_eq!(tokens, expected);
        Ok(())
    }
}
