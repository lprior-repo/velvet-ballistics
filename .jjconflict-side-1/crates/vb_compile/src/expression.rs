#![forbid(unsafe_code)]
//! Cold expression lexer/parser used by the compiler AST boundary.

use crate::CompileError;
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

const MAX_EXPRESSION_SOURCE_BYTES: usize = 4096;
const MAX_EXPRESSION_TOKENS: usize = 256;
const MAX_EXPRESSION_DEPTH: u8 = 64;
const MAX_EXPRESSION_DEPTH_USIZE: usize = 64;
const MAX_HELPER_ARGS: usize = 8;

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Integer(i64),
    Float(f64),
    String(Box<str>),
    Reference(Box<str>),
    Ident(Box<str>),
    Operator(BinaryOp),
    Unary(UnaryOp),
    LeftParen,
    RightParen,
    Comma,
    End,
}

#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    index: usize,
}

/// Parses one source expression string into the cold expression tree.
pub fn parse_expression(source: &str) -> Result<ParsedExpression, CompileError> {
    let tokens = lex(source)?;
    let mut parser = Parser::new(source, tokens);
    parser.parse_complete()
}

#[path = "expression_lexer.rs"]
mod lexer;

#[path = "expression_parser.rs"]
mod parser;

use lexer::lex;
use parser::Parser;
pub(crate) fn parse_helper(name: &str) -> Option<ExpressionHelper> {
    match name {
        "contains" => Some(ExpressionHelper::Contains),
        "starts_with" => Some(ExpressionHelper::StartsWith),
        "ends_with" => Some(ExpressionHelper::EndsWith),
        "has" => Some(ExpressionHelper::Has),
        "exists" => Some(ExpressionHelper::Exists),
        "length" => Some(ExpressionHelper::Length),
        "empty" => Some(ExpressionHelper::Empty),
        "append" => Some(ExpressionHelper::Append),
        "append_if" => Some(ExpressionHelper::AppendIf),
        "merge" => Some(ExpressionHelper::Merge),
        "sum" => Some(ExpressionHelper::Sum),
        "count" => Some(ExpressionHelper::Count),
        "unique" => Some(ExpressionHelper::Unique),
        _ => None,
    }
}

fn limit_error(source: &str, limit: &'static str, max: usize) -> CompileError {
    CompileError::ExpressionLimitExceeded {
        expression: Box::<str>::from(source),
        limit,
        max,
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

#[cfg(test)]
#[path = "expression/tests.rs"]
mod tests;
