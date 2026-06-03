#![forbid(unsafe_code)]
//! Expression evaluation engine.

mod accessors;
mod core;
#[cfg(test)]
mod core_tests;
mod ops;
mod ops_text_list;
#[cfg(test)]
mod ops_text_list_tests;
mod stack;

pub use accessors::{eval_accessor, eval_accessor_with_store};
pub use core::{eval_expr, eval_expr_with_store};

#[cfg(kani)]
mod kani_stack;

#[cfg(kani)]
mod kani_div_zero;

#[cfg(test)]
mod tests;
