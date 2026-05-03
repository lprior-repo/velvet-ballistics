//! Expression evaluation engine.

mod accessors;
mod core;
mod ops;
mod ops_text_list;
mod stack;

pub use accessors::{eval_accessor, eval_accessor_with_store};
pub use core::{eval_expr, eval_expr_with_store};

#[cfg(test)]
mod tests;
