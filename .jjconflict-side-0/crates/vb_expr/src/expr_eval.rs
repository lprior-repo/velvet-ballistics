#![forbid(unsafe_code)]
//! Bounded stack-based expression bytecode evaluator.

mod environment;
mod evaluate;
mod helper_store;
mod helper_store_values;
mod ops;

pub use crate::lexer::{BinaryOp, UnaryOp};
pub use crate::parser::ExprHelper;
pub use crate::{ExprError, ExprResult};
pub use environment::eval_helper;
pub use evaluate::{eval_expr_program, eval_expr_program_with_store};
pub use helper_store::eval_helper_with_store;
pub use ops::{eval_binary_op, eval_unary_op};
pub use vb_core::limits::MAX_EXPRESSION_STACK;

#[cfg(test)]
#[path = "expr_eval_tests.rs"]
mod tests;
