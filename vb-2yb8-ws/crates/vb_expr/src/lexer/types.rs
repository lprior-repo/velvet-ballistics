#![forbid(unsafe_code)]
//! Expression token types for vb_expr.

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
