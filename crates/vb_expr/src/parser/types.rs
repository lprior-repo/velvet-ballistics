//! Expression AST types for vb_expr.

/// Parsed expression AST node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprAst {
    /// Null, boolean, integer, or string literal.
    Literal(ExprLiteral),
    /// Source reference beginning with `$`.
    Reference(Box<str>),
    /// Prefix unary expression.
    Unary {
        /// Unary operator.
        op: crate::lexer::UnaryOp,
        /// Operand expression.
        expr: Box<ExprAst>,
    },
    /// Infix binary expression.
    Binary {
        /// Binary operator.
        op: crate::lexer::BinaryOp,
        /// Left operand.
        left: Box<ExprAst>,
        /// Right operand.
        right: Box<ExprAst>,
    },
    /// Built-in helper call.
    Helper {
        /// Helper name.
        name: ExprHelper,
        /// Parsed helper arguments.
        args: Box<[ExprAst]>,
    },
}

/// Literal value in the expression AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprLiteral {
    /// Null literal.
    Null,
    /// Boolean literal.
    Bool(bool),
    /// Signed 64-bit integer literal.
    I64(i64),
    /// Double-quoted string literal.
    Text(Box<str>),
}

/// Built-in helper function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprHelper {
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
}
