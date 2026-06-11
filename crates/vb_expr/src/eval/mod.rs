#![forbid(unsafe_code)]
//! Public expression evaluator module.
//!
//! Production evaluation is implemented in `evaluate.rs` only. The historical
//! Kani dispatcher in `core.rs` stays outside production builds behind
//! `cfg(kani)`.

mod environment;
mod evaluate;

pub use crate::lexer::{BinaryOp, UnaryOp};
pub use crate::parser::ExprHelper;
pub use crate::{ExprError, ExprResult};
pub use environment::eval_helper;
pub use evaluate::{
    eval_binary_op, eval_expr_program, eval_expr_program_with_accessors_and_store,
    eval_expr_program_with_store, eval_helper_with_store, eval_unary_op,
};
pub use vb_core::limits::MAX_EXPRESSION_STACK;

#[cfg(kani)]
pub mod core;

#[cfg(test)]
#[path = "../eval_tests.rs"]
mod legacy_tests;
