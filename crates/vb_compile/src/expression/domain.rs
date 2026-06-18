#![forbid(unsafe_code)]
//! Domain types for the cold expression grammar.

use vb_core::FiniteF64;

/// Parsed v1 expression tree retained by the cold compiler AST.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParsedExpression {
    /// Null, boolean, integer, or string literal.
    Literal(ExpressionLiteral),
    /// Source reference beginning with `$`.
    Reference(Box<str>),
    /// Prefix unary expression.
    Unary {
        /// Unary operator.
        op: UnaryOp,
        /// Operand expression.
        expr: Box<ParsedExpression>,
    },
    /// Infix binary expression.
    Binary {
        /// Binary operator.
        op: BinaryOp,
        /// Left operand.
        left: Box<ParsedExpression>,
        /// Right operand.
        right: Box<ParsedExpression>,
    },
    /// Built-in helper call retained for later bytecode lowering.
    HelperCall {
        /// Helper name.
        name: ExpressionHelper,
        /// Parsed helper arguments.
        args: Box<[ParsedExpression]>,
    },
}

/// Built-in helper accepted by the v1 expression grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExpressionHelper {
    /// `contains(value, needle)`.
    Contains,
    /// `starts_with(value, prefix)`.
    StartsWith,
    /// `ends_with(value, suffix)`.
    EndsWith,
    /// `has(object, key)`.
    Has,
    /// `exists(value)`.
    Exists,
    /// `length(value)`.
    Length,
    /// `empty(value)`.
    Empty,
    /// `append(list, value)`.
    Append,
    /// `append_if(list, value, condition)`.
    AppendIf,
    /// `merge(left, right)`.
    Merge,
    /// `sum(list)`.
    Sum,
    /// `count(list)`.
    Count,
    /// `unique(list)`.
    Unique,
    /// `coalesce(value, fallback)`.
    Coalesce,
}

/// Literal value accepted by the expression grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExpressionLiteral {
    /// Null literal.
    Null,
    /// Boolean literal.
    Bool(bool),
    /// Signed 64-bit integer literal.
    I64(i64),
    /// IEEE-754 double-precision literal.
    F64(FiniteF64),
    /// Double-quoted string literal.
    Text(Box<str>),
}

/// Prefix unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnaryOp {
    /// Logical negation.
    Not,
    /// Numeric negation.
    Neg,
}

/// Left-associative infix operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
