#![cfg(kani)]
#![forbid(unsafe_code)]
//! Kani-only evaluator compatibility shim.
//!
//! Production evaluation lives in `evaluate.rs`. This module exists only so
//! Kani harnesses can keep a stable `crate::eval::core::*` import path without
//! carrying a second evaluator implementation.

pub use super::environment::eval_helper;
pub use super::evaluate::{
    eval_binary_op, eval_expr_program, eval_expr_program_with_accessors_and_store,
    eval_expr_program_with_store, eval_helper_with_store, eval_unary_op,
};
