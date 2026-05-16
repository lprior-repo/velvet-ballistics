#![forbid(unsafe_code)]
//! Bounded stack-based expression bytecode evaluator.

pub mod environment;
pub mod evaluate;

#[path = "../eval_tests.rs"]
mod tests;

pub use environment::{
    eval_helper, expect_bool, expect_i64, expect_list, expect_object, expect_symbol,
};
pub use evaluate::{
    eval_binary_op, eval_expr_program, eval_expr_program_with_store, eval_helper_with_store,
    eval_unary_op,
};
