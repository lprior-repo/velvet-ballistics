#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::unused_self)]

//! Cold-path expression lexer, parser, and bytecode compiler for velvet-ballistics.
//!
//! Expressions are parsed into an AST, type-checked, and compiled to a bounded
//! stack-based bytecode (`ExprProgram`) for deterministic hot-path evaluation.

pub mod bytecode;
pub mod eval;
pub mod lexer;
pub mod parser;
pub mod typecheck;

#[cfg(test)]
mod property_tests;
#[cfg(test)]
mod harness_tests;

#[cfg(kani)]
pub mod proofs;

#[cfg(kani)]
pub mod kani_expr_stack;
#[cfg(kani)]
pub mod kani;

pub use bytecode::{
    ReferenceResolver, check_expr_stack_bound, compile_expr, compile_expr_to_bytecode,
    compile_expr_with_pool, compile_expr_with_resolver,
};
pub use eval::{
    eval_binary_op, eval_expr_program, eval_expr_program_with_store, eval_helper,
    eval_helper_with_store, eval_unary_op,
};

use thiserror::Error;

/// Expression error type.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExprError {
    /// Returned when an unexpected token is encountered during parsing.
    #[error("unexpected token: {token}")]
    UnexpectedToken { token: String },

    /// Returned when the expression ends unexpectedly (e.g., unclosed parenthesis).
    #[error("unexpected end of expression")]
    UnexpectedEof,

    /// Returned when an operator is not recognized in the expression grammar.
    #[error("unknown operator: {op}")]
    UnknownOperator { op: String },

    /// Returned when a helper function name is not registered.
    #[error("unknown helper: {helper}")]
    UnknownHelper { helper: String },

    /// Returned when the evaluation stack exceeds its maximum depth.
    #[error("stack overflow: max {max}")]
    StackOverflow { max: u8 },

    /// Returned when a binary operator is applied with insufficient stack operands.
    #[error("stack underflow")]
    StackUnderflow,

    /// Returned when a value does not match the expected type for the operation.
    #[error("type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },

    /// Returned when a division or modulo operation has a zero divisor.
    #[error("division by zero")]
    DivisionByZero,

    /// Returned when an arithmetic operation produces a value exceeding the representable range.
    #[error("integer overflow")]
    IntegerOverflow,

    /// Returned when a reference string is malformed or points to an undefined variable.
    #[error("invalid reference: {reference}")]
    InvalidReference { reference: String },

    /// Returned when the expression exceeds the maximum token count.
    #[error("expression too long: {len} tokens, max {max}")]
    ExpressionTooLong { len: usize, max: usize },

    /// Returned when a string literal is not closed before the expression ends.
    #[error("unterminated string")]
    UnterminatedString,

    /// Returned when a parsed integer cannot fit in the target integer type.
    #[error("integer out of range")]
    IntegerOutOfRange,

    /// Returned when a float value is NaN or infinite (Inf/-Inf).
    #[error("non-finite float")]
    NonFiniteFloat,

    /// Returned when a character does not match any valid token start.
    #[error("unexpected character: {ch}")]
    UnexpectedChar { ch: char },

    /// Returned when the parser's recursion depth exceeds the maximum allowed.
    #[error("parse depth exceeded: max {max}")]
    ParseDepthExceeded { max: usize },

    /// Returned when a helper call provides more arguments than the helper accepts.
    #[error("too many helper arguments: {len}, max {max}")]
    TooManyHelperArgs { len: usize, max: usize },

    /// Returned when a helper function is called with the wrong number of arguments.
    #[error("helper arity mismatch: {helper} expects {expected}, got {actual}")]
    HelperArityMismatch {
        helper: String,
        expected: usize,
        actual: usize,
    },

    #[error("bytecode too long: {len} ops, max {max}")]
    BytecodeTooLong { len: usize, max: usize },

    #[error("constant pool overflow")]
    ConstantPoolOverflow,

    #[error("unsupported literal: {literal}")]
    UnsupportedLiteral { literal: String },
}

impl From<vb_core::CoreError> for ExprError {
    fn from(e: vb_core::CoreError) -> Self {
        match e {
            vb_core::CoreError::NonFiniteNumber => ExprError::NonFiniteFloat,
            vb_core::CoreError::DivisionByZero => ExprError::DivisionByZero,
            vb_core::CoreError::ExpressionStackUnderflow => ExprError::StackUnderflow,
            vb_core::CoreError::ExpressionStackOverflow { max } => ExprError::StackOverflow { max },
            vb_core::CoreError::TypeMismatch { expected, found } => ExprError::TypeMismatch {
                expected: expected.to_string(),
                found: found.to_string(),
            },
            _ => ExprError::UnexpectedEof,
        }
    }
}

pub type ExprResult<T> = Result<T, ExprError>;
