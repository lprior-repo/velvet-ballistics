//! Expression tokenizer producing bounded token streams.

use crate::{ExprError, ExprResult};

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
    if input.len() > MAX_SOURCE_BYTES {
        return Err(ExprError::ExpressionTooLong {
            len: input.len(),
            max: MAX_SOURCE_BYTES,
        });
    }
    let mut lexer = Lexer::new(input);
    lexer.lex_all()
}

struct Lexer<'a> {
    source: &'a str,
    index: usize,
    tokens: Vec<Token>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            index: 0,
            tokens: Vec::new(),
        }
    }

    fn lex_all(&mut self) -> ExprResult<Vec<Token>> {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.bump_char(ch);
            } else {
                self.lex_one(ch)?;
            }
        }
        self.push_token(Token::End)?;
        Ok(core::mem::take(&mut self.tokens))
    }

    fn lex_one(&mut self, ch: char) -> ExprResult<()> {
        if ch.is_ascii_digit() {
            self.lex_integer()
        } else if is_ident_start(ch) {
            self.lex_ident()
        } else {
            self.lex_symbol(ch)
        }
    }

    fn lex_integer(&mut self) -> ExprResult<()> {
        let start = self.index;
        while self.peek_char().is_some_and(|ch: char| ch.is_ascii_digit()) {
            self.bump_current();
        }
        let text = self.slice(start, self.index)?;
        let value = text
            .parse::<i64>()
            .map_err(|_| ExprError::IntegerOutOfRange)?;
        self.push_token(Token::Literal(LiteralToken::I64(value)))
    }

    fn lex_ident(&mut self) -> ExprResult<()> {
        let start = self.index;
        while self.peek_char().is_some_and(is_ident_continue) {
            self.bump_current();
        }
        let text = self.slice(start, self.index)?;
        let token = classify_ident(text);
        self.push_token(token)
    }

    fn lex_symbol(&mut self, ch: char) -> ExprResult<()> {
        match ch {
            '$' => self.lex_reference(),
            '"' => self.lex_string(),
            '(' => self.single_token(Token::LParen, ch),
            ')' => self.single_token(Token::RParen, ch),
            ',' => self.single_token(Token::Comma, ch),
            '!' | '=' | '<' | '>' => self.lex_compound_operator(ch),
            '+' | '-' | '*' | '/' => self.lex_arithmetic_operator(ch),
            _ => Err(ExprError::UnexpectedChar { ch }),
        }
    }

    fn lex_reference(&mut self) -> ExprResult<()> {
        let start = self.index;
        self.bump_current();
        let body_start = self.index;
        while self.peek_char().is_some_and(is_reference_continue) {
            self.bump_current();
        }
        if self.index == body_start {
            return self.push_token(Token::Dollar);
        }
        let reference = self.slice(start, self.index)?;
        self.push_token(Token::Reference(Box::from(reference)))
    }

    fn lex_string(&mut self) -> ExprResult<()> {
        self.bump_current();
        let value_start = self.index;
        while let Some(ch) = self.peek_char() {
            if ch == '"' {
                let value = Box::from(self.slice(value_start, self.index)?);
                self.bump_current();
                return self.push_token(Token::Literal(LiteralToken::Text(value)));
            }
            self.bump_char(ch);
        }
        Err(ExprError::UnterminatedString)
    }

    fn lex_compound_operator(&mut self, ch: char) -> ExprResult<()> {
        self.bump_current();
        let next = self.peek_char();
        match (ch, next) {
            ('!', Some('=')) => {
                self.bump_current();
                self.push_token(Token::Operator(BinaryOp::NotEq))
            }
            ('=', Some('=')) => {
                self.bump_current();
                self.push_token(Token::Operator(BinaryOp::Eq))
            }
            ('<', Some('=')) => {
                self.bump_current();
                self.push_token(Token::Operator(BinaryOp::Lte))
            }
            ('>', Some('=')) => {
                self.bump_current();
                self.push_token(Token::Operator(BinaryOp::Gte))
            }
            ('<', _) => self.push_token(Token::Operator(BinaryOp::Lt)),
            ('>', _) => self.push_token(Token::Operator(BinaryOp::Gt)),
            _ => Err(ExprError::UnexpectedChar { ch }),
        }
    }

    fn lex_arithmetic_operator(&mut self, ch: char) -> ExprResult<()> {
        self.bump_current();
        let token = match ch {
            '+' => Token::Operator(BinaryOp::Add),
            '-' => Token::Operator(BinaryOp::Sub),
            '*' => Token::Operator(BinaryOp::Mul),
            '/' => Token::Operator(BinaryOp::Div),
            _ => return Err(ExprError::UnexpectedChar { ch }),
        };
        self.push_token(token)
    }

    fn single_token(&mut self, token: Token, ch: char) -> ExprResult<()> {
        self.bump_char(ch);
        self.push_token(token)
    }

    fn push_token(&mut self, token: Token) -> ExprResult<()> {
        if self.tokens.len() >= MAX_TOKENS {
            return Err(ExprError::ExpressionTooLong {
                len: self.tokens.len().saturating_add(1),
                max: MAX_TOKENS,
            });
        }
        self.tokens.push(token);
        Ok(())
    }

    fn peek_char(&self) -> Option<char> {
        self.source.get(self.index..).and_then(|s| s.chars().next())
    }

    fn bump_current(&mut self) {
        if let Some(ch) = self.peek_char() {
            self.bump_char(ch);
        }
    }

    fn bump_char(&mut self, ch: char) {
        self.index = self.index.saturating_add(ch.len_utf8());
    }

    fn slice(&self, start: usize, end: usize) -> ExprResult<&str> {
        self.source.get(start..end).ok_or(ExprError::UnexpectedEof)
    }
}

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

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_reference_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')
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
}
