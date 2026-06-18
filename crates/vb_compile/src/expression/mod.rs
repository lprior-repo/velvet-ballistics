#![forbid(unsafe_code)]
//! Cold expression lexer/parser used by the compiler AST boundary.

mod domain;
mod helpers;
mod lexer;
mod parser;

pub use domain::{BinaryOp, ExpressionHelper, ExpressionLiteral, ParsedExpression, UnaryOp};
pub use parser::parse_expression;

// Re-export for external tests that consumed the old `pub(crate)` path.
#[allow(unused_imports)]
pub(crate) use helpers::parse_helper;

// Re-export constants and error type for the test harness (accessed via `super::*`).
#[allow(unused_imports)]
pub(crate) use crate::CompileError;
#[allow(unused_imports)]
pub(crate) use lexer::{
    MAX_EXPRESSION_DEPTH, MAX_EXPRESSION_DEPTH_USIZE, MAX_EXPRESSION_SOURCE_BYTES,
    MAX_EXPRESSION_TOKENS, MAX_HELPER_ARGS,
};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
