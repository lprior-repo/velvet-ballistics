#![forbid(unsafe_code)]
//! Expression lexer, parser, type checker, bytecode compiler, and evaluator.
//!
//! Owns the full expression evaluation pipeline for Velvet Ballistics.

pub mod expr_bytecode;
pub mod expr_eval;
pub mod expr_lexer;
pub mod expr_parser;
pub mod expr_proofs;
pub mod expr_stack_ops;
pub mod expr_typecheck;

#[cfg(test)]
mod expr_property_tests;

#[cfg(test)]
mod expr_builtin_eval;
#[cfg(test)]
mod expr_eval_tests;

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

#[cfg(test)]
mod core_error_conversion_tests {
    use super::*;

    #[test]
    fn core_error_division_by_zero_maps_to_expr_division_by_zero() {
        let core_err = vb_core::CoreError::DivisionByZero;
        let expr_err: ExprError = core_err.into();
        assert!(matches!(expr_err, ExprError::DivisionByZero));
    }

    #[test]
    fn core_error_expression_stack_underflow_maps_to_expr_stack_underflow() {
        let core_err = vb_core::CoreError::ExpressionStackUnderflow;
        let expr_err: ExprError = core_err.into();
        assert!(matches!(expr_err, ExprError::StackUnderflow));
    }

    #[test]
    fn core_error_expression_stack_overflow_maps_to_expr_stack_overflow() {
        let core_err = vb_core::CoreError::ExpressionStackOverflow { max: 255 };
        let expr_err: ExprError = core_err.into();
        assert_eq!(expr_err, ExprError::StackOverflow { max: 255 });
    }

    #[test]
    fn core_error_type_mismatch_maps_to_expr_type_mismatch() {
        let core_err = vb_core::CoreError::TypeMismatch {
            expected: "i64",
            found: "string",
        };
        let expr_err: ExprError = core_err.into();
        assert_eq!(
            expr_err,
            ExprError::TypeMismatch {
                expected: "i64".to_string(),
                found: "string".to_string(),
            }
        );
    }

    #[test]
    fn core_error_non_finite_number_maps_to_expr_non_finite_float() {
        let core_err = vb_core::CoreError::NonFiniteNumber;
        let expr_err: ExprError = core_err.into();
        assert!(matches!(expr_err, ExprError::NonFiniteFloat));
    }

    #[test]
    fn core_error_fallback_maps_to_expr_unexpected_eof() {
        // Use a CoreError variant not explicitly mapped
        let core_err = vb_core::CoreError::InvalidProgramCounter {
            step: vb_core::StepIdx::new(0),
        };
        let expr_err: ExprError = core_err.into();
        assert!(matches!(expr_err, ExprError::UnexpectedEof));
    }
}
