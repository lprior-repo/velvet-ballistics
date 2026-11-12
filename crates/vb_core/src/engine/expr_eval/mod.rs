#![forbid(unsafe_code)]
//! Expression evaluation engine.

mod accessors;
mod core;
mod ops;
mod ops_text_list;
mod stack;
pub(crate) mod taints;

pub use accessors::{eval_accessor, eval_accessor_with_store};
pub use core::{eval_expr, eval_expr_with_store};

#[cfg(kani)]
#[path = "../../verification/kani/expr_eval_kani_stack.rs"]
mod kani_stack;

#[cfg(kani)]
#[path = "../../verification/kani/expr_eval_kani_div_zero.rs"]
mod kani_div_zero;

#[cfg(kani)]
#[path = "../../verification/kani/expr_eval_kani_stack_bounds.rs"]
mod kani_stack_bounds;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod taint_tests;
