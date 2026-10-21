#![forbid(unsafe_code)]
//! Expression lexer, parser, type checker, bytecode compiler, and evaluator.
//!
//! Owns the full expression evaluation pipeline for Velvet Ballistics.

pub mod expr_bytecode;
pub mod expr_eval;
pub mod expr_lexer;
pub mod expr_parser;
pub mod expr_typecheck;
pub mod expr_stack_ops;

#[cfg(test)]
mod expr_property_tests;

#[cfg(test)]
mod expr_builtin_eval;
#[cfg(test)]
mod expr_eval_tests;
#[cfg(test)]
mod expr_eval_tests_core;
#[cfg(test)]
mod expr_eval_tests_logic;
#[cfg(test)]
mod expr_eval_tests_store;

// Re-exports for backward compatibility (vb_expr public API)
pub use expr_bytecode as bytecode;
pub use expr_eval as eval;
pub use expr_lexer as lexer;
pub use expr_parser as parser;
pub use expr_stack_ops as stack_ops;
pub use expr_typecheck as typecheck;

// Expression error type
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
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
    #[error("non-finite float")]
    NonFiniteFloat,
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

// Re-exports from expr modules (vb_expr public API)
pub use bytecode::{
    ReferenceResolver, check_expr_stack_bound, compile_expr, compile_expr_to_bytecode,
    compile_expr_with_pool, compile_expr_with_resolver,
};
pub use eval::{
    eval_binary_op, eval_expr_program, eval_expr_program_with_store, eval_helper,
    eval_helper_with_store, eval_unary_op,
};

#[cfg(test)]
pub fn new_env() -> eval::Environment {
    eval::Environment::new(64)
}

impl From<vb_core::CoreError> for ExprError {
    fn from(err: vb_core::CoreError) -> Self {
        match err {
            vb_core::CoreError::NonFiniteNumber => ExprError::NonFiniteFloat,
            _ => ExprError::UnexpectedToken { token: err.to_string() },
        }
    }
}
