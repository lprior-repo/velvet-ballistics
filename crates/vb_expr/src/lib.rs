#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]

//! Cold-path expression lexer, parser, and bytecode compiler for velvet-ballastics.
//!
//! Expressions are parsed into an AST, type-checked, and compiled to a bounded
//! stack-based bytecode (`ExprProgram`) for deterministic hot-path evaluation.

pub mod bytecode;
pub mod eval;
pub mod lexer;
pub mod parser;
pub mod typecheck;

pub use bytecode::{
    ReferenceResolver, check_expr_stack_bound, compile_expr, compile_expr_to_bytecode,
    compile_expr_with_pool, compile_expr_with_resolver,
};
pub use eval::{eval_binary_op, eval_expr_program, eval_helper, eval_unary_op};

use thiserror::Error;

/// Expression error type.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ExprError {
    #[error("unexpected token: {token}")]
    UnexpectedToken { token: String },

    #[error("unexpected end of expression")]
    UnexpectedEof,

    #[error("unknown operator: {op}")]
    UnknownOperator { op: String },

    #[error("unknown helper: {helper}")]
    UnknownHelper { helper: String },

    #[error("stack overflow: max {max}")]
    StackOverflow { max: u8 },

    #[error("stack underflow")]
    StackUnderflow,

    #[error("type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },

    #[error("division by zero")]
    DivisionByZero,

    #[error("integer overflow")]
    IntegerOverflow,

    #[error("invalid reference: {reference}")]
    InvalidReference { reference: String },

    #[error("expression too long: {len} tokens, max {max}")]
    ExpressionTooLong { len: usize, max: usize },

    #[error("unterminated string")]
    UnterminatedString,

    #[error("integer out of range")]
    IntegerOutOfRange,

    #[error("unexpected character: {ch}")]
    UnexpectedChar { ch: char },

    #[error("parse depth exceeded: max {max}")]
    ParseDepthExceeded { max: usize },

    #[error("too many helper arguments: {len}, max {max}")]
    TooManyHelperArgs { len: usize, max: usize },

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

pub type ExprResult<T> = Result<T, ExprError>;
