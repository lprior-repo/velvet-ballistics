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
mod builtin_eval_tests;
#[cfg(test)]
mod harness_tests;
#[cfg(test)]
mod property_tests;

#[cfg(kani)]
pub mod proofs;

#[cfg(kani)]
pub mod kani;
#[cfg(kani)]
pub mod kani_expr_stack;

pub use bytecode::{
    ReferenceResolver, check_expr_stack_bound, compile_expr, compile_expr_to_bytecode,
    compile_expr_with_pool, compile_expr_with_resolver,
};
pub use eval::{
    eval_binary_op, eval_expr_program, eval_expr_program_with_accessors_and_store,
    eval_expr_program_with_context, eval_expr_program_with_store, eval_helper,
    eval_helper_with_store, eval_unary_op,
};

use thiserror::Error;

/// Explicit accessor availability for expression evaluation.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum AccessorContext<'a> {
    /// Accessor table is available for `ExprOp::LoadAccessor`.
    Present(&'a [vb_core::AccessorProgram]),
    /// Accessor table is intentionally unavailable at this API boundary.
    Absent(AccessorContextAbsence),
}

/// Reason an expression evaluation context has no accessor table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AccessorContextAbsence {
    /// Source-compatible legacy evaluator APIs do not accept an accessor table.
    LegacyApiNoAccessorTable,
}

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

    /// Returned when `LoadAccessor` is evaluated without an accessor table.
    #[error("missing accessor context: {absence:?}")]
    MissingAccessorContext { absence: AccessorContextAbsence },

    /// Returned when a `LoadAccessor` opcode points outside the accessor table.
    #[error("accessor index out of bounds: {accessor:?}")]
    AccessorOutOfBounds { accessor: vb_core::AccessorIdx },

    /// Returned when an accessor root slot is outside the provided slot slice.
    #[error("accessor root slot out of bounds: {root:?}")]
    AccessorRootOutOfBounds { root: vb_core::SlotIdx },

    /// Returned when an accessor root slot exists but has no runtime value.
    #[error("accessor root slot uninitialized: {root:?}")]
    AccessorRootUninitialized { root: vb_core::SlotIdx },

    /// Returned when an accessor path exceeds the protocol depth bound.
    #[error("accessor path too deep: {depth}, max {max}")]
    AccessorPathTooDeep { depth: usize, max: usize },

    /// Returned when an accessor path segment is applied to an incompatible value.
    #[error("unsupported accessor traversal: {segment} on {found}")]
    UnsupportedAccessorTraversal {
        segment: &'static str,
        found: &'static str,
    },

    /// Returned when an object accessor field is not present.
    #[error("object field not found: {field:?}")]
    ObjectFieldNotFound { field: vb_core::ids::SymbolId },

    /// Returned when a list accessor index is outside the list value.
    #[error("list index out of bounds: {index}")]
    ListIndexOutOfBounds { index: u32 },

    /// Returned when an accessor object handle does not resolve in the value store.
    #[error("object handle out of bounds: {object:?}")]
    ObjectHandleOutOfBounds { object: vb_core::ids::ObjectId },

    /// Returned when an accessor list handle does not resolve in the value store.
    #[error("list handle out of bounds: {list:?}")]
    ListHandleOutOfBounds { list: vb_core::ids::ListId },
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
            vb_core::CoreError::UnsupportedAccessorTraversal { segment, found } => {
                ExprError::UnsupportedAccessorTraversal { segment, found }
            }
            vb_core::CoreError::ObjectFieldNotFound { field } => {
                ExprError::ObjectFieldNotFound { field }
            }
            vb_core::CoreError::ListIndexOutOfBounds { index } => {
                ExprError::ListIndexOutOfBounds { index }
            }
            vb_core::CoreError::ListOutOfBounds { list } => {
                ExprError::ListHandleOutOfBounds { list }
            }
            vb_core::CoreError::ObjectOutOfBounds { object } => {
                ExprError::ObjectHandleOutOfBounds { object }
            }
            _ => ExprError::UnexpectedEof,
        }
    }
}

pub type ExprResult<T> = Result<T, ExprError>;
