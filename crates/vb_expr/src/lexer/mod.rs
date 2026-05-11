#![forbid(unsafe_code)]
//! Expression tokenizer producing bounded token streams.
//!
//! Uses [`logos`] to generate a finite-state lexer, then converts the raw
//! `LogosToken` stream into the public [`Token`] type while enforcing
//! source-length and token-count bounds.

use crate::{ExprError, ExprResult};
use logos::Logos;

pub use types::{BinaryOp, LiteralToken, SpannedToken, Token, TokenSpan, UnaryOp};

pub mod tests;
pub mod types;
pub mod miri_tests;

/// Maximum source bytes accepted by the lexer.
const MAX_SOURCE_BYTES: usize = 4096;
/// Maximum tokens per expression (excluding the End token).
const MAX_TOKENS: usize = 256;

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
        if let Ok(logos_tok) = result {
            let tok = convert_logos_token(logos_tok, lexer.slice())?;
            push_spanned_token(&mut tokens, tok, span.start, span.end)?;
        } else {
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

    // --- Floating-point literals ---
    #[regex(r"[0-9]+\.[0-9]+")]
    Float,

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
        LogosToken::Float => {
            let value = slice
                .parse::<f64>()
                .map_err(|_| ExprError::NonFiniteFloat)?;
            let finite = vb_core::FiniteF64::new(value).map_err(|_| ExprError::NonFiniteFloat)?;
            Ok(Token::Literal(LiteralToken::F64(finite)))
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
